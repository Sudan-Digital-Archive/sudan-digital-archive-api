//! Response models for the API endpoints.
//!
//! This module contains all the response structures used by the API endpoints,
//! including authentication, crawl operations, and accession management.

use ::entity::accessions_with_metadata::Model as AccessionsWithMetadataModel;
use ::entity::dublin_metadata_subject_ar::Model as DublinMetadataSubjectArModel;
use ::entity::dublin_metadata_subject_en::Model as DublinMetadataSubjectEnModel;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Response from authentication endpoint containing JWT token.
#[derive(Deserialize, ToSchema)]
pub struct AuthResponse {
    pub access_token: String,
}

/// Response from crawl creation endpoint.
#[derive(Deserialize, ToSchema)]
pub struct CreateCrawlResponse {
    /// Unique identifier for the created crawl
    pub id: Uuid,
    /// Job identifier for the initiated crawl
    pub run_now_job: String,
}

/// Response containing crawl status information.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetCrawlResponse {
    pub last_crawl_state: String,
}

/// Response containing WACZ file information.
#[derive(Deserialize, ToSchema)]
pub struct GetWaczUrlResponse {
    pub resources: Vec<WaczItem>,
}

/// Individual WACZ file item information.
#[derive(Deserialize, ToSchema)]
pub struct WaczItem {
    pub path: String,
}

/// Response for retrieving a single accession with its metadata.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct GetOneAccessionResponse {
    pub accession: AccessionsWithMetadataModel,
    pub wacz_url: String,
}

/// Response for listing accessions with pagination.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub struct ListAccessionsResponse {
    pub items: Vec<AccessionsWithMetadataModel>,
    pub num_pages: u64,
    pub page: u64,
    pub per_page: u64,
}

/// Response containing a single subject with its identifier.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct SubjectResponse {
    pub id: i32,
    pub subject: String,
}

/// Response for listing Arabic language subjects with pagination.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ListSubjectsArResponse {
    pub items: Vec<DublinMetadataSubjectArModel>,
    pub num_pages: u64,
    pub page: u64,
    pub per_page: u64,
}

/// Response for listing English language subjects with pagination.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ListSubjectsEnResponse {
    pub items: Vec<DublinMetadataSubjectEnModel>,
    pub num_pages: u64,
    pub page: u64,
    pub per_page: u64,
}
