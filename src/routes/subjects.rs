//! Routes for managing Dublin Metadata Subjects to accessions.
//! These act somewhat like 'tags'; they constitute a limited keyword vocabulary of descriptors
//! for accessions.
//!
//! This module provides HTTP endpoints for creating, and listing subjects.
//! It uses in-memory repositories for testing to avoid I/O operations.

use crate::app_factory::AppState;
use crate::auth::validate_at_least_researcher;
use crate::models::auth::AuthenticatedUser;
use crate::models::request::{CreateSubjectRequest, DeleteSubjectRequest, SubjectPagination};
use crate::models::response::{ListSubjectsArResponse, ListSubjectsEnResponse, SubjectResponse};
use ::entity::sea_orm_active_enums::Role;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use validator::Validate;

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

#[utoipa::path(
    post,
    path = "/api/v1/metadata-subjects",
    tag = "Subjects",
    request_body = CreateSubjectRequest,
    responses(
        (status = 201, description = "Created", body = SubjectResponse),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("jwt_cookie_auth" = [])
    )
)]
async fn create_subject(
    State(state): State<AppState>,
    authenticated_user: AuthenticatedUser,
    Json(payload): Json<CreateSubjectRequest>,
) -> Response {
    if !validate_at_least_researcher(&authenticated_user.role) {
        return (StatusCode::FORBIDDEN, "Must have at least researcher role").into_response();
    }
    if let Err(err) = payload.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }
    state.subjects_service.create_one(payload).await
}

#[utoipa::path(
    get,
    path = "/api/v1/metadata-subjects",
    tag = "Subjects",
    params(
        SubjectPagination
    ),
    responses(
        (status = 200, description = "OK", body = ListSubjectsEnResponse, content_type = "application/json"),
        (status = 200, description = "OK", body = ListSubjectsArResponse, content_type = "application/json"),
        (status = 400, description = "Bad request")
    )
)]
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

#[utoipa::path(
    delete,
    path = "/api/v1/metadata-subjects/{subject_id}",
    tag = "Subjects",
    params(
        ("subject_id" = i32, Path, description = "Subject ID")
    ),
    request_body = DeleteSubjectRequest,
    responses(
        (status = 200, description = "Subject deleted"),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    security(
        ("jwt_cookie_auth" = [])
    )
)]
async fn delete_subject(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    authenticated_user: AuthenticatedUser,
    Json(payload): Json<DeleteSubjectRequest>,
) -> Response {
    if authenticated_user.role != Role::Admin {
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
        build_test_app, get_mock_jwt, mock_paginated_subjects_ar, mock_paginated_subjects_en,
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
    async fn create_one_subject_no_auth() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/metadata-subjects")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
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

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    #[tokio::test]
    async fn create_one_subject_en() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/metadata-subjects")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
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
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
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

    #[tokio::test]
    async fn delete_one_subject_no_auth() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri("/api/v1/metadata-subjects/1")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
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

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    #[tokio::test]
    async fn delete_one_subject_with_auth() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri("/api/v1/metadata-subjects/1")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "lang": "english",
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
