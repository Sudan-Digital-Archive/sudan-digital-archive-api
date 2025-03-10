//! Request models for the API endpoints.
//!
//! This module contains all the request structures used by the API endpoints,
//! including validation rules for incoming data.

use crate::models::common::{BrowserProfile, MetadataLanguage};
use chrono::NaiveDateTime;
use serde::Deserialize;
use validator::Validate;

/// Request for creating a new accession with metadata.
#[derive(Debug, Clone, Validate, Deserialize)]
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
    pub metadata_subjects: Vec<i32>,
}

/// Request for initiating a new Browsertrix crawl.
#[derive(Debug, Validate, Deserialize)]
pub struct CreateCrawlRequest {
    #[validate(url)]
    pub url: String,
    pub browser_profile: Option<BrowserProfile>,
}

/// Pagination and filtering parameters for listing accessions.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct AccessionPagination {
    pub page: u64,
    #[validate(range(min = 1, max = 200))]
    pub per_page: u64,
    pub lang: MetadataLanguage,
    pub metadata_subjects: Option<Vec<i32>>,
    #[validate(length(min = 1, max = 500))]
    pub query_term: Option<String>,
    pub date_from: Option<NaiveDateTime>,
    pub date_to: Option<NaiveDateTime>,
}

/// Request for creating a new subject category.
#[derive(Debug, Clone, Validate, Deserialize)]
pub struct CreateSubjectRequest {
    #[validate(length(min = 1, max = 100))]
    pub metadata_subject: String,
    pub lang: MetadataLanguage,
}

/// Pagination and filtering parameters for listing subjects.
#[derive(Debug, Validate, Deserialize)]
pub struct SubjectPagination {
    pub page: u64,
    #[validate(range(min = 1, max = 200))]
    pub per_page: u64,
    pub lang: MetadataLanguage,
    #[validate(length(min = 1, max = 500))]
    pub query_term: Option<String>,
}

/// Request for creating a new subject category.
#[derive(Debug, Clone, Validate, Deserialize)]
pub struct LoginRequest {
    #[validate(length(min = 1, max = 100))]
    pub email: String,
}