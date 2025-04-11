//! Routes for managing Dublin Metadata Subjects to accessions.
//! These act somewhat like 'tags'; they constitute a limited keyword vocabulary of descriptors
//! for accessions.
//!
//! This module provides HTTP endpoints for creating, and listing subjects.
//! It uses in-memory repositories for testing to avoid I/O operations.

use crate::app_factory::AppState;
use crate::models::auth::JWTClaims;
use crate::models::request::{AuthorizeRequest, LoginRequest};
use axum::extract::State;
use axum::http::{
    header::{HeaderMap, HeaderValue, SET_COOKIE},
    StatusCode,
};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use tracing::{error, info, warn};
use validator::Validate;

pub fn get_auth_routes() -> Router<AppState> {
    Router::new().nest(
        "/auth",
        Router::new()
            .route("/", post(login))
            .route("/authorize", post(authorize))
            .route("/jwt-dev-test", get(protected)),
    )
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
        Ok(success) => match success {
            Some(session_and_user) => {
                let (session_id, user_id) = session_and_user;
                info!(
                    "Sending login email to user with email {} and deleting expired sessions",
                    payload.email
                );
                tokio::spawn(async move {
                    state.auth_service.clone().delete_expired_sessions().await;
                    state
                        .auth_service
                        .send_login_email(session_id, user_id, payload.email)
                        .await;
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

async fn authorize(
    State(state): State<AppState>,
    Json(payload): Json<AuthorizeRequest>,
) -> Response {
    let session_expiry_time_result = state
        .auth_service
        .clone()
        .get_session_expiry(payload.clone())
        .await;
    match session_expiry_time_result {
        Ok(good_sesh) => match good_sesh {
            Some(sesh_exists) => {
                let cookie_string_result = state
                    .auth_service
                    .build_auth_cookie_string(payload.user_id, sesh_exists);
                match cookie_string_result {
                    Ok(cookie_string) => {
                        let mut headers = HeaderMap::new();
                        let header_value_result = HeaderValue::from_str(&cookie_string);
                        match header_value_result {
                            Ok(good_header_value) => {
                                headers.insert(SET_COOKIE, good_header_value);
                                (StatusCode::OK, headers, "Authentication successful")
                                    .into_response()
                            }
                            Err(err) => {
                                let message = format!("Failed to create cookie header: {}", err);
                                error!(message);
                                (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
                            }
                        }
                    }
                    Err(err) => {
                        let message = format!("Server error occurred: {}", err);
                        error!(message);
                        (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
                    }
                }
            }
            None => {
                let message = "Session does not exist for user";
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
#[axum::debug_handler]
async fn protected(State(_state): State<AppState>, claims: JWTClaims) -> Response {
    let user_data = format!("Welcome to the protected area :)\nYour data:\n{claims}",);

    (StatusCode::OK, user_data).into_response()
}
