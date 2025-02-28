//! Response models for the API endpoints.
//!
//! This module contains all the response structures used by the API endpoints,
//! including authentication, crawl operations, and accession management.

use ::entity::accessions_with_metadata::Model as AccessionsWithMetadataModel;
use ::entity::dublin_metadata_subject_ar::Model as DublinMetadataSubjectArModel;
use ::entity::dublin_metadata_subject_en::Model as DublinMetadataSubjectEnModel;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Response from authentication endpoint containing JWT token.
#[derive(Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
}

/// Response from crawl creation endpoint.
#[derive(Deserialize)]
pub struct CreateCrawlResponse {
    /// Unique identifier for the created crawl
    pub id: Uuid,
    /// Job identifier for the initiated crawl
    pub run_now_job: String,
}

/// Response containing crawl status information.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCrawlResponse {
    pub last_crawl_state: String,
}

/// Response containing WACZ file information.
#[derive(Deserialize)]
pub struct GetWaczUrlResponse {
    pub resources: Vec<WaczItem>,
}

/// Individual WACZ file item information.
#[derive(Deserialize)]
pub struct WaczItem {
    pub path: String,
}

/// Response for retrieving a single accession with its metadata.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct GetOneAccessionResponse {
    pub accession: AccessionsWithMetadataModel,
    pub wacz_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SubjectResponse {
    pub id: i32,
    pub subject: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListSubjectsArResponse {
    pub items: Vec<DublinMetadataSubjectArModel>,
    pub num_pages: u64,
    pub page: u64,
    pub per_page: u64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ListSubjectsEnResponse {
    pub items: Vec<DublinMetadataSubjectEnModel>,
    pub num_pages: u64,
    pub page: u64,
    pub per_page: u64,
}
