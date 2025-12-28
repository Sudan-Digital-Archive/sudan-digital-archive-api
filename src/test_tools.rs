//! Test utilities for creating mock implementations and test fixtures.
//! This module provides in-memory implementations of repositories and services
//! to facilitate testing without requiring actual database or external API connections.

use crate::app_factory::{create_app, AppState};
use crate::auth::JWT_KEYS;
use crate::models::auth::JWTClaims;
use crate::models::common::MetadataLanguage;
use crate::models::request::{
    AccessionPaginationWithPrivate, CreateAccessionRequest, CreateAccessionRequestRaw,
    CreateCrawlRequest,
};
use crate::models::response::CreateCrawlResponse;
use crate::repos::accessions_repo::AccessionsRepo;
use crate::repos::auth_repo::{ApiKeyUserInfo, AuthRepo};
use crate::repos::browsertrix_repo::BrowsertrixRepo;
use crate::repos::emails_repo::EmailsRepo;
use crate::repos::s3_repo::S3Repo;
use crate::repos::subjects_repo::SubjectsRepo;
use crate::services::accessions_service::AccessionsService;
use crate::services::auth_service::AuthService;
use crate::services::subjects_service::SubjectsService;
use ::entity::sea_orm_active_enums::{DublinMetadataFormat, Role};
use async_trait::async_trait;
use axum::Router;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use entity::accessions_with_metadata::Model as AccessionsWithMetadataModel;
use entity::dublin_metadata_subject_ar::Model as DublinMetadataSubjectArModel;
use entity::dublin_metadata_subject_en::Model as DublinMetadataSubjectEnModel;
use entity::sea_orm_active_enums::CrawlStatus;
use jsonwebtoken::{encode, Header};
use reqwest::{Error, RequestBuilder, Response};
use sea_orm::DbErr;
use std::error::Error as StdError;
use std::sync::Arc;
use uuid::Uuid;
/// In-memory implementation of AccessionsRepo for testing.
/// Returns predefined mock data instead of interacting with a database.
#[derive(Clone, Debug, Default)]
pub struct InMemoryAccessionsRepo {}

#[async_trait]
impl AccessionsRepo for InMemoryAccessionsRepo {
    /// Mock implementation that always succeeds without storing data.
    async fn write_one(
        &self,
        _create_accession_request: CreateAccessionRequest,
        _org_id: Uuid,
        _crawl_id: Uuid,
        _job_run_id: String,
        _crawl_status: CrawlStatus,
    ) -> Result<i32, DbErr> {
        Ok(10)
    }

    /// Mock implementation for raw accession creation.
    async fn write_one_raw(
        &self,
        _create_accession_request: CreateAccessionRequestRaw,
    ) -> Result<i32, DbErr> {
        Ok(10)
    }

    /// Returns a predefined mock accession.
    async fn get_one(
        &self,
        _id: i32,
        _private: bool,
    ) -> Result<Option<AccessionsWithMetadataModel>, DbErr> {
        Ok(Some(mock_one_accession_with_metadata()))
    }

    /// Returns predefined mock paginated accessions.
    async fn list_paginated(
        &self,
        _params: AccessionPaginationWithPrivate,
    ) -> Result<(Vec<AccessionsWithMetadataModel>, u64), DbErr> {
        Ok(mock_paginated_en())
    }

    async fn delete_one(&self, _id: i32) -> Result<Option<()>, DbErr> {
        Ok(Some(()))
    }

    async fn update_one(
        &self,
        _id: i32,
        _update_accession_request: crate::models::request::UpdateAccessionRequest,
    ) -> Result<Option<AccessionsWithMetadataModel>, DbErr> {
        Ok(Some(mock_one_accession_with_metadata()))
    }
}

/// In-memory implementation of SubjectsRepo for testing.
/// Provides mock data for subject-related operations.
#[derive(Clone, Debug, Default)]
pub struct InMemorySubjectsRepo {}

