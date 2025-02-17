//! Response models for the API endpoints.
//!
//! This module contains all the response structures used by the API endpoints,
//! including authentication, crawl operations, and accession management.

use ::entity::accession::Model as AccessionModel;
use ::entity::dublin_metadata_ar::Model as DublinMetataArModel;
use ::entity::dublin_metadata_en::Model as DublinMetadataEnModel;
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
    pub accession: AccessionModel,
    pub metadata_ar: Option<DublinMetataArModel>,
    pub metadata_en: Option<DublinMetadataEnModel>,
    pub wacz_url: String,
}

/// Paginated response for Arabic accession listings.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ListAccessionsArResponse {
    pub items: Vec<(AccessionModel, Option<DublinMetataArModel>)>,
    pub num_pages: u64,
    pub page: u64,
    pub per_page: u64,
}

/// Paginated response for English accession listings.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ListAccessionsEnResponse {
    pub items: Vec<(AccessionModel, Option<DublinMetadataEnModel>)>,
    pub num_pages: u64,
    pub page: u64,
    pub per_page: u64,
}
