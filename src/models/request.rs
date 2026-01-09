//! Request models for the API endpoints.
//!
//! This module contains all the request structures used by the API endpoints,
//! including validation rules for incoming data.

use crate::models::common::{BrowserProfile, MetadataLanguage};
use chrono::NaiveDateTime;
use entity::sea_orm_active_enums::DublinMetadataFormat;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

/// Request for creating a new accession with crawl + metadata.
#[derive(Debug, Clone, Validate, Deserialize, ToSchema)]
pub struct CreateAccessionRequest {
    #[validate(url)]
    pub url: String,
    pub metadata_language: MetadataLanguage,
    #[validate(length(min = 1, max = 200))]
    pub metadata_title: String,
    #[validate(length(min = 1, max = 2000))]
    pub metadata_description: Option<String>,
    pub metadata_time: NaiveDateTime,
    pub browser_profile: Option<BrowserProfile>,
    #[validate(length(min = 1, max = 200))]
    #[schema(example = json!([1, 2, 3]))]
    pub metadata_subjects: Vec<i32>,
    pub is_private: bool,
    pub metadata_format: DublinMetadataFormat,
    pub s3_filename: Option<String>,
}

/// Request for creating a new accession from raw file + metadata.
#[derive(Debug, Clone, Validate, Deserialize, ToSchema)]
pub struct CreateAccessionRequestRaw {
    pub metadata_language: MetadataLanguage,
    #[validate(length(min = 1, max = 200))]
    pub metadata_title: String,
    #[validate(length(min = 1, max = 2000))]
    pub metadata_description: Option<String>,
    pub metadata_time: NaiveDateTime,
    #[validate(length(min = 1, max = 200))]
    #[schema(example = json!([1, 2, 3]))]
    pub metadata_subjects: Vec<i32>,
    pub is_private: bool,
    pub metadata_format: DublinMetadataFormat,
    #[validate(url)]
    pub original_url: String,
    pub s3_filename: String,
}

/// Request for initiating a new Browsertrix crawl.
#[derive(Debug, Validate, Deserialize, ToSchema)]
pub struct CreateCrawlRequest {
    #[validate(url)]
    pub url: String,
    pub browser_profile: Option<BrowserProfile>,
}

/// Pagination and filtering parameters for listing accessions.
#[derive(Debug, Clone, Deserialize, Validate, IntoParams, ToSchema)]
#[serde(default)]
pub struct AccessionPagination {
    pub page: u64,
    #[validate(range(min = 1, max = 200))]
    pub per_page: u64,
    pub lang: MetadataLanguage,
    #[schema(example = json!([1, 2, 3]))]
    pub metadata_subjects: Vec<i32>,
    pub metadata_subjects_inclusive_filter: Option<bool>,
    #[validate(length(min = 1, max = 500))]
    pub query_term: Option<String>,
    pub date_from: Option<NaiveDateTime>,
    pub date_to: Option<NaiveDateTime>,
}

impl Default for AccessionPagination {
    fn default() -> Self {
        Self {
            page: 0,
            per_page: 20,
            lang: MetadataLanguage::English,
            metadata_subjects: [].to_vec(),
            metadata_subjects_inclusive_filter: None,
            query_term: None,
            date_from: None,
            date_to: None,
        }
    }
}

/// Pagination and filtering parameters for listing accessions, including private ones.
#[derive(Debug, Clone, Deserialize, Validate, IntoParams, ToSchema)]
#[serde(default)]
pub struct AccessionPaginationWithPrivate {
    pub page: u64,
    #[validate(range(min = 1, max = 200))]
    pub per_page: u64,
    pub lang: MetadataLanguage,
    #[schema(example = json!([1, 2, 3]))]
    pub metadata_subjects: Vec<i32>,
    pub metadata_subjects_inclusive_filter: Option<bool>,
    #[validate(length(min = 1, max = 500))]
    pub query_term: Option<String>,
    pub date_from: Option<NaiveDateTime>,
    pub date_to: Option<NaiveDateTime>,
    pub is_private: bool,
}

impl Default for AccessionPaginationWithPrivate {
    fn default() -> Self {
        Self {
            page: 0,
            per_page: 20,
            lang: MetadataLanguage::English,
            metadata_subjects: [].to_vec(),
            metadata_subjects_inclusive_filter: None,
            query_term: None,
            date_from: None,
            date_to: None,
            is_private: false,
        }
    }
}

/// Request for creating a new subject category.
#[derive(Debug, Clone, Validate, Deserialize, ToSchema)]
pub struct CreateSubjectRequest {
    #[validate(length(min = 1, max = 100))]
    pub metadata_subject: String,
    pub lang: MetadataLanguage,
}

/// Pagination and filtering parameters for listing subjects.
#[derive(Debug, Validate, Deserialize, IntoParams, ToSchema)]
pub struct SubjectPagination {
    pub page: u64,
    #[validate(range(min = 1, max = 200))]
    pub per_page: u64,
    pub lang: MetadataLanguage,
    #[validate(length(min = 1, max = 500))]
    pub query_term: Option<String>,
}

/// Request for creating a new subject category.
#[derive(Debug, Clone, Validate, Deserialize, ToSchema)]
pub struct LoginRequest {
    #[validate(length(min = 1, max = 100))]
    pub email: String,
}

#[derive(Debug, Clone, Validate, Deserialize, ToSchema)]
pub struct AuthorizeRequest {
    pub session_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Validate, Deserialize, ToSchema)]
pub struct UpdateAccessionRequest {
    pub metadata_language: MetadataLanguage,
    #[validate(length(min = 1, max = 200))]
    pub metadata_title: String,
    #[validate(length(min = 1, max = 2000))]
    pub metadata_description: Option<String>,
    pub metadata_time: NaiveDateTime,
    #[validate(length(min = 1, max = 200))]
    #[schema(example = json!([1, 2, 3]))]
    pub metadata_subjects: Vec<i32>,
    pub is_private: bool,
}

/// Request for deleting a subject category.
#[derive(Debug, Clone, Validate, Deserialize, ToSchema)]
pub struct DeleteSubjectRequest {
    pub lang: MetadataLanguage,
}