#[async_trait]
impl SubjectsRepo for InMemorySubjectsRepo {
    /// Returns a predefined subject response without storing data.
    async fn write_one(
        &self,
        _create_subject_request: crate::models::request::CreateSubjectRequest,
    ) -> Result<crate::models::response::SubjectResponse, DbErr> {
        Ok(crate::models::response::SubjectResponse {
            id: 1,
            subject: "some cool archive".to_string(),
        })
    }
    async fn delete_one(
        &self,
        _subject_id: i32,
        _metadata_language: MetadataLanguage,
    ) -> Result<Option<()>, DbErr> {
        Ok(Some(()))
    }
    /// Returns predefined mock Arabic subjects.
    async fn list_paginated_ar(
        &self,
        _page: u64,
        _per_page: u64,
        _query_term: Option<String>,
    ) -> Result<(Vec<DublinMetadataSubjectArModel>, u64), DbErr> {
        Ok(mock_paginated_subjects_ar())
    }

    /// Returns predefined mock English subjects.
    async fn list_paginated_en(
        &self,
        _page: u64,
        _per_page: u64,
        _query_term: Option<String>,
    ) -> Result<(Vec<DublinMetadataSubjectEnModel>, u64), DbErr> {
        Ok(mock_paginated_subjects_en())
    }

    /// Always returns true for subject verification in tests.
    async fn verify_subjects_exist(
        &self,
        _subject_ids: Vec<i32>,
        _metadata_language: MetadataLanguage,
    ) -> Result<bool, DbErr> {
        Ok(true)
    }
}

/// In-memory implementation of EmailsRepo for testing.
#[derive(Clone, Debug, Default)]
pub struct InMemoryEmailsRepo {}

#[async_trait]
impl EmailsRepo for InMemoryEmailsRepo {
    async fn send_email(&self, _to: String, _subject: String, _email: String) -> Result<(), Error> {
        Ok(())
    }
}

/// In-memory implementation of AuthRepo for testing.
#[derive(Clone, Debug, Default)]
pub struct InMemoryAuthRepo {}

#[async_trait]
impl AuthRepo for InMemoryAuthRepo {
    async fn get_user_by_email(&self, _email: String) -> Result<Option<Uuid>, DbErr> {
        Ok(Some(Uuid::new_v4()))
    }

    async fn create_session(&self, _user_id: Uuid) -> Result<Uuid, DbErr> {
        Ok(Uuid::new_v4())
    }

    async fn delete_expired_sessions(&self) {
        // No-op for tests
    }

    async fn get_session_expiry(
        &self,
        _authorize_request: crate::models::request::AuthorizeRequest,
    ) -> Result<Option<chrono::NaiveDateTime>, DbErr> {
        Ok(Some(chrono::NaiveDateTime::default()))
    }

    async fn get_one(&self, _user_id: Uuid) -> Result<Option<entity::archive_user::Model>, DbErr> {
        Ok(Some(entity::archive_user::Model {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            role: entity::sea_orm_active_enums::Role::Admin,
            is_active: true,
        }))
    }

    async fn create_api_key_for_user(&self, _user_id: Uuid) -> Result<String, DbErr> {
        Ok("mock_api_key_secret".to_string())
    }

    async fn verify_api_key(&self, _api_key: String) -> Result<Option<ApiKeyUserInfo>, DbErr> {
        Ok(Some(ApiKeyUserInfo {
            email: "test@example.com".to_string(),
            role: Role::Admin,
        }))
    }

    async fn delete_expired_api_keys(&self) {
        // No-op for tests
    }
}

/// In-memory implementation of BrowsertrixRepo for testing.
/// Mocks interactions with the Browsertrix API.
pub struct InMemoryBrowsertrixRepo {}

