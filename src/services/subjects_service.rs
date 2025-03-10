//! Service layer for managing archive metadata subjects.
//!
//! This module handles the business logic for creating and listing subject tags
//! that are used to categorize archival records in both Arabic and English.

use crate::models::common::MetadataLanguage;
use crate::models::request::CreateSubjectRequest;
use crate::models::response::{ListSubjectsArResponse, ListSubjectsEnResponse};
use crate::repos::subjects_repo::SubjectsRepo;
use axum::response::{IntoResponse, Response};
use axum::Json;
use http::StatusCode;
use sea_orm::DbErr;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Service for managing metadata subjects in multiple languages.
/// Uses dynamic traits for dependency injection
#[derive(Clone)]
pub struct SubjectsService {
    pub subjects_repo: Arc<dyn SubjectsRepo>,
}

impl SubjectsService {
    /// Creates a new metadata subject.
    ///
    /// # Arguments
    /// * `payload` - The creation request containing subject text and language
    ///
    /// # Returns
    /// Returns a JSON response with the created subject or an error response
    pub async fn create_one(self, payload: CreateSubjectRequest) -> Response {
        info!(
            "Creating new {} subject {}...",
            payload.lang, payload.metadata_subject
        );
        let write_result = self.subjects_repo.write_one(payload.clone()).await;
        match write_result {
            Err(write_error) => {
                if write_error
                    .to_string()
                    .contains("duplicate key value violates unique constraint")
                {
                    warn!(%write_error,
                        "Can't write {} subject since subject {} already exists",
                        payload.lang, payload.metadata_subject);
                    return (
                        StatusCode::BAD_REQUEST,
                        format!("Subject {} already exists", payload.metadata_subject),
                    )
                        .into_response();
                }
                error!(%write_error, "Error occurred writing subject");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error").into_response()
            }
            Ok(new_subject) => (StatusCode::CREATED, Json(new_subject)).into_response(),
        }
    }

    /// Lists paginated subjects with optional search filtering.
    ///
    /// # Arguments
    /// * `page` - The page number to retrieve
    /// * `per_page` - Number of items per page
    /// * `metadata_language` - Language of subjects to retrieve (Arabic or English)
    /// * `query_term` - Optional search term to filter subjects
    ///
    /// # Returns
    /// Returns a JSON response containing paginated subjects or an error response
    pub async fn list(
        self,
        page: u64,
        per_page: u64,
        metadata_language: MetadataLanguage,
        query_term: Option<String>,
    ) -> Response {
        info!("Getting page {page} of {metadata_language} subjects with per page {per_page}...");
        match metadata_language {
            MetadataLanguage::Arabic => {
                match self
                    .subjects_repo
                    .list_paginated_ar(page, per_page, query_term)
                    .await
                {
                    Ok(rows) => {
                        let list_subjects_resp = ListSubjectsArResponse {
                            items: rows.0,
                            num_pages: rows.1,
                            page,
                            per_page,
                        };
                        Json(list_subjects_resp).into_response()
                    }
                    Err(err) => {
                        error!( % err, "Error occurred paginating {metadata_language} subjects");
                        (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error")
                            .into_response()
                    }
                }
            }
            MetadataLanguage::English => {
                match self
                    .subjects_repo
                    .list_paginated_en(page, per_page, query_term)
                    .await
                {
                    Ok(rows) => {
                        let list_subjects_resp = ListSubjectsEnResponse {
                            items: rows.0,
                            num_pages: rows.1,
                            page,
                            per_page,
                        };
                        Json(list_subjects_resp).into_response()
                    }
                    Err(err) => {
                        error!( % err, "Error occurred paginating {metadata_language} subjects");
                        (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error")
                            .into_response()
                    }
                }
            }
        }
    }

    /// Verifies that all subject IDs in the provided list exist in the database.
    ///
    /// # Arguments
    /// * `metadata_subjects` - List of subject IDs to verify
    /// * `metadata_language` - Language of the subjects to check
    ///
    /// # Returns
    /// Returns true if all subjects exist, false otherwise, or a database error
    pub async fn verify_subjects_exist(
        self,
        metadata_subjects: Vec<i32>,
        metadata_language: MetadataLanguage,
    ) -> Result<bool, DbErr> {
        self.subjects_repo
            .verify_subjects_exist(metadata_subjects, metadata_language)
            .await
    }
}
