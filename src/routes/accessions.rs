//! Routes for managing archival records (accessions) in the digital archive.
//!
//! This module provides HTTP endpoints for creating, retrieving, and listing accessions.
//! It uses in-memory repositories for testing to avoid I/O operations.

use crate::app_factory::AppState;
use crate::models::request::{AccessionPagination, CreateAccessionRequest};
use axum::extract::{Path, State};
use axum_extra::extract::Query;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use validator::Validate;

/// Creates routes for accession-related endpoints under `/accessions`.
pub fn get_accessions_routes() -> Router<AppState> {
    Router::new().nest(
        "/accessions",
        Router::new()
            .route("/", get(list_accessions))
            .route("/", post(create_accession))
            .route("/{accession_id}", get(get_one_accession)),
    )
}

/// Creates a new accession and initiates a web crawl task.
///
/// Returns a 201 CREATED status on success, or 400 BAD REQUEST if validation fails.
async fn create_accession(
    State(state): State<AppState>,
    Json(payload): Json<CreateAccessionRequest>,
) -> Response {
    if let Err(err) = payload.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }
    let cloned_payload = payload.clone();
    let subjects_exist = state
        .subjects_service
        .clone()
        .verify_subjects_exist(
            cloned_payload.metadata_subjects,
            cloned_payload.metadata_language,
        )
        .await;
    match subjects_exist {
        Err(err) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
        }
        Ok(flag) => {
            if !flag {
                return (StatusCode::BAD_REQUEST, "Subjects do not exist").into_response();
            }
        }
    };
    let cloned_state = state.clone();
    tokio::spawn(async move {
        cloned_state.accessions_service.create_one(payload).await;
    });
    (StatusCode::CREATED, "Started browsertrix crawl task!").into_response()
}

/// Retrieves a single accession by its ID.
///
/// Returns the accession details if found, or appropriate error response if not found.
async fn get_one_accession(State(state): State<AppState>, Path(id): Path<i32>) -> Response {
    state.accessions_service.get_one(id).await
}

/// Lists accessions with pagination and filtering support.
///
/// Supports filtering by language, date range, and search terms.
/// Returns 400 BAD REQUEST if pagination parameters are invalid.
async fn list_accessions(
    State(state): State<AppState>,
    pagination: Query<AccessionPagination>,
) -> Response {
    if let Err(err) = pagination.0.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }
    state
        .accessions_service
        .list(
            pagination.0.page,
            pagination.0.per_page,
            pagination.0.lang,
            pagination.0.metadata_subjects,
            pagination.0.query_term,
            pagination.0.date_from,
            pagination.0.date_to,
        )
        .await
}

#[cfg(test)]
mod tests {
    use crate::models::common::MetadataLanguage;
    use crate::models::request::CreateAccessionRequest;
    use crate::models::response::GetOneAccessionResponse;
    use crate::test_tools::{
        build_test_accessions_service, build_test_app, mock_one_accession_with_metadata,
        mock_paginated_ar, mock_paginated_en,
    };
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use entity::accessions_with_metadata::Model as AccessionsWithMetadataModel;
    use http_body_util::BodyExt;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn run_one_crawl() {
        let accessions_service = build_test_accessions_service();
        accessions_service
            .create_one(CreateAccessionRequest {
                url: "".to_string(),
                metadata_language: MetadataLanguage::English,
                metadata_title: "".to_string(),
                metadata_description: Some("".to_string()),
                metadata_time: Default::default(),
                browser_profile: None,
                metadata_subjects: vec![1, 2, 3],
            })
            .await;
    }

    #[tokio::test]
    async fn run_one_crawl_without_description() {
        let accessions_service = build_test_accessions_service();
        accessions_service
            .create_one(CreateAccessionRequest {
                url: "".to_string(),
                metadata_language: MetadataLanguage::English,
                metadata_title: "".to_string(),
                metadata_subjects: vec![1, 2, 3],
                metadata_description: None,
                metadata_time: Default::default(),
                browser_profile: None,
            })
            .await;
    }
    #[tokio::test]
    async fn create_one_accession() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/accessions")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_vec(&json!({
    "url": "https://www.theguardian.com/business/2025/jan/10/britain-energy-costs-labour-power-plants-uk-cold-weather?utm_source=firefox-newtab-en-gb",
    "metadata_language": "english",
    "metadata_title": "Guardian piece",
    "metadata_subject": "UK energy costs",
    "metadata_description": "Blah de blah",
    "metadata_time": "2024-11-01T23:32:00",
    "browser_profile": null,
    "metadata_subjects": [1]
})).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual = String::from_utf8((&body).to_vec()).unwrap();
        let expected = "Started browsertrix crawl task!".to_string();
        assert_eq!(actual, expected)
    }

    #[tokio::test]
    async fn create_one_accession_no_description() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/accessions")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "url": "https://facebook.com/some/story",
                            "metadata_language": "english",
                            "metadata_title": "Guardian piece",
                            "browser_profile": "facebook",
                            "metadata_description": null,
                            "metadata_time": "2024-11-01T23:32:00",
                            "browser_profile": "facebook",
                            "metadata_subjects": [1]
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual = String::from_utf8((&body).to_vec()).unwrap();
        let expected = "Started browsertrix crawl task!".to_string();
        assert_eq!(actual, expected)
    }
    #[tokio::test]
    async fn get_one_accession() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/accessions/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual: GetOneAccessionResponse = serde_json::from_slice(&body).unwrap();
        let mocked_resp = mock_one_accession_with_metadata();
        let expected = GetOneAccessionResponse {
            accession: mocked_resp,
            wacz_url: "my url".to_owned(),
        };
        assert_eq!(actual, expected)
    }

    #[tokio::test]
    async fn list_accessions_en() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/accessions?page=0&per_page=1&lang=english")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual: ([AccessionsWithMetadataModel; 1], u64) =
            serde_json::from_slice(&body).unwrap();
        let mocked_resp = mock_paginated_en();
        let expected = mocked_resp;
        assert_eq!(actual.1, expected.1);
        assert_eq!(actual.0.len(), expected.0.len());
    }

    #[tokio::test]
    async fn list_accessions_ar() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/accessions?page=0&per_page=1&lang=arabic")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual: ([AccessionsWithMetadataModel; 1], u64) =
            serde_json::from_slice(&body).unwrap();
        let mocked_resp = mock_paginated_ar();
        let expected = mocked_resp;
        assert_eq!(actual.1, expected.1);
        assert_eq!(actual.0.len(), expected.0.len());
    }
}
