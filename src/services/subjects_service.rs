use crate::models::common::MetadataLanguage;
use crate::models::request::CreateSubjectRequest;
use crate::models::response::{ListSubjectsArResponse, ListSubjectsEnResponse};
use crate::repos::subjects_repo::SubjectsRepo;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::NaiveDateTime;
use http::StatusCode;
use std::sync::Arc;
use tracing::{error, info};

#[derive(Clone)]
pub struct SubjectsService {
    pub subjects_repo: Arc<dyn SubjectsRepo>,
}

impl SubjectsService {
    pub async fn create_one(self, payload: CreateSubjectRequest) -> Response {
        info!(
            "Creating new {} subject {}...",
            payload.lang, payload.metadata_subject
        );
        let write_result = self.subjects_repo.write_one(payload).await;
        match write_result {
            Err(write_error) => {
                error!(%write_error, "Error occurred writing subject");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error").into_response()
            }
            Ok(new_subject) => (StatusCode::CREATED, Json(new_subject)).into_response(),
        }
    }

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
        };
    }
}
