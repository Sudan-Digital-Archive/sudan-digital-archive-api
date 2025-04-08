//! Routes for managing Dublin Metadata Subjects to accessions.
//! These act somewhat like 'tags'; they constitute a limited keyword vocabulary of descriptors
//! for accessions.
//!
//! This module provides HTTP endpoints for creating, and listing subjects.
//! It uses in-memory repositories for testing to avoid I/O operations.

use crate::app_factory::AppState;
use crate::models::request::LoginRequest;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use tracing::{error, info, warn};
use validator::Validate;

pub fn get_auth_routes() -> Router<AppState> {
    Router::new().nest("/auth", Router::new().route("/", post(login)))
}

async fn login(State(state): State<AppState>, Json(payload): Json<LoginRequest>) -> Response {
    if let Err(err) = payload.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }
    let login_result = state
        .auth_service
        .clone()
        .log_user_in(payload.clone())
        .await;
    match login_result {
        Ok(token) => match token {
            Some(token) => {
                info!(
                    "Sending login email to user with email {} and deleting expired sessions",
                    payload.email
                );
                tokio::spawn(async move {
                    state.auth_service.clone().delete_expired_sessions().await;
                    state.auth_service.send_login_email(token, payload.email).await;
                });
                (StatusCode::OK, "Login email sent").into_response()
            }
            None => {
                let message = format!("User with email {} not found", payload.email);
                warn!(message);
                (StatusCode::NOT_FOUND, message).into_response()
            }
        },
        Err(err) => {
            let message = format!("Server error occurred: {}", err);
            error!(message);
            (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
    }
}
