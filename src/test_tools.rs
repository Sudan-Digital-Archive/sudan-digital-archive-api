//! Test utilities for creating mock implementations and test fixtures.
//! This module provides in-memory implementations of repositories and services
//! to facilitate testing without requiring actual database or external API connections.

use crate::app_factory::{create_app, AppState};
use crate::models::common::MetadataLanguage;
use crate::models::request::{AccessionPagination, CreateAccessionRequest, CreateCrawlRequest};
use crate::models::response::CreateCrawlResponse;
use crate::repos::accessions_repo::AccessionsRepo;
use crate::repos::browsertrix_repo::BrowsertrixRepo;
use crate::repos::subjects_repo::SubjectsRepo;
use crate::services::accessions_service::AccessionsService;
use crate::services::subjects_service::SubjectsService;
use async_trait::async_trait;
use axum::Router;
use entity::accessions_with_metadata::Model as AccessionsWithMetadataModel;
use entity::dublin_metadata_subject_ar::Model as DublinMetadataSubjectArModel;
use entity::dublin_metadata_subject_en::Model as DublinMetadataSubjectEnModel;
use entity::sea_orm_active_enums::CrawlStatus;
use reqwest::{Error, RequestBuilder, Response};
use sea_orm::DbErr;
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
    ) -> Result<(), DbErr> {
        Ok(())
    }

    /// Returns a predefined mock accession.
    async fn get_one(&self, _id: i32) -> Result<Option<AccessionsWithMetadataModel>, DbErr> {
        Ok(Some(mock_one_accession_with_metadata()))
    }

    /// Returns predefined mock paginated accessions.
    async fn list_paginated(
        &self,
        _params: AccessionPagination,
    ) -> Result<(Vec<AccessionsWithMetadataModel>, u64), DbErr> {
        Ok(mock_paginated_en())
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

/// Builds a test accessions service with in-memory repositories.
/// Useful for unit testing service functionality without database connections.
pub fn build_test_accessions_service() -> AccessionsService {
    let accessions_repo = Arc::new(InMemoryAccessionsRepo::default());
    let browsertrix_repo = Arc::new(InMemoryBrowsertrixRepo {});
    AccessionsService {
        accessions_repo,
        browsertrix_repo,
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
    let app_state = AppState {
        accessions_service,
        subjects_service,
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
        crawl_status: CrawlStatus::BadCrawl,
        crawl_timestamp: Default::default(),
        crawl_id: Default::default(),
        org_id: Default::default(),
        job_run_id: "".to_string(),
        dublin_metadata_date: Default::default(),
        has_arabic_metadata: false,
        has_english_metadata: false,
        title_en: Some("".to_string()),
        description_en: Some("".to_string()),
        title_ar: Some("".to_string()),
        description_ar: Some("".to_string()),
        subjects_en: Some(vec!["archive".to_string()]),
        subjects_ar: Some(vec!["mrhaba archive".to_string()]),
        seed_url: "".to_string(),
        subjects_en_ids: Some(vec![1]),
        subjects_ar_ids: Some(vec![3]),
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
