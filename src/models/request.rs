//! Request models for the API endpoints.
//! 
//! This module contains all the request structures used by the API endpoints,
//! including validation rules for incoming data.

use crate::models::common::MetadataLanguage;
use chrono::NaiveDateTime;
use serde::Deserialize;
use validator::Validate;

/// Request for creating a new accession with metadata.
#[derive(Debug, Validate, Deserialize)]
pub struct CreateAccessionRequest {
    #[validate(url)]
    pub url: String,
    pub metadata_language: MetadataLanguage,
    #[validate(length(min = 1, max = 200))]
    pub metadata_title: String,
    #[validate(length(min = 1))]
    pub metadata_subject: String,
    #[validate(length(min = 1, max = 2000))]
    pub metadata_description: String,
    pub metadata_time: NaiveDateTime,
}

/// Request for initiating a new Browsertrix crawl.
#[derive(Debug, Validate, Deserialize)]
pub struct CreateCrawlRequest {
    #[validate(url)]
    pub url: String,
}

/// Pagination and filtering parameters for listing accessions.
#[derive(Debug, Validate, Deserialize)]
pub struct Pagination {
    pub page: u64,
    #[validate(range(min = 1, max = 200))]
    pub per_page: u64,
    pub lang: MetadataLanguage,
    #[validate(length(min = 1, max = 500))]
    pub query_term: Option<String>,
    pub date_from: Option<NaiveDateTime>,
    pub date_to: Option<NaiveDateTime>,
}
