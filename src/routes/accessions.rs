//! Routes for managing archival records (accessions) in the digital archive.
//!
//! This module provides HTTP endpoints for creating, retrieving, and listing accessions.
//! It uses in-memory repositories for testing to avoid I/O operations.

use crate::app_factory::AppState;
use crate::auth::{validate_at_least_contributor, validate_at_least_researcher};
use crate::models::auth::AuthenticatedUser;
use crate::models::request::{
    AccessionPagination, AccessionPaginationWithPrivate, CreateAccessionRawMultipartRequest,
    CreateAccessionRequest, UpdateAccessionRequest,
};
use crate::models::response::{GetOneAccessionResponse, ListAccessionsResponse};
use ::entity::sea_orm_active_enums::Role;
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use axum_extra::extract::Query;
use tracing::{error, info};
use validator::Validate;

/// Creates routes for accession-related endpoints under `/accessions`.
pub fn get_accessions_routes(max_file_upload_size: usize) -> Router<AppState> {
    Router::new().nest(
        "/accessions",
        Router::new()
            .route("/", get(list_accessions))
            .route("/private", get(list_accessions_private))
            .route("/crawl", post(create_accession_crawl))
            .route("/raw", post(create_accession_raw))
            // Increase limit; default is 2MB; this only applies to raw upload endpoint
            // see https://docs.rs/axum/latest/axum/extract/struct.DefaultBodyLimit.html
            .layer(DefaultBodyLimit::max(max_file_upload_size))
            .route("/{accession_id}", get(get_one_accession))
            .route("/private/{accession_id}", get(get_one_private_accession))
            .route("/{accession_id}", delete(delete_accession))
            .route("/{accession_id}", put(update_accession)),
    )
}

#[utoipa::path(
    post,
    path = "/api/v1/accessions/raw",
    tag = "Accessions",
    request_body(
        content = CreateAccessionRawMultipartRequest,
        content_type = "multipart/form-data",
        description = "Multipart upload request. \n\n**Important:** The `metadata` field MUST be the first part of the form and contain the JSON metadata. The `file` field MUST be the second part and contain the binary file content."
    ),
    responses(
        (status = 201, description = "Accession created!"),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("jwt_cookie_auth" = []),
        ("api_key_auth" = [])
    )
)]
async fn create_accession_raw(
    State(state): State<AppState>,
    authenticated_user: AuthenticatedUser,
    multipart: Multipart,
) -> Response {
    if !validate_at_least_contributor(&authenticated_user.role) {
        return (StatusCode::FORBIDDEN, "Must have at least contributor role").into_response();
    }
    info!("Received raw accession creation request via multipart/form-data");
    let create_accession_raw_request = match state
        .accessions_service
        .clone()
        .extract_accession_from_multipart_form(multipart, state.subjects_service)
        .await
    {
        Ok(data) => data,
        Err(response) => return response,
    };

    match state
        .accessions_service
        .clone()
        .write_one_raw(create_accession_raw_request)
        .await
    {
        Ok(id) => {
            info!("Raw accession created with id: {}", id);
            (
                StatusCode::CREATED,
                format!("Accession created with id: {id}"),
            )
                .into_response()
        }
        Err(err) => {
            error!("Failed to create raw accession: {:?}", err);
            err
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/accessions/crawl",
    tag = "Accessions",
    request_body = CreateAccessionRequest,
    responses(
        (status = 201, description = "Started browsertrix crawl task!"),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("jwt_cookie_auth" = []),
        ("api_key_auth" = [])
    )
)]
async fn create_accession_crawl(
    State(state): State<AppState>,
    authenticated_user: AuthenticatedUser,
    Json(payload): Json<CreateAccessionRequest>,
) -> Response {
    if !validate_at_least_contributor(&authenticated_user.role) {
        return (StatusCode::FORBIDDEN, "Must have at least contributor role").into_response();
    }
    if let Err(err) = payload.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }
    let subjects_exist = state
        .subjects_service
        .clone()
        .verify_subjects_exist(payload.metadata_subjects.clone(), payload.metadata_language)
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
        state
            .accessions_service
            .create_one(payload, authenticated_user.user_id)
            .await;
    });
    (StatusCode::CREATED, "Started browsertrix crawl task!").into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/accessions/{accession_id}",
    tag = "Accessions",
    params(
        ("accession_id" = i32, Path, description = "Accession ID")
    ),
    responses(
        (status = 200, description = "OK", body = GetOneAccessionResponse),
        (status = 404, description = "Not found")
    )
)]
async fn get_one_accession(State(state): State<AppState>, Path(id): Path<i32>) -> Response {
    state.accessions_service.get_one(id, false).await
}

