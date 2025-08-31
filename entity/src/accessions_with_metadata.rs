use super::sea_orm_active_enums::CrawlStatus;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(table_name = "accessions_with_metadata")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub is_private: bool,
    pub crawl_status: CrawlStatus,
    pub crawl_timestamp: DateTime,
    pub crawl_id: Uuid,
    pub org_id: Uuid,
    pub job_run_id: String,
    pub seed_url: String,
    pub dublin_metadata_date: DateTime,
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

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AccessionsWithMetadataSchemaModel {
    pub id: i32,
    pub is_private: bool,
    pub crawl_status: CrawlStatus,
    pub crawl_timestamp: DateTime,
    pub crawl_id: Uuid,
    pub org_id: Uuid,
    pub job_run_id: String,
    pub seed_url: String,
    pub dublin_metadata_date: DateTime,
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