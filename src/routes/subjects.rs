//! Routes for managing Dublin Metadata Subjects to accessions.
//! These act somewhat like 'tags'; they constitute a limited keyword vocabulary of descriptors
//! for accessions.
//!
//! This module provides HTTP endpoints for creating, and listing subjects.
//! It uses in-memory repositories for testing to avoid I/O operations.

use crate::app_factory::AppState;
use crate::models::request::{CreateSubjectRequest, SubjectPagination, DeleteSubjectRequest};
use axum::extract::{Query, State, Path};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, delete};
use axum::{Json, Router};
use validator::Validate;
use crate::models::auth::JWTClaims;
use ::entity::sea_orm_active_enums::Role;

/// Creates routes for subject-related endpoints under `/metadata-subjects`.
pub fn get_subjects_routes() -> Router<AppState> {
    Router::new().nest(
        "/metadata-subjects",
        Router::new()
            .route("/", get(list_subjects))
            .route("/", post(create_subject))
            .route("/{subject_id}", delete(delete_subject)),
    )
}

/// Creates a new metadata subject.
///
/// Returns a 201 CREATED status on success, or 400 BAD REQUEST if validation fails.
async fn create_subject(
    State(state): State<AppState>,
    // TODO: Later should add a role like researcher and validate user has
    // researcher or admin role
    _claims: JWTClaims,
    Json(payload): Json<CreateSubjectRequest>,
) -> Response {
    if let Err(err) = payload.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }
    state.subjects_service.create_one(payload).await
}

/// Lists metadata subjects with pagination and filtering support.
///
/// Supports filtering by language and search query.
/// Returns 400 BAD REQUEST if pagination parameters are invalid.
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

async fn delete_subject(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    claims: JWTClaims,
    Json(payload): Json<DeleteSubjectRequest>,

) -> Response {
    if claims.role != Role::Admin {
        return (StatusCode::FORBIDDEN, "Insufficient permissions").into_response();
    }
    if let Err(err) = payload.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }
    state.subjects_service.delete_one(id, payload.lang).await
}
#[cfg(test)]
mod tests {

    use crate::models::response::{
        ListSubjectsArResponse, ListSubjectsEnResponse, SubjectResponse,
    };
    use crate::test_tools::{
        build_test_app, mock_paginated_subjects_ar, mock_paginated_subjects_en,get_mock_jwt
    };
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn create_one_subject_en() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/metadata-subjects")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(http::header::AUTHORIZATION, format!("Bearer {}", get_mock_jwt()))
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "lang": "english",
                            "metadata_subject": "some cool archive"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual: SubjectResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(actual.subject, "some cool archive".to_string());
    }

    #[tokio::test]
    async fn create_one_subject_ar() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/metadata-subjects")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(http::header::AUTHORIZATION, format!("Bearer {}", get_mock_jwt()))
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "lang": "arabic",
                            "metadata_subject": "some cool archive"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual: SubjectResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(actual.subject, "some cool archive".to_string());
    }

    #[tokio::test]
    async fn list_subjects_en() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/metadata-subjects?page=0&per_page=1&lang=english")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual: ListSubjectsEnResponse = serde_json::from_slice(&body).unwrap();
        let mocked_resp = mock_paginated_subjects_en();
        assert_eq!(actual.num_pages, mocked_resp.1);
        assert_eq!(actual.items.len(), mocked_resp.0.len());
    }

    #[tokio::test]
    async fn list_subjects_ar() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/metadata-subjects?page=0&per_page=1&lang=arabic")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual: ListSubjectsArResponse = serde_json::from_slice(&body).unwrap();
        let mocked_resp = mock_paginated_subjects_ar();
        assert_eq!(actual.num_pages, mocked_resp.1);
        assert_eq!(actual.items.len(), mocked_resp.0.len());
    }
}