#[utoipa::path(
    get,
    path = "/api/v1/accessions/private/{accession_id}",
    tag = "Accessions",
    responses(
        (status = 200, description = "OK", body = GetOneAccessionResponse),
        (status = 404, description = "Not found"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("jwt_cookie_auth" = []),
        ("api_key_auth" = [])
    )
)]
async fn get_one_private_accession(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    authenticated_user: AuthenticatedUser,
) -> Response {
    if !validate_at_least_researcher(&authenticated_user.role) {
        return (StatusCode::FORBIDDEN, "Must have at least researcher role").into_response();
    }
    state.accessions_service.get_one(id, true).await
}

#[utoipa::path(
    get,
    path = "/api/v1/accessions",
    tag = "Accessions",
    params(
        AccessionPagination
    ),
    responses(
        (status = 200, description = "OK", body = ListAccessionsResponse),
        (status = 400, description = "Bad request")
    )
)]
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
        metadata_subjects_inclusive_filter: pagination.0.metadata_subjects_inclusive_filter,
        query_term: pagination.0.query_term,
        url_filter: pagination.0.url_filter,
        date_from: pagination.0.date_from,
        date_to: pagination.0.date_to,
        is_private: false,
    };
    state.accessions_service.list(list_params).await
}

#[utoipa::path(
    get,
    path = "/api/v1/accessions/private",
    tag = "Accessions",
    params(
        AccessionPaginationWithPrivate
    ),
    responses(
        (status = 200, description = "OK", body = ListAccessionsResponse),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("jwt_cookie_auth" = []),
        ("api_key_auth" = [])
    )
)]
async fn list_accessions_private(
    State(state): State<AppState>,
    pagination: Query<AccessionPaginationWithPrivate>,
    authenticated_user: AuthenticatedUser,
) -> Response {
    if !validate_at_least_researcher(&authenticated_user.role) {
        return (StatusCode::FORBIDDEN, "Must have at least researcher role").into_response();
    }
    if let Err(err) = pagination.0.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }

    state.accessions_service.list(pagination.0).await
}

#[utoipa::path(
    delete,
    path = "/api/v1/accessions/{accession_id}",
    tag = "Accessions",
    responses(
        (status = 200, description = "Accession deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    security(
        ("jwt_cookie_auth" = []),
        ("api_key_auth" = [])
    )
)]
async fn delete_accession(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    authenticated_user: AuthenticatedUser,
) -> Response {
    if authenticated_user.role != Role::Admin {
        return (StatusCode::FORBIDDEN, "Insufficient permissions").into_response();
    }

    state.accessions_service.delete_one(id).await
}

