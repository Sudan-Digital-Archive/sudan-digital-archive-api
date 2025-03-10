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
use validator::Validate;

pub fn get_auth_routes() -> Router<AppState> {
    Router::new().nest("/auth", Router::new().route("/", post(login)))
}

async fn login(State(state): State<AppState>, Json(payload): Json<LoginRequest>) -> Response {
    if let Err(err) = payload.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }
    // TODO: Return 500 JSON response is this is invalid or 404 if not found
    state.auth_service.log_user_in(payload).await;
    todo!()
    // TODO: Create async task to send an email
    // TODO: Create async task to delete expired db rows
    // TODO: Return a 200 JSON response 
}