#[async_trait]
impl BrowsertrixRepo for InMemoryBrowsertrixRepo {
    /// Returns a random UUID as organization ID.
    fn get_org_id(&self) -> Uuid {
        Uuid::new_v4()
    }

    /// Mock refresh authentication that does nothing.
    async fn refresh_auth(&self) {
        // No-op for tests
    }

    /// Returns a fixed mock URL for WACZ files.
    async fn get_wacz_url(&self, _job_run_id: &str) -> Result<String, Error> {
        Ok("my url".to_owned())
    }
    /// Returns fixed bytes for WACZ file content.
    async fn download_wacz(&self, _crawl_id: &str) -> Result<bytes::Bytes, Error> {
        // Return empty bytes for testing
        Ok(bytes::Bytes::from_static(b"{}"))
    }
    /// Returns a mock response for any request.
    async fn make_request(&self, _req: RequestBuilder) -> Result<Response, Error> {
        Ok(reqwest::Response::from(http::Response::new(
            "mock test data",
        )))
    }

    /// Returns a fixed authentication token.
    async fn authenticate(&self) -> Result<String, Error> {
        Ok("test_token".to_string())
    }

    /// Mock initialization that does nothing.
    async fn initialize(&mut self) {
        // No-op for tests
    }

    /// Returns a mock crawl response with random UUID and fixed job ID.
    async fn create_crawl(
        &self,
        _create_crawl_request: CreateCrawlRequest,
    ) -> Result<CreateCrawlResponse, Error> {
        Ok(CreateCrawlResponse {
            id: Uuid::new_v4(),
            run_now_job: "test_job_123".to_string(),
        })
    }

    /// Returns a fixed "complete" status for any crawl.
    async fn get_crawl_status(&self, _crawl_id: Uuid) -> Result<String, Error> {
        Ok("complete".to_owned())
    }
}
/// Mock implementation for testing
#[derive(Debug, Clone, Default)]
pub struct InMemoryS3Repo {
    #[allow(dead_code)]
    pub bucket: String,
}

#[async_trait]
impl S3Repo for InMemoryS3Repo {
    async fn new(
        bucket: String,
        _endpoint_url: &str,
        _access_key: &str,
        _secret_key: &str,
    ) -> Result<Self, Box<dyn StdError>> {
        Ok(Self { bucket })
    }

    async fn upload_from_bytes(
        &self,
        key: &str,
        _bytes: Bytes,
        _content_type: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Return a deterministic mock ETag based on the key
        Ok(format!("mock-etag-{}", key))
    }

    async fn get_presigned_url(
        &self,
        _object_key: &str,
        _expires_in: u64,
    ) -> Result<String, Box<dyn StdError>> {
        Ok("my url".to_string())
    }

    async fn initiate_multipart_upload(
        &self,
        key: &str,
        _content_type: &str,
    ) -> Result<String, Box<dyn StdError>> {
        Ok(format!("mock-upload-id-{}", key))
    }

    async fn upload_part(
        &self,
        _key: &str,
        _upload_id: &str,
        part_number: i32,
        _bytes: Bytes,
    ) -> Result<(String, i32), Box<dyn StdError>> {
        Ok((format!("mock-etag-part-{}", part_number), part_number))
    }

    async fn complete_multipart_upload(
        &self,
        key: &str,
        _upload_id: &str,
        _parts: Vec<(String, i32)>,
    ) -> Result<String, Box<dyn StdError>> {
        Ok(format!("mock-final-etag-{}", key))
    }
}
/// Builds a test accessions service with in-memory repositories.
/// Useful for unit testing service functionality without database connections.
pub fn build_test_accessions_service() -> AccessionsService {
    let accessions_repo = Arc::new(InMemoryAccessionsRepo::default());
    let browsertrix_repo = Arc::new(InMemoryBrowsertrixRepo {});
    let emails_repo = Arc::new(InMemoryEmailsRepo::default());
    let s3_repo = Arc::new(InMemoryS3Repo {
        bucket: "test-bucket".to_string(),
    });
    AccessionsService {
        accessions_repo,
        browsertrix_repo,
        emails_repo,
        s3_repo,
    }
}

