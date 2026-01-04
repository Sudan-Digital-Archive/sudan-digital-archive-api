//! Service layer for managing archival accessions (records).
//!
//! This module handles the business logic for creating, retrieving, and listing
//! archival records, including their associated web crawls and metadata in both
//! Arabic and English.
use crate::models::request::AccessionPaginationWithPrivate;
use crate::models::request::{
    CreateAccessionRequest, CreateAccessionRequestRaw, CreateCrawlRequest, UpdateAccessionRequest,
};
use crate::models::response::{GetOneAccessionResponse, ListAccessionsResponse};
use crate::repos::accessions_repo::AccessionsRepo;
use crate::repos::browsertrix_repo::BrowsertrixRepo;
use crate::repos::emails_repo::EmailsRepo;
use crate::repos::s3_repo::S3Repo;
use crate::services::subjects_service::SubjectsService;
use ::entity::accessions_with_metadata::Model as AccessionWithMetadataModel;
use axum::extract::multipart::Field;
use axum::extract::Multipart;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use bytes::Bytes;
use entity::sea_orm_active_enums::{CrawlStatus, DublinMetadataFormat};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use validator::Validate;

static FIVE_MB: usize = 5 * 1024 * 1024;

#[derive(PartialEq, Eq)]
enum MultiPartExtractionStep {
    ExpectMetadata,
    ExpectFile,
}

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

    /// Enriches an accession with WACZ URL from Browsertrix service.
    ///
    /// This private helper function retrieves the WACZ URL for an accession
    /// using the job_run_id and returns an appropriate HTTP response.
    ///
    /// # Arguments
    /// * `query_result` - Optional accession model to enrich
    ///
    /// # Returns
    /// HTTP response with the enriched accession or error status
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

    /// Writes a raw accession record (file-based, no crawl).
    ///
    /// # Arguments
    /// * `payload` - The raw accession request with metadata and S3 filename
    ///
    /// # Returns
    /// Result containing the accession ID or an error response
    pub async fn write_one_raw(self, payload: CreateAccessionRequestRaw) -> Result<i32, Response> {
        info!(
            "Writing raw accession with title: {}",
            payload.metadata_title
        );
        let write_result = self.accessions_repo.write_one_raw(payload).await;
        match write_result {
            Err(err) => {
                error!(%err, "Error occurred writing raw accession to db");
                Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal database error").into_response())
            }
            Ok(id) => {
                info!("Raw accession written to db successfully with id {id}");
                Ok(id)
            }
        }
    }

    /// Uploads a file from a multipart field to S3 with smart chunk handling.
    ///
    /// This method streams the file and decides on upload strategy as it reads:
    /// - Files under 5MB: buffered and uploaded with a single request
    /// - Files over 5MB: multipart upload initiated and chunks streamed directly to S3
    ///
    /// # Arguments
    /// * `key` - The S3 object key where the file will be uploaded
    /// * `field` - The multipart field containing the file data
    /// * `content_type` - The MIME type of the file
    ///
    /// # Returns
    /// Result containing the upload ID or an error response
    async fn upload_from_multipart_field(
        self,
        key: String,
        mut field: Field<'_>,
        content_type: String,
    ) -> Result<String, Response> {
        info!(
            "Starting streaming upload for key: {} with content type: {}",
            key, content_type
        );

        let mut buffer = Vec::with_capacity(FIVE_MB);
        let mut total_size = 0;
        let mut upload_id: Option<String> = None;
        let mut upload_parts: Vec<(String, i32)> = Vec::new();
        let mut part_number = 1i32;
        let mut loop_iteration_counter = 0;
        while let Some(chunk) = field.chunk().await.map_err(|err| {
            error!("Failed to read chunk from field: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read file stream",
            )
                .into_response()
        })? {
            loop_iteration_counter += 1;
            total_size += chunk.len();
            buffer.extend_from_slice(&chunk);
            info!(
                "Received chunk of {} bytes, total so far: {:.1} MB, loop count {loop_iteration_counter}",
                chunk.len(),
                total_size as f64 / 1024.0 / 1024.0
            );

            // case where we are under 5MB so we don't do multipart upload since this requires
            // 5MB otherwise it fails
            if upload_id.is_none() && total_size <= FIVE_MB {
                info!("Skipping on loop iteration {loop_iteration_counter}");
                continue;

            // Case where we haven't started a multipart upload but we're over 5MB, so we need to start one!
            } else if upload_id.is_none() && total_size > FIVE_MB {
                info!(
                    "File exceeded 5MB threshold at {:.1} MB, initiating multipart upload. Loop count {loop_iteration_counter}",
                    total_size as f64 / 1024.0 / 1024.0
                );
                match self
                    .s3_repo
                    .initiate_multipart_upload(&key, &content_type)
                    .await
                {
                    Ok(id) => {
                        upload_id = Some(id.clone());
                        info!("Initiated multipart upload with id: {}", id);
                    }
                    Err(err) => {
                        error!(%err, "Failed to initiate multipart upload for key: {}", key);
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to initiate upload",
                        )
                            .into_response());
                    }
                }
            }
            // Case where we have started a multipart upload already so we need to upload the next chunk!
            if let Some(ref id) = upload_id {
                if buffer.len() <= FIVE_MB {
                    warn!("Waiting for chunk to reach five mb, the min size for each part {loop_iteration_counter}");
                    continue;
                }
                info!("Trying to upload next chunk on {loop_iteration_counter}");
                let part_bytes = Bytes::from(buffer.split_off(0));
                info!(
                    "Uploading part {} with {:.1} MB. Loop count {loop_iteration_counter}",
                    part_number,
                    part_bytes.len() as f64 / 1024.0 / 1024.0
                );
                match self
                    .s3_repo
                    .upload_part(&key, id, part_number, part_bytes)
                    .await
                {
                    Ok((etag, _)) => {
                        upload_parts.push((etag, part_number));
                        info!("Successfully uploaded part {}, loop iteration {loop_iteration_counter}", part_number);
                        part_number += 1;
                    }
                    Err(err) => {
                        error!(%err, "Failed to upload part {} for key: {}", part_number, key);
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to upload file part",
                        )
                            .into_response());
                    }
                }
            } else {
                error!("Multipart upload hasn't started and size exceeded 5MB, which should not happen :-(");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to broker stream into multipart or single upload",
                )
                    .into_response());
            }
        }
        info!("Exited loop");
        // Handle stream end; we now either need to bundle up all the multipart upload parts into the final
        // object or if we didn't do a multipart upload because it was under 5MB, we need to do a single upload
        if let Some(id) = upload_id {
            if !buffer.is_empty() {
                info!(
                    "Uploading final part {} with {:.1} MB",
                    part_number,
                    buffer.len() as f64 / 1024.0 / 1024.0
                );
                let part_bytes = Bytes::from(buffer.split_off(0));
                match self
                    .s3_repo
                    .upload_part(&key, &id, part_number, part_bytes)
                    .await
                {
                    Ok((etag, _)) => {
                        upload_parts.push((etag, part_number));
                        info!("Successfully uploaded final part {}", part_number);
                    }
                    Err(err) => {
                        error!(%err, "Failed to upload final part for key: {}", key);
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to upload final part",
                        )
                            .into_response());
                    }
                }
            }

            info!(
                "Completing multipart upload for key: {} with  parts count: {}",
                key,
                upload_parts.len()
            );
            match self
                .s3_repo
                .complete_multipart_upload(&key, &id, upload_parts)
                .await
            {
                Ok(_) => {
                    info!(
                        "Successfully completed multipart upload for key: {}, total size: {:.1} MB",
                        key,
                        total_size as f64 / 1024.0 / 1024.0
                    );
                    Ok(id)
                }
                Err(err) => {
                    error!(%err, "Failed to complete multipart upload for key: {}", key);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to complete upload",
                    )
                        .into_response())
                }
            }
        } else {
            info!(
                "Using simple upload for {:.1} MB",
                total_size as f64 / 1024.0 / 1024.0
            );
            match self
                .s3_repo
                .upload_from_bytes(&key, Bytes::from(buffer), &content_type)
                .await
            {
                Ok(_) => {
                    info!(
                        "Successfully uploaded file with key: {} and content type: {}",
                        key, content_type
                    );
                    Ok(key)
                }
                Err(err) => {
                    error!(%err, "Failed to upload file to S3. Key: {}, Content-Type: {}", key, content_type);
                    Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to upload file")
                        .into_response())
                }
            }
        }
    }

    /// Extracts and validates accession data from a multipart form submission.
    ///
    /// This method processes a multipart form containing metadata JSON and an optional file upload.
    /// It validates the metadata, checks subject existence, uploads files to S3, and returns a
    /// complete CreateAccessionRequestRaw object ready for database storage.
    ///
    /// The multipart form must have:
    /// - First field: "metadata" containing JSON with accession metadata
    /// - Optional subsequent fields: file uploads with filenames
    ///
    /// # Arguments
    /// * `multipart` - The multipart form data from the HTTP request
    /// * `subjects_service` - Service for validating metadata subjects exist
    ///
    /// # Returns
    /// Result containing the parsed accession request or an HTTP error response
    pub async fn extract_accession_from_multipart_form(
        self,
        mut multipart: Multipart,
        subjects_service: SubjectsService,
    ) -> Result<CreateAccessionRequestRaw, Response> {
        let mut metadata_payload: Option<CreateAccessionRequestRaw> = None;
        let mut uploaded_key: Option<String> = None;
        let mut step = MultiPartExtractionStep::ExpectMetadata; // first field must be the metadata JSON

        while let Some(field) = multipart.next_field().await.map_err(|e| {
            error!("Failed to read multipart field: {e:?}");
            (StatusCode::BAD_REQUEST, "Malformed multipart request").into_response()
        })? {
            let field_name = field.name().unwrap_or("unknown").to_owned();
            let filename_opt = field.file_name().map(str::to_owned);
            let content_type = field
                .content_type()
                .map(str::to_owned)
                .unwrap_or_else(|| "application/octet-stream".to_string());

            info!(
                "Processing multipart field: name={:?}, filename={:?}, content_type={:?}",
                field_name, filename_opt, content_type
            );

            if step == MultiPartExtractionStep::ExpectMetadata {
                if field_name != "metadata" {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "Metadata field should be the first form field",
                    )
                        .into_response());
                }

                let text = field.text().await.map_err(|e| {
                    error!("Failed to read metadata text: {e:?}");
                    (StatusCode::BAD_REQUEST, "Unable to read metadata field").into_response()
                })?;

                let parsed: CreateAccessionRequestRaw =
                    serde_json::from_str(&text).map_err(|e| {
                        let error_msg = format!("Failed to parse metadata JSON: {e:?}");
                        error!(error_msg);
                        (StatusCode::BAD_REQUEST, error_msg).into_response()
                    })?;

                if let Err(v_err) = parsed.validate() {
                    warn!("Invalid create accession request payload: {v_err:?}");
                    return Err((StatusCode::BAD_REQUEST, v_err.to_string()).into_response());
                }

                info!("Extracted and validated metadata JSON");
                let subjects_exist = subjects_service
                    .clone()
                    .verify_subjects_exist(
                        parsed.metadata_subjects.clone(),
                        parsed.metadata_language,
                    )
                    .await;

                match subjects_exist {
                    Err(err) => {
                        return Err(
                            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
                        );
                    }
                    Ok(flag) => {
                        if !flag {
                            return Err(
                                (StatusCode::BAD_REQUEST, "Subjects do not exist").into_response()
                            );
                        }
                    }
                };
                info!("Validated metadata subjects exist");

                metadata_payload = Some(parsed);
                step = MultiPartExtractionStep::ExpectFile;
                continue;
            }

            if filename_opt.is_some() {
                let create_request = metadata_payload.as_mut().ok_or_else(|| {
                    (StatusCode::BAD_REQUEST, "File part arrived before metadata").into_response()
                })?;

                let file_ext = match create_request.metadata_format {
                    DublinMetadataFormat::Wacz => "wacz",
                };

                // Discard the original filename since we have all that from the metadata
                // Use this to make sure there are no filename collisions between objects in s3
                let unique_name = format!("{}.{}", Uuid::new_v4(), file_ext);
                create_request.s3_filename = unique_name.clone();

                let upload_res = self
                    .clone()
                    .upload_from_multipart_field(unique_name.clone(), field, content_type.clone())
                    .await
                    .map_err(|e| {
                        error!("Failed to upload file {unique_name}: {e:?}");
                        e
                    })?;

                uploaded_key = Some(upload_res);
                info!("Successfully uploaded file: {unique_name}");
                continue;
            }

            error!("Skipping unexpected field without filename: name={field_name}");
        }

        let file_key = uploaded_key.ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Missing name of file uploaded to S3",
            )
                .into_response()
        })?;

        if let Some(ref mut req) = metadata_payload {
            req.s3_filename = file_key;
        }

        metadata_payload.ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not extract metadata",
            )
                .into_response()
        })
    }
}
