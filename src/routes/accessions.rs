//! Routes for managing archival records (accessions) in the digital archive.
//!
//! This module provides HTTP endpoints for creating, retrieving, and listing accessions.
//! It uses in-memory repositories for testing to avoid I/O operations.

use crate::app_factory::AppState;
use crate::models::auth::JWTClaims;
use crate::models::request::{
    AccessionPagination, AccessionPaginationWithPrivate, CreateAccessionRequest,
    UpdateAccessionRequest,
};
use ::entity::sea_orm_active_enums::Role;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use axum_extra::extract::Query;
use validator::Validate;

/// Creates routes for accession-related endpoints under `/accessions`.
pub fn get_accessions_routes() -> Router<AppState> {
    Router::new().nest(
        "/accessions",
        Router::new()
            .route("/", get(list_accessions))
            .route("/private", get(list_accessions_private))
            .route("/", post(create_accession))
            .route("/{accession_id}", get(get_one_accession))
            .route("/private/{accession_id}", get(get_one_private_accession))
            .route("/{accession_id}", delete(delete_accession))
            .route("/{accession_id}", put(update_accession)),
    )
}

/// Creates a new accession and initiates a web crawl task.
///
/// Returns a 201 CREATED status on success, or 400 BAD REQUEST if validation fails.
async fn create_accession(
    State(state): State<AppState>,
    // TODO: Later should add a role like researcher and validate user has
    // researcher or admin role
    _claims: JWTClaims,
    Json(payload): Json<CreateAccessionRequest>,
) -> Response {
    if let Err(err) = payload.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }
    let subjects_exist = state
        .subjects_service
        .clone()
        .verify_subjects_exist(
            payload.metadata_subjects.clone(),
            payload.metadata_language.clone(),
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
    tokio::spawn(async move {
        state.accessions_service.create_one(payload).await;
    });
    (StatusCode::CREATED, "Started browsertrix crawl task!").into_response()
}

/// Retrieves a single accession by its ID.
///
/// Returns the accession details if found, or appropriate error response if not found.
async fn get_one_accession(State(state): State<AppState>, Path(id): Path<i32>) -> Response {
    state.accessions_service.get_one(id, false).await
}

async fn get_one_private_accession(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    // TODO: Later should add a role like researcher and validate user has
    // researcher or admin role
    _claims: JWTClaims,
) -> Response {
    state.accessions_service.get_one(id, true).await
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
    let list_params = AccessionPaginationWithPrivate {
        page: pagination.0.page,
        per_page: pagination.0.per_page,
        lang: pagination.0.lang,
        metadata_subjects: pagination.0.metadata_subjects,
        query_term: pagination.0.query_term,
        date_from: pagination.0.date_from,
        date_to: pagination.0.date_to,
        is_private: false,
    };
    state.accessions_service.list(list_params).await
}

async fn list_accessions_private(
    State(state): State<AppState>,
    pagination: Query<AccessionPaginationWithPrivate>,
    // TODO: Later should add a role like researcher and validate user has
    // researcher or admin role
    _claims: JWTClaims,
) -> Response {
    if let Err(err) = pagination.0.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }

    state.accessions_service.list(pagination.0).await
}

async fn delete_accession(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    claims: JWTClaims,
) -> Response {
    if claims.role != Role::Admin {
        return (StatusCode::FORBIDDEN, "Insufficient permissions").into_response();
    }

    state.accessions_service.delete_one(id).await
}

