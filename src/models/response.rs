//! Response models for the API endpoints.
//!
//! This module contains all the response structures used by the API endpoints,
//! including authentication, crawl operations, and accession management.

use ::entity::sea_orm_active_enums::CrawlStatus;
use chrono::NaiveDateTime;
use entity::accessions_with_metadata::Model as AccessionsWithMetadataModel;
use entity::dublin_metadata_subject_ar::Model as DublinMetadataSubjectArModel;
use entity::dublin_metadata_subject_en::Model as DublinMetadataSubjectEnModel;
use entity::sea_orm_active_enums::MetadataFormat;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AccessionsWithMetadataResponse {
    pub id: i32,
    pub is_private: bool,
    pub crawl_status: CrawlStatus,
    pub crawl_timestamp: NaiveDateTime,
    pub crawl_id: Uuid,
    pub org_id: Uuid,
    pub job_run_id: String,
    pub seed_url: String,
    pub dublin_metadata_date: NaiveDateTime,
    pub dublin_metadata_format: MetadataFormat,
    pub title_en: Option<String>,
    pub description_en: Option<String>,
    pub subjects_en: Option<Vec<String>>,
    pub subjects_en_ids: Option<Vec<i32>>,
    pub title_ar: Option<String>,
    pub description_ar: Option<String>,
    pub subjects_ar: Option<Vec<String>>,
    pub subjects_ar_ids: Option<Vec<i32>>,
    pub has_english_metadata: bool,
    pub has_arabic_metadata: bool,
}

impl From<AccessionsWithMetadataModel> for AccessionsWithMetadataResponse {
    fn from(model: AccessionsWithMetadataModel) -> Self {
        Self {
            id: model.id,
            is_private: model.is_private,
            crawl_status: model.crawl_status,
            crawl_timestamp: model.crawl_timestamp,
            crawl_id: model.crawl_id,
            org_id: model.org_id,
            job_run_id: model.job_run_id,
            seed_url: model.seed_url,
            dublin_metadata_date: model.dublin_metadata_date,
            dublin_metadata_format: model.dublin_metadata_format,
            title_en: model.title_en,
            description_en: model.description_en,
            subjects_en: model.subjects_en,
            subjects_en_ids: model.subjects_en_ids,
            title_ar: model.title_ar,
            description_ar: model.description_ar,
            subjects_ar: model.subjects_ar,
            subjects_ar_ids: model.subjects_ar_ids,
            has_english_metadata: model.has_english_metadata,
            has_arabic_metadata: model.has_arabic_metadata,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct DublinMetadataSubjectArResponse {
    pub id: i32,
    pub subject: String,
}

impl From<DublinMetadataSubjectArModel> for DublinMetadataSubjectArResponse {
    fn from(model: DublinMetadataSubjectArModel) -> Self {
        Self {
            id: model.id,
            subject: model.subject,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct DublinMetadataSubjectEnResponse {
    pub id: i32,
    pub subject: String,
}

impl From<DublinMetadataSubjectEnModel> for DublinMetadataSubjectEnResponse {
    fn from(model: DublinMetadataSubjectEnModel) -> Self {
        Self {
            id: model.id,
            subject: model.subject,
        }
    }
}

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
    pub accession: AccessionsWithMetadataResponse,
    pub wacz_url: String,
}

/// Response for listing accessions with pagination.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub struct ListAccessionsResponse {
    pub items: Vec<AccessionsWithMetadataResponse>,
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
    pub items: Vec<DublinMetadataSubjectArResponse>,
    pub num_pages: u64,
    pub page: u64,
    pub per_page: u64,
}

/// Response for listing English language subjects with pagination.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ListSubjectsEnResponse {
    pub items: Vec<DublinMetadataSubjectEnResponse>,
    pub num_pages: u64,
    pub page: u64,
    pub per_page: u64,
}
