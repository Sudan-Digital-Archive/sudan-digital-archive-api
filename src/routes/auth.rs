//! Authentication routes.
//!
//! This module provides HTTP endpoints for user authentication, including login and authorization.
//! It also includes a protected route for testing JWT authentication.
//! The module uses an authentication service to handle the authentication logic.

use crate::app_factory::AppState;
use crate::models::auth::AuthenticatedUser;
use crate::models::request::{AuthorizeRequest, LoginRequest};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use tracing::error;
use validator::Validate;

pub fn get_auth_routes() -> Router<AppState> {
    Router::new().nest(
        "/auth",
        Router::new()
            .route("/", post(login))
            .route("/authorize", post(authorize))
            .route("/", get(verify)),
    )
}

#[utoipa::path(
    post,
    path = "/api/v1/auth",
    tag = "Auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "OK"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn login(State(state): State<AppState>, Json(payload): Json<LoginRequest>) -> Response {
    if let Err(err) = payload.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }

    let login_result = state.auth_service.clone().login(payload).await;

    match login_result {
        Ok(response) => response,
        Err(err) => {
            let message = format!("Server error occurred: {err}");
            error!(message);
            (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/authorize",
    tag = "Auth",
    request_body = AuthorizeRequest,
    responses(
        (status = 200, description = "OK"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn authorize(
    State(state): State<AppState>,
    Json(payload): Json<AuthorizeRequest>,
) -> Response {
    let auth_result = state.auth_service.authorize(payload).await;

    match auth_result {
        Ok(response) => response,
        Err(err) => {
            let message = format!("Server error occurred: {err}");
            error!(message);
            (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/auth",
    tag = "Auth",
    responses(
        (status = 200, description = "OK", body = String),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("jwt_cookie_auth" = [])
    )
)]
async fn verify(State(_state): State<AppState>, authenticated_user: AuthenticatedUser) -> Response {
    let user_data = format!("Verifying your JWT...\nYour data:\n{authenticated_user}");
    (StatusCode::OK, user_data).into_response()
}