async fn update_accession(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    // TODO: Later should add a role like researcher and validate user has
    // researcher or admin role
    _claims: JWTClaims,
    Json(payload): Json<UpdateAccessionRequest>,
) -> Response {
    let subjects_exist = state
        .subjects_service
        .clone()
        .verify_subjects_exist(
            payload.metadata_subjects.clone(),
            payload.metadata_language.clone(),
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
    state.accessions_service.update_one(id, payload).await
}

#[cfg(test)]
mod tests {
    use crate::models::common::MetadataLanguage;
    use crate::models::request::CreateAccessionRequest;
    use crate::models::response::{GetOneAccessionResponse, ListAccessionsResponse};
    use crate::test_tools::{
        build_test_accessions_service, build_test_app, get_mock_jwt,
        mock_one_accession_with_metadata, mock_paginated_ar, mock_paginated_en,
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
                is_private: false,
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
                is_private: true,
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
                    .header(http::header::AUTHORIZATION, format!("Bearer {}", get_mock_jwt()))
                    .body(Body::from(
                        serde_json::to_vec(&json!({
    "url": "https://www.theguardian.com/business/2025/jan/10/britain-energy-costs-labour-power-plants-uk-cold-weather?utm_source=firefox-newtab-en-gb",
    "metadata_language": "english",
    "metadata_title": "Guardian piece",
    "metadata_subject": "UK energy costs",
    "metadata_description": "Blah de blah",
    "metadata_time": "2024-11-01T23:32:00",
    "browser_profile": null,
    "metadata_subjects": [1],
    "is_private": false
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
                    .header(
                        http::header::AUTHORIZATION,
                        format!("Bearer {}", get_mock_jwt()),
                    )
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "url": "https://facebook.com/some/story",
                            "metadata_language": "english",
                            "metadata_title": "Guardian piece",
                            "browser_profile": "facebook",
                            "metadata_description": null,
                            "metadata_time": "2024-11-01T23:32:00",
                            "browser_profile": "facebook",
                            "metadata_subjects": [1],
                            "is_private": true
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
    async fn get_one_private_accession_no_auth() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/accessions/private/1")
                    .header(
                        http::header::AUTHORIZATION,
                        format!("Bearer {}", get_mock_jwt()),
                    )
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
    async fn get_one_private_accession_with_auth() {
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
        let actual: ListAccessionsResponse = serde_json::from_slice(&body).unwrap();
        let mocked_resp = mock_paginated_en();
        let expected = mocked_resp;
        assert_eq!(actual.num_pages, expected.1);
        assert_eq!(actual.items.len(), expected.0.len());
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
        let actual: ListAccessionsResponse = serde_json::from_slice(&body).unwrap();
        let mocked_resp = mock_paginated_ar();
        let expected = mocked_resp;
        assert_eq!(actual.num_pages, expected.1);
        assert_eq!(actual.items.len(), expected.0.len());
    }

    #[tokio::test]
    async fn list_accessions_private_no_auth() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/accessions/private?page=0&per_page=1&lang=english&private=true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_accessions_private_with_auth_en() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/accessions?page=0&per_page=1&lang=english")
                    .header(
                        http::header::AUTHORIZATION,
                        format!("Bearer {}", get_mock_jwt()),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual: ListAccessionsResponse = serde_json::from_slice(&body).unwrap();
        let mocked_resp = mock_paginated_en();
        let expected = mocked_resp;
        assert_eq!(actual.num_pages, expected.1);
        assert_eq!(actual.items.len(), expected.0.len());
    }
    #[tokio::test]
    async fn delete_one_accession_no_auth() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri("/api/v1/accessions/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn delete_one_accession_with_auth() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri("/api/v1/accessions/1")
                    .header(
                        http::header::AUTHORIZATION,
                        format!("Bearer {}", get_mock_jwt()),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual = String::from_utf8((&body).to_vec()).unwrap();
        let expected = "Accession deleted".to_string();
        assert_eq!(actual, expected);
    }
    #[tokio::test]
    async fn update_one_accession_no_auth() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::PUT)
                    .uri("/api/v1/accessions/1")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "metadata_language": "english",
                            "metadata_title": "Guardian piece",
                            "metadata_subject": "UK energy costs",
                            "metadata_description": "Blah de blah",
                            "metadata_time": "2024-11-01T23:32:00",
                            "browser_profile": null,
                            "metadata_subjects": [1],
                            "is_private": false
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
    async fn update_one_accession_with_auth() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::PUT)
                    .uri("/api/v1/accessions/1")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(
                        http::header::AUTHORIZATION,
                        format!("Bearer {}", get_mock_jwt()),
                    )
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "metadata_language": "english",
                            "metadata_title": "Guardian piece",
                            "metadata_subject": "UK energy costs",
                            "metadata_description": "Blah de blah",
                            "metadata_time": "2024-11-01T23:32:00",
                            "metadata_subjects": [1],
                            "is_private": false
                        }))
                        .unwrap(),
                    ))
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
}
