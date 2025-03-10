//! Service layer for managing archival accessions (records).
//!
//! This module handles the business logic for creating, retrieving, and listing
//! archival records, including their associated web crawls and metadata in both
//! Arabic and English.

use crate::models::request::AccessionPagination;
use crate::models::request::{CreateAccessionRequest, CreateCrawlRequest};
use crate::models::response::{GetOneAccessionResponse, ListAccessionsResponse};
use crate::repos::accessions_repo::AccessionsRepo;
use crate::repos::browsertrix_repo::BrowsertrixRepo;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
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
    /// * `params` - Struct containing all pagination and filtering parameters
    ///
    /// # Returns
    /// JSON response containing paginated accessions or an error response
    pub async fn list(self, params: AccessionPagination) -> Response {
        info!(
            "Getting page {} of {} accessions with per page {}...",
            params.page, params.lang, params.per_page
        );

        let rows = self.accessions_repo.list_paginated(params.clone()).await;

        match rows {
            Err(err) => {
                error!(%err, "Error occurred paginating accessions");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error").into_response()
            }
            Ok(rows) => {
                let resp = ListAccessionsResponse {
                    items: rows.0,
                    num_pages: rows.1,
                    page: params.page,
                    per_page: params.per_page,
                };
                Json(resp).into_response()
            }
        }
    }

    /// Retrieves a single accession by ID with its associated metadata and WACZ URL.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the accession
    ///
    /// # Returns
    /// JSON response containing the accession details or an error response
    pub async fn get_one(self, id: i32) -> Response {
        info!("Getting accession with id {id}");
        let query_result = self.accessions_repo.get_one(id).await;
        match query_result {
            Err(query_result) => {
                error!(%query_result, "Error occurred retrieving accession");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error").into_response()
            }
            Ok(query_result) => match query_result {
                Some(accession) => {
                    match self
                        .browsertrix_repo
                        .get_wacz_url(&accession.job_run_id)
                        .await
                    {
                        Ok(wacz_url) => {
                            let resp = GetOneAccessionResponse {
                                accession,
                                wacz_url,
                            };
                            Json(resp).into_response()
                        }
                        Err(err) => {
                            error!(%err, "Error occurred retrieving wacz url");
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
    /// You should validate that `metadata_subjects` exist in the
    /// payload before calling this method - it will error out
    /// if they don't.
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
                                let trimmed_description = payload
                                    .metadata_description
                                    .map(|description| description.trim().to_string());
                                let create_accessions_request = CreateAccessionRequest {
                                    url: payload.url,
                                    browser_profile: payload.browser_profile,
                                    metadata_language: payload.metadata_language,
                                    metadata_title: trimmed_title,
                                    metadata_description: trimmed_description,
                                    metadata_time: payload.metadata_time,
                                    metadata_subjects: payload.metadata_subjects,
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
