//! Repository module for managing accessions in the digital archive.
//!
//! This module provides functionality for creating, retrieving, and listing
//! accession records with their associated metadata in both Arabic and English.

use crate::models::common::MetadataLanguage;
use crate::models::request::{
    AccessionPaginationWithPrivate, CreateAccessionRequest, UpdateAccessionRequest,
};
use crate::repos::filter_builder::{build_filter_expression, FilterParams};
use async_trait::async_trait;
use chrono::Utc;
use entity::accession::ActiveModel as AccessionActiveModel;
use entity::accession::Entity as Accession;

use entity::accessions_with_metadata;
use entity::accessions_with_metadata::Entity as AccessionWithMetadata;
use entity::accessions_with_metadata::Model as AccessionWithMetadataModel;
use entity::dublin_metadata_ar::ActiveModel as DublinMetadataArActiveModel;
use entity::dublin_metadata_ar::Entity as DublinMetadataAr;
use entity::dublin_metadata_ar_subjects::ActiveModel as DublinMetadataSubjectsArActiveModel;
use entity::dublin_metadata_ar_subjects::Entity as DublinMetadataSubjectsAr;
use entity::dublin_metadata_en::ActiveModel as DublinMetadataEnActiveModel;
use entity::dublin_metadata_en::Entity as DublinMetadataEn;
use entity::dublin_metadata_en_subjects::ActiveModel as DublinMetadataSubjectsEnActiveModel;
use entity::dublin_metadata_en_subjects::Entity as DublinMetadataSubjectsEn;
use entity::dublin_metadata_subject_ar::Entity as DublinMetadataSubjectAr;
use entity::dublin_metadata_subject_en::Entity as DublinMetadataSubjectEn;
use entity::sea_orm_active_enums::CrawlStatus;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, TransactionTrait, TryIntoModel,
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
    ) -> Result<i32, DbErr>;

    /// Retrieves an accession record by its ID along with associated metadata.
    async fn get_one(
        &self,
        id: i32,
        private: bool,
    ) -> Result<Option<AccessionWithMetadataModel>, DbErr>;

    /// Lists accessions with pagination and filtering options.
    ///
    /// # Arguments
    /// * `params` - Parameters for filtering and pagination
    async fn list_paginated(
        &self,
        params: AccessionPaginationWithPrivate,
    ) -> Result<(Vec<AccessionWithMetadataModel>, u64), DbErr>;

    /// Deletes an accession record by its ID.
    ///
    /// # Arguments
    /// * `id` - The ID of the accession to delete
    async fn delete_one(&self, id: i32) -> Result<Option<()>, DbErr>;

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
    ) -> Result<i32, DbErr> {
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
            is_private: ActiveValue::Set(create_accession_request.is_private),
        };
        let saved_accession = accession.clone().save(&txn).await?;
        txn.commit().await?;
        Ok(*saved_accession.id.as_ref())
    }

    async fn get_one(
        &self,
        id: i32,
        private: bool,
    ) -> Result<Option<AccessionWithMetadataModel>, DbErr> {
        let accession = AccessionWithMetadata::find()
            .filter(accessions_with_metadata::Column::Id.eq(id))
            .filter(accessions_with_metadata::Column::IsPrivate.eq(private))
            .one(&self.db_session)
            .await?;
        Ok(accession)
    }

    async fn list_paginated(
        &self,
        params: AccessionPaginationWithPrivate,
    ) -> Result<(Vec<AccessionWithMetadataModel>, u64), DbErr> {
        let filter_params = FilterParams {
            metadata_language: params.lang,
            metadata_subjects: params.metadata_subjects,
            query_term: params.query_term,
            date_from: params.date_from,
            date_to: params.date_to,
            is_private: params.is_private,
        };
        let filter_expression = build_filter_expression(filter_params);
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

    async fn delete_one(&self, id: i32) -> Result<Option<()>, DbErr> {
        let txn = self.db_session.begin().await?;
        let accession = Accession::find_by_id(id).one(&txn).await?;
        Accession::delete_by_id(id).exec(&txn).await?;
        match accession {
            Some(accession_record) => {
                if let Some(metadata_id) = accession_record.dublin_metadata_en {
                    let metadata_en = DublinMetadataEn::find_by_id(metadata_id).one(&txn).await?;
                    if let Some(metadata_record) = metadata_en {
                        DublinMetadataSubjectsEn::delete_many()
                            .filter(<entity::dublin_metadata_en_subjects::Entity as EntityTrait>::Column::MetadataId.eq(metadata_record.id))
                            .exec(&txn)
                            .await?;
                        DublinMetadataSubjectEn::delete_by_id(metadata_record.id)
                            .exec(&txn)
                            .await?;
                        DublinMetadataEn::delete_by_id(metadata_id)
                            .exec(&txn)
                            .await?;
                    }
                }
                if let Some(metadata_id) = accession_record.dublin_metadata_ar {
                    let metadata_ar = DublinMetadataAr::find_by_id(metadata_id).one(&txn).await?;
                    if let Some(metadata_record) = metadata_ar {
                        DublinMetadataSubjectsAr::delete_many().filter(<entity::dublin_metadata_ar_subjects::Entity as EntityTrait>::Column::MetadataId.eq(metadata_record.id))
                            .exec(&txn)
                            .await?;
                        DublinMetadataSubjectAr::delete_by_id(metadata_record.id)
                            .exec(&txn)
                            .await?;
                        DublinMetadataAr::delete_by_id(metadata_id)
                            .exec(&txn)
                            .await?;
                    }
                }
                txn.commit().await?;
                Ok(Some(()))
            }
            None => Ok(None),
        }
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
                match update_accession_request.metadata_language {
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
                        let mut new_subject_links: Vec<DublinMetadataSubjectsEnActiveModel> =
                            vec![];
                        for subject_id in update_accession_request.metadata_subjects.iter() {
                            let subjects_link = DublinMetadataSubjectsEnActiveModel {
                                metadata_id: ActiveValue::Set(metadata_id),
                                subject_id: ActiveValue::Set(*subject_id),
                            };
                            new_subject_links.push(subjects_link);
                        }
                        DublinMetadataSubjectsEn::delete_many().filter(<entity::dublin_metadata_en_subjects::Entity as EntityTrait>::Column::MetadataId.eq(metadata_id))
                            .exec(&txn)
                            .await?;
                        DublinMetadataSubjectsEn::insert_many(new_subject_links)
                            .exec(&txn)
                            .await?;
                        accession_active.dublin_metadata_en = ActiveValue::Set(Some(metadata_id));
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
                        let mut new_subject_links: Vec<DublinMetadataSubjectsArActiveModel> =
                            vec![];
                        for subject_id in update_accession_request.metadata_subjects.iter() {
                            let subjects_link = DublinMetadataSubjectsArActiveModel {
                                metadata_id: ActiveValue::Set(metadata_id),
                                subject_id: ActiveValue::Set(*subject_id),
                            };
                            new_subject_links.push(subjects_link);
                        }
                        DublinMetadataSubjectsAr::delete_many().filter(<entity::dublin_metadata_ar_subjects::Entity as EntityTrait>::Column::MetadataId.eq(metadata_id))
                            .exec(&txn)
                            .await?;
                        DublinMetadataSubjectsAr::insert_many(new_subject_links)
                            .exec(&txn)
                            .await?;
                        accession_active.dublin_metadata_ar = ActiveValue::Set(Some(metadata_id));
                    }
                };
                accession_active.dublin_metadata_date =
                    ActiveValue::Set(update_accession_request.metadata_time);
                accession_active.is_private = ActiveValue::Set(update_accession_request.is_private);
                accession_active.update(&txn).await?;
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
