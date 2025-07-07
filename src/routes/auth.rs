//! Authentication routes.
//!
//! This module provides HTTP endpoints for user authentication, including login and authorization.
//! It also includes a protected route for testing JWT authentication.
//! The module uses an authentication service to handle the authentication logic.

use crate::app_factory::AppState;
use crate::models::auth::JWTClaims;
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

async fn verify(State(_state): State<AppState>, claims: JWTClaims) -> Response {
    let user_data = format!("Verifying your JWT...\nYour data:\n{claims}");
    (StatusCode::OK, user_data).into_response()
}