#[utoipa::path(
    put,
    path = "/api/v1/accessions/{accession_id}",
    tag = "Accessions",
    request_body = UpdateAccessionRequest,
    responses(
        (status = 200, description = "OK", body = GetOneAccessionResponse),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    security(
        ("jwt_cookie_auth" = []),
        ("api_key_auth" = [])
    )
)]
async fn update_accession(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    authenticated_user: AuthenticatedUser,
    Json(payload): Json<UpdateAccessionRequest>,
) -> Response {
    if !validate_at_least_researcher(&authenticated_user.role) {
        return (StatusCode::FORBIDDEN, "Must have at least researcher role").into_response();
    }
    let subjects_exist = state
        .subjects_service
        .clone()
        .verify_subjects_exist(payload.metadata_subjects.clone(), payload.metadata_language)
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
    use bytes::Bytes;
    use entity::sea_orm_active_enums::DublinMetadataFormat;
    use http_body_util::BodyExt;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tower::ServiceExt;

    async fn build_multipart_form_data(
        metadata_json: serde_json::Value,
        file_bytes: Vec<u8>,
        file_name: &str,
        file_content_type: &str,
        metadata_first: bool,
    ) -> Body {
        let boundary = "------------------------abcdef1234567890";
        let mut form_body_parts: Vec<Bytes> = Vec::new();

        let metadata_part = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"metadata\"\r\nContent-Type: application/json\r\n\r\n{metadata_json}\r\n",
            boundary = boundary,
            metadata_json = metadata_json.to_string()
        );

        let file_part_header = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: {file_content_type}\r\n\r\n",
            boundary = boundary,
            file_name = file_name,
            file_content_type = file_content_type
        );
        let file_part_footer = "\r\n";

        if metadata_first {
            form_body_parts.push(Bytes::from(metadata_part.into_bytes()));
            form_body_parts.push(Bytes::from(file_part_header.into_bytes()));
            form_body_parts.push(Bytes::from(file_bytes));
            form_body_parts.push(Bytes::from(file_part_footer.as_bytes()));
        } else {
            form_body_parts.push(Bytes::from(file_part_header.into_bytes()));
            form_body_parts.push(Bytes::from(file_bytes));
            form_body_parts.push(Bytes::from(file_part_footer.as_bytes()));
            form_body_parts.push(Bytes::from(metadata_part.into_bytes()));
        }

        form_body_parts.push(Bytes::from(format!("--{}--\r\n", boundary).into_bytes()));

        Body::from(form_body_parts.concat())
    }

    #[tokio::test]
    async fn run_one_crawl() {
        let accessions_service = build_test_accessions_service();
        accessions_service
            .create_one(
                CreateAccessionRequest {
                    url: "".to_string(),
                    metadata_language: MetadataLanguage::English,
                    metadata_title: "".to_string(),
                    metadata_description: Some("".to_string()),
                    metadata_time: Default::default(),
                    browser_profile: None,
                    metadata_subjects: vec![1, 2, 3],
                    is_private: false,
                    metadata_format: DublinMetadataFormat::Wacz,
                    s3_filename: Some("test-file.wacz".to_string()),
                },
                "archiver@gmail.com".to_string(),
            )
            .await;
    }

    #[tokio::test]
    async fn run_one_crawl_without_description() {
        let accessions_service = build_test_accessions_service();
        accessions_service
            .create_one(
                CreateAccessionRequest {
                    url: "".to_string(),
                    metadata_language: MetadataLanguage::English,
                    metadata_title: "".to_string(),
                    metadata_subjects: vec![1, 2, 3],
                    metadata_description: None,
                    metadata_time: Default::default(),
                    browser_profile: None,
                    is_private: true,
                    metadata_format: DublinMetadataFormat::Wacz,
                    s3_filename: Some("test-file-2.wacz".to_string()),
                },
                "emailsare4eva@aol.com".to_string(),
            )
            .await;
    }
    #[tokio::test]
    async fn create_one_accession_crawl() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/accessions/crawl")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
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
    "is_private": false,
    "metadata_format": "wacz",
    "s3_filename": "guardian-article.wacz"
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
    async fn create_one_accession_crawl_no_description() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/accessions/crawl")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
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
                            "is_private": true,
                            "metadata_format": "wacz",
                            "s3_filename": "facebook-story.wacz"
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
            accession: mocked_resp.into(),
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
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual: GetOneAccessionResponse = serde_json::from_slice(&body).unwrap();
        let mocked_query = mock_one_accession_with_metadata();
        let expected = GetOneAccessionResponse {
            accession: mocked_query.into(),
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
            accession: mocked_resp.into(),
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
    async fn list_accessions_no_query_params() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/accessions")
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
                    .uri("/api/v1/accessions/private?page=0&per_page=1&lang=english")
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
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
    async fn list_accessions_private_with_auth_no_query_params() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/accessions/private?page=0&per_page=1&lang=english")
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
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
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
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
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
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
            accession: mocked_resp.into(),
            wacz_url: "my url".to_owned(),
        };
        assert_eq!(actual, expected)
    }

    #[tokio::test]
    async fn create_accession_raw_no_auth() {
        let app = build_test_app();
        let metadata = json!({
            "metadata_language": "english",
            "metadata_title": "Test Title",
            "metadata_description": "Test Description",
            "metadata_time": "2024-01-01T00:00:00",
            "metadata_subjects": [1],
            "is_private": false,
            "metadata_format": "wacz",
            "original_url": "https://coolurl.com",
            "s3_filename": "test-no-auth.wacz"
        });
        let file_bytes = vec![0; 100]; // 100 bytes file
        let body = build_multipart_form_data(
            metadata,
            file_bytes,
            "test-file.wacz",
            "application/wacz",
            true,
        )
        .await;

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/accessions/raw")
                    .header(
                        http::header::CONTENT_TYPE,
                        "multipart/form-data; boundary=------------------------abcdef1234567890",
                    )
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_accession_raw_small_file() {
        let app = build_test_app();
        let metadata = json!({
            "metadata_language": "english",
            "metadata_title": "Test Small File",
            "metadata_description": "Small file description",
            "metadata_time": "2024-01-01T00:00:00",
            "metadata_subjects": [1],
            "is_private": false,
            "metadata_format": "wacz",
            "original_url": "https://coolurl.com",
            "s3_filename": "test-small.wacz"
        });
        let file_bytes = vec![0; 1024 * 1024]; // 1MB file
        let body = build_multipart_form_data(
            metadata,
            file_bytes,
            "small-file.wacz",
            "application/wacz",
            true,
        )
        .await;

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/accessions/raw")
                    .header(
                        http::header::CONTENT_TYPE,
                        "multipart/form-data; boundary=------------------------abcdef1234567890",
                    )
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual = String::from_utf8((&body).to_vec()).unwrap();
        assert_eq!(actual, "Accession created with id: 10");
    }

    #[tokio::test]
    async fn create_accession_raw_large_file() {
        let app = build_test_app();
        let metadata = json!({
            "metadata_language": "english",
            "metadata_title": "Test Large File",
            "metadata_description": "Large file description",
            "metadata_time": "2024-01-01T00:00:00",
            "metadata_subjects": [1],
            "is_private": false,
            "metadata_format": "wacz",
            "original_url": "https://coolurl.com",
            "s3_filename": "test-large.wacz"
        });
        let file_bytes = vec![0; 6 * 1024 * 1024]; // 6MB file
        let body = build_multipart_form_data(
            metadata,
            file_bytes,
            "large-file.wacz",
            "application/wacz",
            true,
        )
        .await;

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/accessions/raw")
                    .header(
                        http::header::CONTENT_TYPE,
                        "multipart/form-data; boundary=------------------------abcdef1234567890",
                    )
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual = String::from_utf8((&body).to_vec()).unwrap();
        assert_eq!(actual, "Accession created with id: 10");
    }

    #[tokio::test]
    async fn create_accession_raw_metadata_order_invalid() {
        let app = build_test_app();
        let metadata = json!({
            "metadata_language": "english",
            "metadata_title": "Test Metadata Order",
            "metadata_description": "Metadata order description",
            "metadata_time": "2024-01-01T00:00:00",
            "metadata_subjects": [1],
            "is_private": false,
            "metadata_format": "wacz",
            "original_url": "https://coolurl.com",
            "s3_filename": "test-order-invalid.wacz"
        });
        let file_bytes = vec![0; 100]; // 100 bytes file
        let body = build_multipart_form_data(
            metadata,
            file_bytes,
            "order-invalid-file.wacz",
            "application/wacz",
            false,
        )
        .await; // metadata_first = false

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/accessions/raw")
                    .header(
                        http::header::CONTENT_TYPE,
                        "multipart/form-data; boundary=------------------------abcdef1234567890",
                    )
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual = String::from_utf8((&body).to_vec()).unwrap();
        assert_eq!(actual, "Metadata field should be the first form field");
    }

    #[tokio::test]
    async fn create_accession_raw_invalid_metadata() {
        let app = build_test_app();
        // Missing required field: metadata_title
        let metadata = json!({
            "metadata_language": "english",
            "metadata_description": "Invalid metadata description",
            "metadata_time": "2024-01-01T00:00:00",
            "metadata_subjects": [1],
            "is_private": false,
            "metadata_format": "wacz",
            "original_url": "https://coolurl.com",
            "s3_filename": "test-invalid-metadata.wacz"
        });
        let file_bytes = vec![0; 100]; // 100 bytes file
        let body = build_multipart_form_data(
            metadata,
            file_bytes,
            "invalid-metadata-file.wacz",
            "application/wacz",
            true,
        )
        .await;

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/accessions/raw")
                    .header(
                        http::header::CONTENT_TYPE,
                        "multipart/form-data; boundary=------------------------abcdef1234567890",
                    )
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual = String::from_utf8((&body).to_vec()).unwrap();
        assert!(actual
            .contains("Failed to parse metadata JSON: Error(\"missing field `metadata_title`\""));
    }
}
