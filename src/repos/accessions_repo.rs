//! Repository module for managing accessions in the digital archive.
//!
//! This module provides functionality for creating, retrieving, and listing
//! accession records with their associated metadata in both Arabic and English.

use crate::models::common::MetadataLanguage;
use crate::models::request::{AccessionPagination, CreateAccessionRequest, UpdateAccessionRequest};
use ::entity::accession::ActiveModel as AccessionActiveModel;
use ::entity::accession::Entity as Accession;
use ::entity::dublin_metadata_ar::ActiveModel as DublinMetadataArActiveModel;
use ::entity::dublin_metadata_ar_subjects::ActiveModel as DublinMetadataSubjectsArActiveModel;
use ::entity::dublin_metadata_ar_subjects::Entity as DublinMetadataSubjectsAr;
use ::entity::dublin_metadata_en::ActiveModel as DublinMetadataEnActiveModel;
use ::entity::dublin_metadata_en_subjects::ActiveModel as DublinMetadataSubjectsEnActiveModel;
use ::entity::dublin_metadata_en_subjects::Entity as DublinMetadataSubjectsEn;

use crate::repos::filter_builder::build_filter_expression;
use ::entity::accessions_with_metadata::Entity as AccessionWithMetadata;
use ::entity::accessions_with_metadata::Model as AccessionWithMetadataModel;
use async_trait::async_trait;
use chrono::Utc;
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
    async fn get_one(&self, id: i32) -> Result<Option<AccessionWithMetadataModel>, DbErr>;

    /// Lists accessions with pagination and filtering options.
    ///
    /// # Arguments
    /// * `params` - Parameters for filtering and pagination
    async fn list_paginated(
        &self,
        params: AccessionPagination,
    ) -> Result<(Vec<AccessionWithMetadataModel>, u64), DbErr>;

    /// Deletes an accession record by its ID.
    ///
    /// # Arguments
    /// * `id` - The ID of the accession to delete
    async fn delete_one(&self, id: i32) -> Result<u64, DbErr>;

    /// Updates an existing accession record with new metadata.
    ///
    /// # Arguments
    /// * `id` - The ID of the accession to update
    /// * `update_accession_request` - The request containing updated metadata details
    async fn update_one(
        &self,
        id: i32,
        update_accession_request: UpdateAccessionRequest,
    ) -> Result<Option<AccessionWithMetadataModel>, DbErr>;
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
                (Some(metadata_id), None)
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
                (None, Some(metadata_id))
            }
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
            is_private: ActiveValue::Set(false),
        };
        accession.save(&txn).await?;
        txn.commit().await?;
        Ok(())
    }

    async fn get_one(&self, id: i32) -> Result<Option<AccessionWithMetadataModel>, DbErr> {
        let accession = AccessionWithMetadata::find_by_id(id)
            .one(&self.db_session)
            .await?;
        Ok(accession)
    }

    async fn list_paginated(
        &self,
        params: AccessionPagination,
    ) -> Result<(Vec<AccessionWithMetadataModel>, u64), DbErr> {
        let filter_expression = build_filter_expression(
            params.lang,
            params.metadata_subjects,
            params.query_term,
            params.date_from,
            params.date_to,
        );
        let accession_pages;
        if let Some(query_filter) = filter_expression {
            accession_pages = AccessionWithMetadata::find()
                .filter(query_filter)
                .paginate(&self.db_session, params.per_page);
        } else {
            accession_pages =
                AccessionWithMetadata::find().paginate(&self.db_session, params.per_page);
        }
        let num_pages = accession_pages.num_pages().await?;
        Ok((accession_pages.fetch_page(params.page).await?, num_pages))
    }

    async fn delete_one(&self, id: i32) -> Result<u64, DbErr> {
        let delete_result = Accession::delete_by_id(id).exec(&self.db_session).await?;
        Ok(delete_result.rows_affected)
    }

    async fn update_one(
        &self,
        id: i32,
        update_accession_request: UpdateAccessionRequest,
    ) -> Result<Option<AccessionWithMetadataModel>, DbErr> {
        let txn = self.db_session.begin().await?;
        let accession = Accession::find_by_id(id).one(&self.db_session).await?;

        match accession {
            Some(accession) => {
                let mut accession_active: AccessionActiveModel = accession.clone().into();
                let (dublin_metadata_en_id, dublin_metadata_ar_id) = match update_accession_request
                    .metadata_language
                {
                    MetadataLanguage::English => {
                        let metadata = DublinMetadataEnActiveModel {
                            id: match accession.dublin_metadata_en {
                                Some(id) => ActiveValue::Set(id),
                                None => Default::default(),
                            },
                            title: ActiveValue::Set(update_accession_request.metadata_title),
                            description: ActiveValue::Set(
                                update_accession_request.metadata_description,
                            ),
                        };
                        let inserted_metadata = metadata.save(&txn).await?;
                        let metadata_id = inserted_metadata.try_into_model()?.id;
                        let mut subject_links: Vec<DublinMetadataSubjectsEnActiveModel> = vec![];
                        for subject_id in update_accession_request.metadata_subjects.iter() {
                            let subjects_link = DublinMetadataSubjectsEnActiveModel {
                                metadata_id: ActiveValue::Set(metadata_id),
                                subject_id: ActiveValue::Set(*subject_id),
                            };
                            subject_links.push(subjects_link);
                        }
                        DublinMetadataSubjectsEn::insert_many(subject_links)
                            .exec(&txn)
                            .await?;
                        (Some(metadata_id), None)
                    }
                    MetadataLanguage::Arabic => {
                        let metadata = DublinMetadataArActiveModel {
                            id: match accession.dublin_metadata_ar {
                                Some(id) => ActiveValue::Set(id),
                                None => Default::default(),
                            },
                            title: ActiveValue::Set(update_accession_request.metadata_title),
                            description: ActiveValue::Set(
                                update_accession_request.metadata_description,
                            ),
                        };
                        let inserted_metadata = metadata.save(&txn).await?;
                        let metadata_id = inserted_metadata.try_into_model()?.id;
                        let mut subject_links: Vec<DublinMetadataSubjectsArActiveModel> = vec![];
                        for subject_id in update_accession_request.metadata_subjects.iter() {
                            let subjects_link = DublinMetadataSubjectsArActiveModel {
                                metadata_id: ActiveValue::Set(metadata_id),
                                subject_id: ActiveValue::Set(*subject_id),
                            };
                            subject_links.push(subjects_link);
                        }
                        DublinMetadataSubjectsAr::insert_many(subject_links)
                            .exec(&txn)
                            .await?;
                        (None, Some(metadata_id))
                    }
                };

                accession_active.dublin_metadata_en = ActiveValue::Set(dublin_metadata_en_id);
                accession_active.dublin_metadata_ar = ActiveValue::Set(dublin_metadata_ar_id);
                accession_active.dublin_metadata_date =
                    ActiveValue::Set(update_accession_request.metadata_time);

                accession_active.update(&self.db_session).await?;
                txn.commit().await?;
                let accession = AccessionWithMetadata::find_by_id(id)
                    .one(&self.db_session)
                    .await?;
                Ok(accession)
            }
            None => Ok(None),
        }
    }
}
