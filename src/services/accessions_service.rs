//! Service layer for managing archival accessions (records).
//!
//! This module handles the business logic for creating, retrieving, and listing
//! archival records, including their associated web crawls and metadata in both
//! Arabic and English.

use crate::models::common::MetadataLanguage;
use crate::models::request::{CreateAccessionRequest, CreateCrawlRequest};
use crate::models::response::{
    GetOneAccessionResponse, ListAccessionsArResponse, ListAccessionsEnResponse,
};
use crate::repos::accessions_repo::AccessionsRepo;
use crate::repos::browsertrix_repo::BrowsertrixRepo;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::NaiveDateTime;
use entity::sea_orm_active_enums::CrawlStatus;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};

/// Service for managing archival accessions and their associated web crawls.
/// Uses dynamic traits for dependency injection
#[derive(Clone)]
pub struct AccessionsService {
    pub accessions_repo: Arc<dyn AccessionsRepo>,
    pub browsertrix_repo: Arc<dyn BrowsertrixRepo>,
}

impl AccessionsService {
    /// Lists paginated accessions with optional filtering.
    ///
    /// # Arguments
    /// * `page` - Page number to retrieve
    /// * `per_page` - Number of items per page
    /// * `metadata_language` - Language of the metadata (Arabic or English)
    /// * `query_term` - Optional search term to filter results
    /// * `date_from` - Optional start date for filtering
    /// * `date_to` - Optional end date for filtering
    ///
    /// # Returns
    /// Returns a JSON response containing paginated accessions or an error response
    pub async fn list(
        self,
        page: u64,
        per_page: u64,
        metadata_language: MetadataLanguage,
        query_term: Option<String>,
        date_from: Option<NaiveDateTime>,
        date_to: Option<NaiveDateTime>,
    ) -> Response {
        info!("Getting page {page} of accessions with per page {per_page}...");
        match metadata_language {
            MetadataLanguage::Arabic => {
                let rows = match self
                    .accessions_repo
                    .list_paginated_ar(page, per_page, query_term, date_from, date_to)
                    .await
                {
                    Ok(rows) => rows,
                    Err(err) => {
                        error!(%err, "Error occurred paginating accessions in Arabic");
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error")
                            .into_response();
                    }
                };
                let rows = ListAccessionsArResponse {
                    items: rows.0,
                    num_pages: rows.1,
                    page,
                    per_page,
                };
                Json(rows).into_response()
            }
            MetadataLanguage::English => {
                let rows = match self
                    .accessions_repo
                    .list_paginated_en(page, per_page, query_term, date_from, date_to)
                    .await
                {
                    Ok(rows) => rows,
                    Err(err) => {
                        error!(%err, "Error occurred paginating accessions in English");
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error")
                            .into_response();
                    }
                };
                let rows = ListAccessionsEnResponse {
                    items: rows.0,
                    num_pages: rows.1,
                    page,
                    per_page,
                };
                Json(rows).into_response()
            }
        }
    }

    /// Retrieves a single accession by ID with its associated metadata and WACZ URL.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the accession
    ///
    /// # Returns
    /// Returns a JSON response containing the accession details or an error response
    pub async fn get_one(self, id: i32) -> Response {
        info!("Getting accession with id {id}");
        let query_result = self.accessions_repo.get_one(id).await;
        match query_result {
            Err(query_result) => {
                error!(%query_result, "Error occurred retrieving accession");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error").into_response()
            }
            Ok(query_result) => match query_result.0 {
                Some(accession_record) => {
                    match self
                        .browsertrix_repo
                        .get_wacz_url(&accession_record.job_run_id)
                        .await
                    {
                        Ok(wacz_url) => {
                            let resp = GetOneAccessionResponse {
                                accession: accession_record,
                                metadata_ar: query_result.1,
                                metadata_en: query_result.2,
                                wacz_url,
                            };
                            Json(resp).into_response()
                        }
                        Err(err) => {
                            error!(%err, "Error occurred retrieiving wacz url");
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Error retrieving wacz url",
                            )
                                .into_response()
                        }
                    }
                }
                None => (StatusCode::NOT_FOUND, "No such record").into_response(),
            },
        }
    }

    /// Creates a new accession by initiating a web crawl and storing the metadata.
    ///
    /// This method performs the following steps:
    /// 1. Launches a web crawl for the specified URL
    /// 2. Polls the crawl status for up to 30 minutes
    /// 3. Creates an accession record once the crawl is complete
    ///
    /// # Arguments
    /// * `payload` - The creation request containing URL and metadata
    pub async fn create_one(self, payload: CreateAccessionRequest) {
        let create_crawl_request = CreateCrawlRequest {
            url: payload.url.clone(),
            browser_profile: payload.browser_profile.clone(),
        };
        let resp = self
            .browsertrix_repo
            .create_crawl(create_crawl_request)
            .await;
        match resp {
            Err(err) => {
                error!(%err, "Error occurred launching browsertrix crawl");
            }
            Ok(resp) => {
                info!("Launched crawl request for url {}", payload.url.clone());
                let time_to_sleep = Duration::from_secs(60);
                let time_to_sleep_as_secs = time_to_sleep.as_secs();
                let mut count = 0;
                while count <= 30 {
                    count += 1;
                    info!("Polled {count} time(s) for url {}", payload.url.clone());
                    let get_crawl_resp = self.browsertrix_repo.get_crawl_status(resp.id).await;
                    match get_crawl_resp {
                        Ok(valid_crawl_resp) => {
                            if valid_crawl_resp == "complete" {
                                let crawl_time_secs = (time_to_sleep * count).as_secs();
                                info!(%valid_crawl_resp, %count, "Crawl complete after {crawl_time_secs}s");
                                let trimmed_title = payload.metadata_title.trim().to_string();
                                let trimmed_description = match payload.metadata_description {
                                    Some(description) => Some(description.trim().to_string()),
                                    None => None,
                                };
                                let create_accessions_request = CreateAccessionRequest {
                                    url: payload.url,
                                    browser_profile: payload.browser_profile,
                                    metadata_language: payload.metadata_language,
                                    metadata_title: trimmed_title,
                                    metadata_description: trimmed_description,
                                    metadata_time: payload.metadata_time,
                                };
                                let write_result = self
                                    .accessions_repo
                                    .write_one(
                                        create_accessions_request,
                                        self.browsertrix_repo.get_org_id(),
                                        resp.id,
                                        resp.run_now_job,
                                        CrawlStatus::Complete,
                                    )
                                    .await;
                                if let Err(err) = write_result {
                                    error!(%err, "Error occurred writing crawl result to db!");
                                }
                                break;
                            } else {
                                sleep(time_to_sleep).await;
                            }
                        }
                        Err(invalid_crawl_resp) => {
                            error!(%invalid_crawl_resp, "Invalid crawl response, trying again in {time_to_sleep_as_secs}s");
                            sleep(time_to_sleep).await;
                        }
                    }
                }
            }
        }
    }
}
