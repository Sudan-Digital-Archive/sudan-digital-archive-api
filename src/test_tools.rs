use crate::app_factory::{create_app, AppState};
use crate::models::request::{CreateAccessionRequest, CreateCrawlRequest};
use crate::models::response::CreateCrawlResponse;
use crate::repos::accessions_repo::AccessionsRepo;
use crate::repos::browsertrix_repo::BrowsertrixRepo;
use crate::services::accessions_service::AccessionsService;
use ::entity::accession::Model as AccessionModel;
use ::entity::dublin_metadata_ar::Model as DublinMetataArModel;
use ::entity::dublin_metadata_en::Model as DublinMetadataEnModel;
use async_trait::async_trait;
use axum::http::HeaderValue;
use axum::Router;
use chrono::NaiveDateTime;
use entity::sea_orm_active_enums::CrawlStatus;
use reqwest::{Error, RequestBuilder, Response};
use sea_orm::DbErr;
use std::sync::Arc;
use uuid::Uuid;
use crate::models::common::MetadataLanguage;
use crate::routes::accessions::get_accessions_routes;
use entity::accessions_with_metadata::Model as AccessionsWithMetadataModel;
use crate::repos::subjects_repo::SubjectsRepo;
use crate::services::subjects_service::SubjectsService;

#[derive(Clone, Debug, Default)]
pub struct InMemoryAccessionsRepo {}

#[async_trait]
impl AccessionsRepo for InMemoryAccessionsRepo {
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

    async fn get_one(
        &self,
        _id: i32,
    ) -> Result<Option<AccessionsWithMetadataModel>, DbErr> {
        Ok(Some(one_accession_with_metadata()))
    }

    async fn create_one(&self, _create_accession_request: crate::models::request::CreateAccessionRequest) -> Result<(), DbErr> {
        Ok(())
    }

    async fn list_paginated(
        &self,
        _page: u64,
        _per_page: u64,
        _metadata_language: MetadataLanguage,
        _metadata_subjects: Option<Vec<i32>>,
        _query_term: Option<String>,
        _date_from: Option<chrono::NaiveDateTime>,
        _date_to: Option<chrono::NaiveDateTime>,
    ) -> Result<(Vec<AccessionsWithMetadataModel>, u64), DbErr> {
        Ok(mock_paginated_en())
    }
}

#[derive(Clone, Debug, Default)]
pub struct InMemorySubjectsRepo {}

#[async_trait]
impl SubjectsRepo for InMemorySubjectsRepo {
    async fn write_one(
        &self,
        _create_subject_request: crate::models::request::CreateSubjectRequest,
    ) -> Result<crate::models::response::SubjectResponse, DbErr> {
        Ok(crate::models::response::SubjectResponse {
            id: 1,
            subject: "some cool archive".to_string(),
        })
    }

    async fn list_paginated_ar(
        &self,
        _page: u64,
        _per_page: u64,
        _query_term: Option<String>,
    ) -> Result<(Vec<entity::dublin_metadata_subject_ar::Model>, u64), DbErr> {
        Ok((vec![], 10))
    }

    async fn list_paginated_en(
        &self,
        _page: u64,
        _per_page: u64,
        _query_term: Option<String>,
    ) -> Result<(Vec<entity::dublin_metadata_subject_en::Model>, u64), DbErr> {
        Ok((vec![], 10))
    }

    async fn verify_subjects_exist(
        &self,
        _subject_ids: Vec<i32>,
        _metadata_language: MetadataLanguage,
    ) -> Result<bool, DbErr> {
        Ok(true)
    }
}

pub struct InMemoryBrowsertrixRepo{}
#[async_trait]
impl BrowsertrixRepo for InMemoryBrowsertrixRepo {
    fn get_org_id(&self) -> Uuid {
        Uuid::new_v4()
    }

    async fn refresh_auth(&self) {
        // No-op for tests
    }

    async fn get_wacz_url(&self, _job_run_id: &str) -> Result<String, Error> {
        Ok("my url".to_owned())
    }

    async fn make_request(&self, _req: RequestBuilder) -> Result<Response, Error> {
        Ok(reqwest::Response::from(http::Response::new(
            "mock test data",
        )))
    }

    async fn authenticate(&self) -> Result<String, Error> {
        Ok("test_token".to_string())
    }

    async fn initialize(&mut self) {
        // No-op for tests
    }

    async fn create_crawl(
        &self,
        _create_crawl_request: CreateCrawlRequest,
    ) -> Result<CreateCrawlResponse, Error> {
        Ok(CreateCrawlResponse {
            id: Uuid::new_v4(),
            run_now_job: "test_job_123".to_string(),
        })
    }

    async fn get_crawl_status(&self, _crawl_id: Uuid) -> Result<String, Error> {
        Ok("complete".to_owned())
    }
}

pub fn build_test_accessions_service() -> AccessionsService {
    let accessions_repo = Arc::new(InMemoryAccessionsRepo::default());
    let browsertrix_repo = Arc::new(InMemoryBrowsertrixRepo{});
    AccessionsService {accessions_repo, browsertrix_repo}
}

pub fn build_test_subjects_service() -> SubjectsService {
    let subjects_repo = Arc::new(InMemorySubjectsRepo::default());
    SubjectsService {subjects_repo}
}

pub fn build_test_app() -> Router<AppState> {
    let accessions_service = build_test_accessions_service();
    let subjects_service = build_test_subjects_service();
    let app_state = AppState { accessions_service, subjects_service };
    let app = get_accessions_routes().with_state(app_state);
    app
}

fn one_accession() -> AccessionModel {
    AccessionModel {
        id: 1,
        dublin_metadata_en: None,
        dublin_metadata_ar: None,
        dublin_metadata_date: Default::default(),
        crawl_status: CrawlStatus::BadCrawl,
        crawl_timestamp: Default::default(),
        org_id: Default::default(),
        crawl_id: Default::default(),
        job_run_id: "".to_string(),
        seed_url: "".to_string(),
    }
}

fn one_dublin_metadata_en() -> DublinMetadataEnModel {
    DublinMetadataEnModel {
        id: 1,
        title: "my metadata".to_string(),
        subject: "some cool archive".to_string(),
        description: Some("archival stuff yo".to_string()),
    }
}

fn one_dublin_metadata_ar() -> DublinMetataArModel {
    DublinMetataArModel {
        id: 1,
        title: "بيانات وصفية خاصة بي".to_string(),
        subject: "بعض الأرشيف الرائع".to_string(),
        description: Some("مواد أرشيفية".to_string()),
    }
}

pub fn mock_get_one() -> (
    Option<AccessionModel>,
    Option<DublinMetataArModel>,
    Option<DublinMetadataEnModel>,
) {
    (Some(one_accession()), None, None)
}
pub fn mock_paginated_en() -> (Vec<AccessionsWithMetadataModel>, u64) {
    (vec![one_accession_with_metadata()], 10)
}
pub fn mock_paginated_ar() -> (Vec<AccessionsWithMetadataModel>, u64) {
    (vec![one_accession_with_metadata()], 10)
}
pub fn one_accession_with_metadata() -> AccessionsWithMetadataModel {
    AccessionsWithMetadataModel {
        id: 1,
        url: "some cool archive".to_string(),
        dublin_metadata_date: Default::default(),
        has_arabic_metadata: false,
        has_english_metadata: false,
        title_en: Some("".to_string()),
        description_en: Some("".to_string()),
        title_ar: Some("".to_string()),
        description_ar: Some("".to_string()),
        subjects_en: Some(vec![1,2,3]),
        subjects_ar: Some(vec![1,2,3]),
    }
}
