//! Routes for managing Dublin Metadata Subjects to accessions.
//! These act somewhat like 'tags'; they constitute a limited keyword vocabulary of descriptors
//! for accessions.
//!
//! This module provides HTTP endpoints for creating, and listing subjects.
//! It uses in-memory repositories for testing to avoid I/O operations.

use crate::app_factory::AppState;
use crate::models::request::{AccessionPagination, CreateSubjectRequest, SubjectPagination};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use validator::Validate;

pub fn get_subjects_routes() -> Router<AppState> {
    Router::new().nest(
        "/metadata-subjects",
        Router::new()
            .route("/", get(list_subjects))
            .route("/", post(create_subject)),
    )
}
async fn create_subject(
    State(state): State<AppState>,
    Json(payload): Json<CreateSubjectRequest>,
) -> Response {
    if let Err(err) = payload.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }
    state.subjects_service.create_one(payload).await
}

async fn list_subjects(
    State(state): State<AppState>,
    pagination: Query<SubjectPagination>,
) -> Response {
    if let Err(err) = pagination.0.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }
    state
        .subjects_service
        .list(
            pagination.0.page,
            pagination.0.per_page,
            pagination.0.lang,
            pagination.0.query_term,
        )
        .await
}
