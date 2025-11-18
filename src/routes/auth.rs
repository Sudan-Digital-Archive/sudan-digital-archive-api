//! Authentication routes.
//!
//! This module provides HTTP endpoints for user authentication, including login and authorization.
//! It also includes a protected route for testing JWT authentication.
//! The module uses an authentication service to handle the authentication logic.

use crate::app_factory::AppState;
use crate::models::auth::AuthenticatedUser;
use crate::models::request::{AuthorizeRequest, LoginRequest};
use crate::models::response::CreateApiKeyResponse;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use ::entity::sea_orm_active_enums::Role;
use tracing::{error, info};
use uuid::Uuid;
use validator::Validate;

pub fn get_auth_routes() -> Router<AppState> {
    Router::new().nest(
        "/auth",
        Router::new()
            .route("/", post(login))
            .route("/authorize", post(authorize))
            .route("/", get(verify))
            .route("/:user_id/api-key", post(create_api_key)),
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

#[utoipa::path(
    post,
    path = "/api/v1/auth/{user_id}/api-key",
    tag = "Auth",
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 201, description = "API key created", body = CreateApiKeyResponse),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt_cookie_auth" = [])
    )
)]
async fn create_api_key(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    authenticated_user: AuthenticatedUser,
) -> Response {
    if authenticated_user.role != Role::Admin {
        return (StatusCode::FORBIDDEN, "Only admins can create API keys").into_response();
    }

    let api_key_result = state.auth_service.create_api_key(user_id).await;

    match api_key_result {
        Ok(api_key_secret) => {
            info!(
                "API key created by admin {} for user {}",
                authenticated_user.user_id, user_id
            );
            let response = CreateApiKeyResponse { api_key_secret };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => {
            error!(
                "Failed to create API key by admin {} for user {}: {}",
                authenticated_user.user_id, user_id, err
            );
            let message = format!("Failed to create API key: {err}");
            (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
    }
}
