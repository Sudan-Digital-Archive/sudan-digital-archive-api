//! Application factory module for configuring and building the API server.
//!
//! This module handles the setup of:
//! - Rate limiting (via tower-governor)
//! - CORS configuration
//! - Middleware stack (compression, timeout, tracing)
//! - Route registration
//!
//! # Rate Limiting
//! The application uses tower-governor for rate limiting with default configuration:
//! - 32 requests per minute per IP address
//! - Regular cleanup of rate limiting storage every 60 seconds
//!
//! Note: Rate limiting is disabled in test mode.

use crate::open_api_spec::ApiDoc;
use crate::routes::accessions::get_accessions_routes;
use crate::routes::auth::get_auth_routes;
use crate::routes::health::healthcheck;
use crate::routes::subjects::get_subjects_routes;
use crate::services::accessions_service::AccessionsService;
use crate::services::auth_service::AuthService;
use crate::services::subjects_service::SubjectsService;
use axum::extract::MatchedPath;
use axum::http::Request;
use axum::routing::get;
use axum::Router;
use http::header::CONTENT_TYPE;
use http::{HeaderValue, Method};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_governor::{governor::GovernorConfig, GovernorLayer};
use tower_http::cors::CorsLayer;
use tower_http::{
    compression::CompressionLayer, timeout::TimeoutLayer, trace::TraceLayer,
    validate_request::ValidateRequestHeaderLayer,
};
use tracing::info_span;
use tracing_subscriber::util::SubscriberInitExt;
use utoipa::OpenApi;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;
/// Application state shared across routes
#[derive(Clone)]
pub struct AppState {
    pub accessions_service: AccessionsService,
    pub auth_service: AuthService,
    pub subjects_service: SubjectsService,
}

/// Creates and configures the main application router with middleware and routes.
///
/// # Arguments
/// * `app_state` - Shared application state containing service instances
/// * `cors_origins` - List of allowed CORS origins
/// * `test` - Boolean flag to disable rate limiting and modify logging for tests
///
/// # Returns
/// Configured Router instance with all routes, middleware, and rate limiting (if not in test mode)
pub fn create_app(app_state: AppState, cors_origins: Vec<HeaderValue>, test: bool) -> Router {
    let subscriber = tracing_subscriber::fmt().with_target(false).pretty();
    // turn on if you want more verbose logs
    // .with_max_level(tracing::Level::DEBUG)

    // this is a pain but it's because the tests are run in different threads
    // when you do cargo test; see
    // https://github.com/tokio-rs/console/issues/505
    if test {
        subscriber.set_default();
    } else {
        subscriber.init();
    }
    let governor_conf = Arc::new(GovernorConfig::default());
    let governor_limiter = governor_conf.limiter().clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(60));
        tracing::info!("rate limiting storage size: {}", governor_limiter.len());
        governor_limiter.retain_recent();
    });
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PUT])
        .allow_origin(cors_origins)
        .allow_headers([CONTENT_TYPE])
        .allow_credentials(true);
    let all_routes: Router<AppState> = build_routes(ApiDoc::openapi());
    let base_routes = all_routes.layer(cors);
    // rate limiting breaks tests *sigh* #security #pita
    if test {
        base_routes.with_state(app_state)
    } else {
        base_routes
            .layer(GovernorLayer {
                config: governor_conf,
            })
            .with_state(app_state)
    }
}

/// Builds the application routes with middleware stack.
///
/// Configures:
/// - Request tracing with method and path logging
/// - 120 second timeout
/// - Response compression
/// - JSON content type validation
/// - Health check endpoint
/// - API routes
fn build_routes(api: utoipa::openapi::OpenApi) -> Router<AppState> {
    let middleware = ServiceBuilder::new()
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                let matched_path = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map(MatchedPath::as_str);
                // add fields to different logs here
                info_span!(
                    "http_request",
                    method = ?request.method(),
                    request_path = matched_path,
                )
            }),
        )
        .layer(TimeoutLayer::new(Duration::from_secs(120)))
        .layer(CompressionLayer::new())
        .layer(ValidateRequestHeaderLayer::accept("application/json"));
    let accessions_routes = get_accessions_routes();
    let subjects_routes = get_subjects_routes();
    let auth_routes = get_auth_routes();
    Router::new()
        .merge(SwaggerUi::new("/docs").url("/docs/openapi.json", api.clone()))
        .merge(Redoc::with_url_and_config(
            "/redoc",
            api,
            || json!({ "hideLogo": true }),
        ))
        .nest("/api/v1", accessions_routes)
        .nest("/api/v1", subjects_routes)
        .nest("/api/v1", auth_routes)
        .nest("/health", Router::new().route("/", get(healthcheck)))
        .layer(middleware)
}