pub fn build_test_auth_service() -> AuthService {
    let auth_repo = Arc::new(InMemoryAuthRepo::default());
    let emails_repo = Arc::new(InMemoryEmailsRepo::default());

    AuthService {
        auth_repo,
        emails_repo,
        jwt_cookie_domain: "test".to_string(),
    }
}

/// Builds a test subjects service with in-memory repository.
pub fn build_test_subjects_service() -> SubjectsService {
    let subjects_repo = Arc::new(InMemorySubjectsRepo::default());
    SubjectsService { subjects_repo }
}

/// Creates a test application instance with in-memory services.
/// The returned Router can be used with axum test utilities.
pub fn build_test_app() -> Router {
    let accessions_service = build_test_accessions_service();
    let subjects_service = build_test_subjects_service();
    let auth_service = build_test_auth_service();
    let app_state = AppState {
        accessions_service,
        subjects_service,
        auth_service,
    };
    create_app(app_state, vec![], true)
}

/// Creates a mock paginated collection of English accessions.
pub fn mock_paginated_en() -> (Vec<AccessionsWithMetadataModel>, u64) {
    (vec![mock_one_accession_with_metadata()], 10)
}

/// Creates a mock paginated collection of Arabic accessions.
pub fn mock_paginated_ar() -> (Vec<AccessionsWithMetadataModel>, u64) {
    (vec![mock_one_accession_with_metadata()], 10)
}

/// Creates a single mock accession with metadata for testing.
pub fn mock_one_accession_with_metadata() -> AccessionsWithMetadataModel {
    AccessionsWithMetadataModel {
        id: 1,
        crawl_status: CrawlStatus::Complete,
        crawl_timestamp: Default::default(),
        crawl_id: Default::default(),
        org_id: Default::default(),
        job_run_id: "some_job_id".to_string(),
        dublin_metadata_date: Default::default(),
        has_arabic_metadata: true,
        has_english_metadata: true,
        title_en: Some("English Title".to_string()),
        description_en: Some("English Description".to_string()),
        title_ar: Some("Arabic Title".to_string()),
        description_ar: Some("Arabic Description".to_string()),
        subjects_en: Some(vec!["archive".to_string()]),
        subjects_ar: Some(vec!["mrhaba archive".to_string()]),
        seed_url: "https://example.com".to_string(),
        subjects_en_ids: Some(vec![1]),
        subjects_ar_ids: Some(vec![3]),
        is_private: true,
        dublin_metadata_format: DublinMetadataFormat::Wacz,
        s3_filename: Some("some_file.wacz".to_string()),
    }
}

/// Creates a collection of mock English subjects for testing.
pub fn mock_paginated_subjects_en() -> (Vec<DublinMetadataSubjectEnModel>, u64) {
    (
        vec![DublinMetadataSubjectEnModel {
            id: 1,
            subject: "English Subject".to_string(),
        }],
        10,
    )
}

/// Creates a collection of mock Arabic subjects for testing.
pub fn mock_paginated_subjects_ar() -> (Vec<DublinMetadataSubjectArModel>, u64) {
    (
        vec![DublinMetadataSubjectArModel {
            id: 1,
            subject: "Arabic Subject".to_string(),
        }],
        10,
    )
}

pub fn get_mock_jwt() -> String {
    let expiry_time: DateTime<Utc> = Utc::now() + chrono::Duration::hours(24);
    let claims = JWTClaims {
        sub: "someuser@gmail.com".to_string(),
        exp: expiry_time.timestamp() as usize,
        role: Role::Admin,
    };
    let jwt =
        encode(&Header::default(), &claims, &JWT_KEYS.encoding).expect("Failed to encode JWT");
    jwt
}
