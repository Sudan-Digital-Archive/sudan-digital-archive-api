//! Service layer for managing archival accessions (records).
//!
//! This module handles the business logic for creating, retrieving, and listing
//! archival records, including their associated web crawls and metadata in both
//! Arabic and English.
use crate::models::request::AccessionPaginationWithPrivate;
use crate::models::request::{CreateAccessionRequest, CreateCrawlRequest, UpdateAccessionRequest};
use crate::models::response::{GetOneAccessionResponse, ListAccessionsResponse};
use crate::repos::accessions_repo::AccessionsRepo;
use crate::repos::browsertrix_repo::BrowsertrixRepo;
use crate::repos::emails_repo::EmailsRepo;
use crate::repos::s3_repo::S3Repo;
use ::entity::accessions_with_metadata::Model as AccessionWithMetadataModel;
use axum::extract::multipart::Field;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use bytes::Bytes;
use entity::sea_orm_active_enums::{CrawlStatus, DublinMetadataFormat};
use futures::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncRead;
use tokio::time::sleep;
use tracing::{error, info};
use uuid::Uuid;

/// Service for managing archival accessions and their associated web crawls.
/// Uses dynamic traits for dependency injection
#[derive(Clone)]
pub struct AccessionsService {
    pub accessions_repo: Arc<dyn AccessionsRepo>,
    pub browsertrix_repo: Arc<dyn BrowsertrixRepo>,
    pub emails_repo: Arc<dyn EmailsRepo>,
    pub s3_repo: Arc<dyn S3Repo>,
}

