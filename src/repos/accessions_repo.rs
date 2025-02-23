//! Repository module for managing accessions in the digital archive.
//!
//! This module provides functionality for creating, retrieving, and listing
//! accession records with their associated metadata in both Arabic and English.

use crate::models::common::MetadataLanguage;
use crate::models::request::CreateAccessionRequest;
use ::entity::accession::ActiveModel as AccessionActiveModel;
use ::entity::accession::Entity as Accession;
use ::entity::dublin_metadata_ar::ActiveModel as DublinMetadataArActiveModel;
use ::entity::dublin_metadata_ar::Entity as DublinMetadataAr;
use ::entity::dublin_metadata_ar_subjects::ActiveModel as DublinMetadataSubjectsArActiveModel;
use ::entity::dublin_metadata_ar_subjects::Entity as DublinMetadataSubjectsAr;
use ::entity::dublin_metadata_en::ActiveModel as DublinMetadataEnActiveModel;
use ::entity::dublin_metadata_en::Entity as DublinMetadataEn;
use ::entity::dublin_metadata_en_subjects::ActiveModel as DublinMetadataSubjectsEnActiveModel;
use ::entity::dublin_metadata_en_subjects::Entity as DublinMetadataSubjectsEn;

use crate::repos::filter_builder::build_filter_expression;
use ::entity::accession::Model as AccessionModel;
use ::entity::accessions_with_metadata::Entity as AccessionWithMetadata;
use ::entity::accessions_with_metadata::Model as AccessionWithMetadataModel;
use ::entity::dublin_metadata_ar::Model as DublinMetataArModel;
use ::entity::dublin_metadata_en::Model as DublinMetadataEnModel;
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use entity::sea_orm_active_enums::CrawlStatus;
use sea_orm::{
    ActiveModelTrait, ActiveValue, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, TransactionTrait, TryIntoModel,
};
use uuid::Uuid;
/// Repository implementation for database operations on accessions.
#[derive(Debug, Clone, Default)]
pub struct DBAccessionsRepo {
    pub db_session: DatabaseConnection,
}

/// Defines the interface for accession-related database operations.
///
/// This trait provides methods for creating and retrieving accession records
/// along with their associated metadata in both Arabic and English.
#[async_trait]
pub trait AccessionsRepo: Send + Sync {
    /// Creates a new accession record with associated metadata.
    ///
    /// # Arguments
    /// * `create_accession_request` - The request containing accession and metadata details
    /// * `org_id` - The Browsertrix organization ID associated with the accession
    /// * `crawl_id` - The ID of the crawl operation
    /// * `job_run_id` - The ID of the job run
    /// * `crawl_status` - The status of the crawl operation
    async fn write_one(
        &self,
        create_accession_request: CreateAccessionRequest,
        org_id: Uuid,
        crawl_id: Uuid,
        job_run_id: String,
        crawl_status: CrawlStatus,
    ) -> Result<(), DbErr>;

    /// Retrieves an accession record by its ID along with associated metadata.
    ///
    /// Returns a tuple containing the accession and its metadata in both languages.
    // async fn get_one(
    //     &self,
    //     id: i32,
    // ) -> Result<
    //     (
    //         Option<AccessionModel>,
    //         Option<DublinMetataArModel>,
    //         Option<DublinMetadataEnModel>,
    //     ),
    //     DbErr,
    // >;
    async fn get_one(&self, id: i32) -> Result<Option<AccessionWithMetadataModel>, DbErr>;

    /// Lists paginated accessions with Arabic metadata.
    ///
    /// # Arguments
    /// * `page` - The page number to retrieve
    /// * `per_page` - Number of items per page
    /// * `query_term` - Optional search term for filtering results
    /// * `date_from` - Optional start date for filtering
    /// * `date_to` - Optional end date for filtering
    async fn list_paginated_ar(
        &self,
        page: u64,
        per_page: u64,
        query_term: Option<String>,
        date_from: Option<NaiveDateTime>,
        date_to: Option<NaiveDateTime>,
    ) -> Result<(Vec<(AccessionModel, Option<DublinMetataArModel>)>, u64), DbErr>;

    /// Lists paginated accessions with English metadata.
    ///
    /// # Arguments
    /// * `page` - The page number to retrieve
    /// * `per_page` - Number of items per page
    /// * `query_term` - Optional search term for filtering results
    /// * `date_from` - Optional start date for filtering
    /// * `date_to` - Optional end date for filtering
    async fn list_paginated_en(
        &self,
        page: u64,
        per_page: u64,
        query_term: Option<String>,
        date_from: Option<NaiveDateTime>,
        date_to: Option<NaiveDateTime>,
    ) -> Result<(Vec<(AccessionModel, Option<DublinMetadataEnModel>)>, u64), DbErr>;
}

