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

pub struct InMemoryAccessionsRepo {}
pub struct InMemoryBrowsertrixRepo {}
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
    ) -> Result<
        (
            Option<AccessionModel>,
            Option<DublinMetataArModel>,
            Option<DublinMetadataEnModel>,
        ),
        DbErr,
    > {
        Ok(mock_get_one())
    }

    async fn list_paginated_ar(
        &self,
        _page: u64,
        _per_page: u64,
        _query_term: Option<String>,
        _date_from: Option<NaiveDateTime>,
        _date_to: Option<NaiveDateTime>,
    ) -> Result<(Vec<(AccessionModel, Option<DublinMetataArModel>)>, u64), DbErr> {
        Ok((vec![(one_accession(), Some(one_dublin_metadata_ar()))], 10))
    }

    async fn list_paginated_en(
        &self,
        _page: u64,
        _per_page: u64,
        _query_term: Option<String>,
        _date_from: Option<NaiveDateTime>,
        _date_to: Option<NaiveDateTime>,
    ) -> Result<(Vec<(AccessionModel, Option<DublinMetadataEnModel>)>, u64), DbErr> {
        Ok(mock_paginated_en())
    }
}

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
    let accessions_repo = InMemoryAccessionsRepo {};
    let browsertrix_repo = InMemoryBrowsertrixRepo {};
    AccessionsService {
        accessions_repo: Arc::new(accessions_repo),
        browsertrix_repo: Arc::new(browsertrix_repo),
    }
}
pub fn build_test_app() -> Router {
    let accessions_service = build_test_accessions_service();
    let app_state = AppState { accessions_service };
    let cors_origins = "http://localhost"
        .to_string()
        .split(",")
        .map(|s| {
            HeaderValue::from_str(s)
                .expect("CORS_URL env var should contain comma separated origins")
        })
        .collect();
    create_app(app_state, cors_origins, true)
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
pub fn mock_paginated_en() -> (Vec<(AccessionModel, Option<DublinMetadataEnModel>)>, u64) {
    (vec![(one_accession(), Some(one_dublin_metadata_en()))], 10)
}
pub fn mock_paginated_ar() -> (Vec<(AccessionModel, Option<DublinMetataArModel>)>, u64) {
    (vec![(one_accession(), Some(one_dublin_metadata_ar()))], 10)
}