impl AccessionsService {
    /// Lists paginated accessions with optional filtering.
    ///
    /// # Arguments
    /// * `params` - Struct containing all pagination and filtering parameters
    ///
    /// # Returns
    /// JSON response containing paginated accessions or an error response
    pub async fn list(self, params: AccessionPaginationWithPrivate) -> Response {
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
                    items: rows.0.into_iter().map(Into::into).collect(),

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
    pub async fn get_one(self, id: i32, private: bool) -> Response {
        info!("Getting {private} accession with id {id}");
        let query_result = self.accessions_repo.get_one(id, private).await;
        match query_result {
            Err(query_result) => {
                error!(%query_result, "Error occurred retrieving accession");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error").into_response()
            }
            Ok(query_result) => {
                if let Some(accession) = query_result {
                    let accession_for_enrich = accession.clone();
                    match (
                        accession_for_enrich.s3_filename,
                        accession_for_enrich.dublin_metadata_format,
                    ) {
                        (Some(s3_filename), DublinMetadataFormat::Wacz) => {
                            match self.s3_repo.get_presigned_url(&s3_filename, 3600).await {
                                Ok(presigned_url) => {
                                    let resp = GetOneAccessionResponse {
                                        accession: accession.into(),
                                        wacz_url: presigned_url,
                                    };
                                    Json(resp).into_response()
                                }
                                Err(err) => {
                                    error!(%err, "Error occurred generating presigned url");
                                    (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        "Could not retrieving wacz url from s3 storage",
                                    )
                                        .into_response()
                                }
                            }
                        }
                        _ => self.enrich_one_with_browsertrix(Some(accession)).await,
                    }
                } else {
                    (StatusCode::NOT_FOUND, "No such record").into_response()
                }
            }
        }
    }

    async fn enrich_one_with_browsertrix(
        self,
        query_result: Option<AccessionWithMetadataModel>,
    ) -> Response {
        match query_result {
            Some(accession) => {
                match self
                    .browsertrix_repo
                    .get_wacz_url(&accession.job_run_id)
                    .await
                {
                    Ok(wacz_url) => {
                        let resp = GetOneAccessionResponse {
                            accession: accession.into(),
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
    /// * `user_email` - Email address to send user to upon successful crawl
    pub async fn create_one(self, payload: CreateAccessionRequest, user_email: String) {
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

                                let wacz_bytes = match self
                                    .browsertrix_repo
                                    .download_wacz(&resp.run_now_job)
                                    .await
                                {
                                    Ok(bytes) => bytes,
                                    Err(err) => {
                                        error!(%err, "Error occurred downloading WACZ file, aborting accession creation");
                                        return;
                                    }
                                };

                                let unique_filename = format!("{}.wacz", Uuid::new_v4());
                                if let Err(err) = self
                                    .s3_repo
                                    .upload_from_bytes(
                                        &unique_filename,
                                        wacz_bytes,
                                        "application/wacz",
                                    )
                                    .await
                                {
                                    error!(%err, "Error occurred uploading WACZ file to S3, aborting accession creation");
                                    return;
                                };
                                info!("WACZ file uploaded to S3 with filename {}", unique_filename);
                                let create_accessions_request = CreateAccessionRequest {
                                    url: payload.url.clone(),
                                    browser_profile: payload.browser_profile,
                                    metadata_language: payload.metadata_language,
                                    metadata_title: trimmed_title,
                                    metadata_description: trimmed_description,
                                    metadata_time: payload.metadata_time,
                                    metadata_subjects: payload.metadata_subjects,
                                    is_private: payload.is_private,
                                    metadata_format: DublinMetadataFormat::Wacz,
                                    s3_filename: Some(unique_filename.clone()),
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
                                match write_result {
                                    Err(err) => {
                                        error!(%err, "Error occurred writing crawl result to db!");
                                    }
                                    Ok(id) => {
                                        info!("Crawl result written to db successfully");
                                        let email_subject =
                                            format!("Your URL {} has been archived!", payload.url);
                                        let email_body = format!(
                                            "We have archived your <a href='https://sudandigitalarchive.com/archive/{}?isPrivate={}&lang={}'>url</a>.",
                                            id, payload.is_private, payload.metadata_language
                                        );
                                        let email_result = self
                                            .emails_repo
                                            .send_email(user_email, email_subject, email_body)
                                            .await;
                                        info!(
                                            "Email sent to user with id {id} for url {}",
                                            payload.url
                                        );
                                        if let Err(err) = email_result {
                                            error!(%err, "Error occurred sending email to user");
                                        }
                                    }
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

    /// Deletes a single accession by ID.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the accession
    ///
    /// # Returns
    /// Response indicating success or failure of the deletion
    pub async fn delete_one(self, id: i32) -> Response {
        info!("Deleting accession with id {id}");
        let delete_result = self.accessions_repo.delete_one(id).await;
        match delete_result {
            Err(err) => {
                error!(%err, "Error occurred deleting accession");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error").into_response()
            }
            Ok(delete_result) => {
                if delete_result.is_some() {
                    (StatusCode::OK, "Accession deleted").into_response()
                } else {
                    (StatusCode::NOT_FOUND, "No such record").into_response()
                }
            }
        }
    }

    /// Updates a single accession by ID.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the accession
    /// * `payload` - The update request containing new metadata
    ///
    /// # Returns
    /// Response indicating success or failure of the update
    pub async fn update_one(self, id: i32, payload: UpdateAccessionRequest) -> Response {
        info!("Updating accession with id {id}");
        let update_result = self.accessions_repo.update_one(id, payload).await;
        match update_result {
            Err(err) => {
                error!(%err, "Error occurred updating accession");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error").into_response()
            }
            Ok(update_result) => {
                if update_result.is_some() {
                    self.enrich_one_with_browsertrix(update_result).await
                } else {
                    error!("Error occurred finding accession in view after update");
                    (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
                }
            }
        }
    }

    /// Uploads a file from a stream to S3 and returns the upload ID.
    ///
    /// # Arguments
    /// * `key` - The S3 object key where the file will be uploaded
    /// * `content_type` - The MIME type of the file
    /// * `reader` - The async read stream of the file
    ///
    /// # Returns
    /// Result containing the upload ID or an error response
    pub async fn upload_from_stream(
        self,
        key: String,
        content_type: String,
        reader: Pin<&mut (dyn AsyncRead + Send)>,
    ) -> Result<String, Response> {
        match self
            .s3_repo
            .upload_from_stream(&key, reader, &content_type)
            .await
        {
            Ok(upload_id) => {
                info!(
                    "Successfully uploaded file with key: {} and content type: {}",
                    key, content_type
                );
                Ok(upload_id)
            }
            Err(err) => {
                error!(
                    %err,
                    key,
                    content_type,
                    "Failed to upload file to S3. Key: {}, Content-Type: {}",
                    key,
                    content_type
                );
                Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to upload file").into_response())
            }
        }
    }

    /// Uploads chunks from a multipart field directly to S3.
    ///
    /// # Arguments
    /// * `key` - The S3 object key where the file will be uploaded
    /// * `chunks` - Vector of byte chunks to upload
    /// * `content_type` - The MIME type of the file
    ///
    /// # Returns
    /// Result containing the upload ID or an error response
    pub async fn upload_from_chunks(
        self,
        key: String,
        chunks: Vec<Bytes>,
        content_type: String,
    ) -> Result<String, Response> {
        match self
            .s3_repo
            .upload_from_chunks(&key, chunks, &content_type)
            .await
        {
            Ok(upload_id) => {
                info!(
                    "Successfully uploaded chunks with key: {} and content type: {}",
                    key, content_type
                );
                Ok(upload_id)
            }
            Err(err) => {
                error!(
                    %err,
                    key,
                    content_type,
                    "Failed to upload chunks to S3. Key: {}, Content-Type: {}",
                    key,
                    content_type
                );
                Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to upload file").into_response())
            }
        }
    }
}