#[async_trait]
impl AccessionsRepo for DBAccessionsRepo {
    async fn write_one(
        &self,
        create_accession_request: CreateAccessionRequest,
        org_id: Uuid,
        crawl_id: Uuid,
        job_run_id: String,
        crawl_status: CrawlStatus,
    ) -> Result<(), DbErr> {
        let txn = self.db_session.begin().await?;
        let (dublin_metadata_en_id, dublin_metadata_ar_id) = match create_accession_request
            .metadata_language
        {
            MetadataLanguage::English => {
                let metadata = DublinMetadataEnActiveModel {
                    id: Default::default(),
                    title: ActiveValue::Set(create_accession_request.metadata_title),
                    description: ActiveValue::Set(create_accession_request.metadata_description),
                };
                let inserted_metadata = metadata.save(&txn).await?;
                let metadata_id = inserted_metadata.try_into_model()?.id;
                let mut subject_links: Vec<DublinMetadataSubjectsEnActiveModel> = vec![];
                for subject_id in create_accession_request.metadata_subjects.iter() {
                    let subjects_link = DublinMetadataSubjectsEnActiveModel {
                        metadata_id: ActiveValue::Set(metadata_id),
                        subject_id: ActiveValue::Set(*subject_id),
                    };
                    subject_links.push(subjects_link);
                }
                DublinMetadataSubjectsEn::insert_many(subject_links)
                    .exec(&txn)
                    .await?;
                metadata_id
            }
            MetadataLanguage::Arabic => {
                let metadata = DublinMetadataArActiveModel {
                    id: Default::default(),
                    title: ActiveValue::Set(create_accession_request.metadata_title),
                    description: ActiveValue::Set(create_accession_request.metadata_description),
                };
                let inserted_metadata = metadata.save(&txn).await?;
                let metadata_id = inserted_metadata.try_into_model()?.id;
                let mut subject_links: Vec<DublinMetadataSubjectsArActiveModel> = vec![];
                for subject_id in create_accession_request.metadata_subjects.iter() {
                    let subjects_link = DublinMetadataSubjectsArActiveModel {
                        metadata_id: ActiveValue::Set(metadata_id),
                        subject_id: ActiveValue::Set(*subject_id),
                    };
                    subject_links.push(subjects_link);
                }
                DublinMetadataSubjectsAr::insert_many(subject_links)
                    .exec(&txn)
                    .await?;
                metadata_id }
        };

        let utc_now = Utc::now();
        let i_hate_timezones = utc_now.naive_utc();
        let accession = AccessionActiveModel {
            id: Default::default(),
            dublin_metadata_en: ActiveValue::Set(dublin_metadata_en_id),
            dublin_metadata_ar: ActiveValue::Set(dublin_metadata_ar_id),
            dublin_metadata_date: ActiveValue::Set(create_accession_request.metadata_time),
            crawl_status: ActiveValue::Set(crawl_status),
            crawl_timestamp: ActiveValue::Set(i_hate_timezones),
            org_id: ActiveValue::Set(org_id),
            crawl_id: ActiveValue::Set(crawl_id),
            job_run_id: ActiveValue::Set(job_run_id),
            seed_url: ActiveValue::Set(create_accession_request.url),
        };
        accession.save(&txn).await?;
        txn.commit().await?;
        Ok(())
    }

    // async fn get_one(
    //     &self,
    //     id: i32,
    // ) -> Result<
    //     (
    //         Option<AccessionModel>,
    //         Option<DublinMetataArModel>,
    //         Option<DublinMetadataEnModel>,
    //     ),
    //     DbErr,
    // > {
    //     let accession = Accession::find_by_id(id).one(&self.db_session).await?;
    //     if let Some(accession) = accession {
    //         let metadata_en = accession
    //             .find_related(DublinMetadataEn)
    //             .one(&self.db_session)
    //             .await?;
    //         let metadata_ar = accession
    //             .find_related(DublinMetadataAr)
    //             .one(&self.db_session)
    //             .await?;
    //         Ok((Some(accession), metadata_ar, metadata_en))
    //     } else {
    //         Ok((None, None, None))
    //     }
    // }

    async fn get_one(&self, id: i32) -> Result<Option<AccessionWithMetadataModel>, DbErr> {
        let accession = AccessionWithMetadata::find_by_id(id)
            .one(&self.db_session)
            .await?;
        Ok(accession)
    }

    async fn list_paginated_ar(
        &self,
        page: u64,
        per_page: u64,
        query_term: Option<String>,
        date_from: Option<NaiveDateTime>,
        date_to: Option<NaiveDateTime>,
    ) -> Result<(Vec<(AccessionModel, Option<DublinMetataArModel>)>, u64), DbErr> {
        let filter_expression =
            build_filter_expression(MetadataLanguage::Arabic, query_term, date_from, date_to);
        let accession_pages;
        if let Some(query_filter) = filter_expression {
            accession_pages = Accession::find()
                .find_also_related(DublinMetadataAr)
                .filter(query_filter)
                .paginate(&self.db_session, per_page);
        } else {
            accession_pages = Accession::find()
                .find_also_related(DublinMetadataAr)
                .paginate(&self.db_session, per_page);
        }
        let num_pages = accession_pages.num_pages().await?;
        Ok((accession_pages.fetch_page(page).await?, num_pages))
    }

    async fn list_paginated_en(
        &self,
        page: u64,
        per_page: u64,
        query_term: Option<String>,
        date_from: Option<NaiveDateTime>,
        date_to: Option<NaiveDateTime>,
    ) -> Result<(Vec<(AccessionModel, Option<DublinMetadataEnModel>)>, u64), DbErr> {
        let filter_expression =
            build_filter_expression(MetadataLanguage::English, query_term, date_from, date_to);
        let accession_pages;
        if let Some(query_filter) = filter_expression {
            accession_pages = Accession::find()
                .find_also_related(DublinMetadataEn)
                .filter(query_filter)
                .paginate(&self.db_session, per_page);
        } else {
            accession_pages = Accession::find()
                .find_also_related(DublinMetadataEn)
                .paginate(&self.db_session, per_page);
        }
        let num_pages = accession_pages.num_pages().await?;
        Ok((accession_pages.fetch_page(page).await?, num_pages))
    }
}
