//! Repository module for managing subject metadata in the digital archive.
//!
//! This module provides functionality for creating and listing subject terms
//! that can be used to categorize archived content in both Arabic and English.

use crate::models::common::MetadataLanguage;
use crate::models::request::CreateSubjectRequest;
use crate::models::response::SubjectResponse;
use ::entity::dublin_metadata_subject_ar::ActiveModel as DublinMetadataSubjectArActiveModel;
use ::entity::dublin_metadata_subject_ar::Entity as DublinMetadataSubjectAr;
use ::entity::dublin_metadata_subject_ar::Model as DublinMetadataSubjectArModel;
use ::entity::dublin_metadata_subject_en::ActiveModel as DublinMetadataSubjectEnActiveModel;
use ::entity::dublin_metadata_subject_en::Entity as DublinMetadataSubjectEn;
use ::entity::dublin_metadata_subject_en::Model as DublinMetadataSubjectEnModel;
use async_trait::async_trait;
use entity::{dublin_metadata_subject_ar, dublin_metadata_subject_en};
use sea_orm::prelude::Expr;
use sea_orm::sea_query::{ExprTrait, Func};
use sea_orm::{
    ActiveModelTrait, ActiveValue, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
};
use sea_orm::{ColumnTrait, QueryFilter};

/// Repository implementation for database operations on subjects.
#[derive(Debug, Clone, Default)]
pub struct DBSubjectsRepo {
    pub db_session: DatabaseConnection,
}

/// Defines the interface for subject-related database operations.
///
/// This trait provides methods for creating and retrieving subject terms
/// that can be used to categorize archived content in both Arabic and English.
#[async_trait]
pub trait SubjectsRepo: Send + Sync {
    /// Creates a new subject term in the specified language.
    ///
    /// # Arguments
    /// * `create_subject_request` - The request containing subject details and language
    async fn write_one(
        &self,
        create_subject_request: CreateSubjectRequest,
    ) -> Result<SubjectResponse, DbErr>;

    /// Lists Arabic subject terms with pagination and optional text search.
    ///
    /// # Arguments
    /// * `page` - The page number to retrieve
    /// * `per_page` - Number of records per page
    /// * `query_term` - Optional text search term
    async fn list_paginated_ar(
        &self,
        page: u64,
        per_page: u64,
        query_term: Option<String>,
    ) -> Result<(Vec<DublinMetadataSubjectArModel>, u64), DbErr>;

    /// Lists English subject terms with pagination and optional text search.
    ///
    /// # Arguments
    /// * `page` - The page number to retrieve
    /// * `per_page` - Number of records per page
    /// * `query_term` - Optional text search term
    async fn list_paginated_en(
        &self,
        page: u64,
        per_page: u64,
        query_term: Option<String>,
    ) -> Result<(Vec<DublinMetadataSubjectEnModel>, u64), DbErr>;

    /// Verifies that all provided subject IDs exist in the database.
    ///
    /// # Arguments
    /// * `subject_ids` - List of subject IDs to verify
    /// * `metadata_language` - Language of the subjects to check
    async fn verify_subjects_exist(
        &self,
        subject_ids: Vec<i32>,
        metadata_language: MetadataLanguage,
    ) -> Result<bool, DbErr>;

    /// Deletes a subject term by its ID.
    ///
    /// # Arguments
    /// * `subject_id` - The ID of the subject to delete.
    /// * `metadata_language` - Language of the subject to delete
    async fn delete_one(
        &self,
        subject_id: i32,
        metadata_language: MetadataLanguage,
    ) -> Result<bool, DbErr>;
}

#[async_trait]
impl SubjectsRepo for DBSubjectsRepo {
    async fn write_one(
        &self,
        create_subject_request: CreateSubjectRequest,
    ) -> Result<SubjectResponse, DbErr> {
        let resp = match create_subject_request.lang {
            MetadataLanguage::English => {
                let subject = DublinMetadataSubjectEnActiveModel {
                    id: Default::default(),
                    subject: ActiveValue::Set(create_subject_request.metadata_subject),
                };
                let new_subject = subject.insert(&self.db_session).await?;
                SubjectResponse {
                    id: new_subject.id,
                    subject: new_subject.subject,
                }
            }
            MetadataLanguage::Arabic => {
                let subject = DublinMetadataSubjectArActiveModel {
                    id: Default::default(),
                    subject: ActiveValue::Set(create_subject_request.metadata_subject),
                };
                let new_subject = subject.insert(&self.db_session).await?;
                SubjectResponse {
                    id: new_subject.id,
                    subject: new_subject.subject,
                }
            }
        };
        Ok(resp)
    }

    async fn list_paginated_ar(
        &self,
        page: u64,
        per_page: u64,
        query_term: Option<String>,
    ) -> Result<(Vec<DublinMetadataSubjectArModel>, u64), DbErr> {
        let subject_pages;
        if let Some(term) = query_term {
            let query_string = format!("%{}%", term.to_lowercase());
            let query_filter = Func::lower(Expr::col(dublin_metadata_subject_ar::Column::Subject))
                .like(&query_string);
            subject_pages = DublinMetadataSubjectAr::find()
                .filter(query_filter)
                .paginate(&self.db_session, per_page);
        } else {
            subject_pages = DublinMetadataSubjectAr::find().paginate(&self.db_session, per_page);
        }
        let num_pages = subject_pages.num_pages().await?;
        Ok((subject_pages.fetch_page(page).await?, num_pages))
    }

    async fn list_paginated_en(
        &self,
        page: u64,
        per_page: u64,
        query_term: Option<String>,
    ) -> Result<(Vec<DublinMetadataSubjectEnModel>, u64), DbErr> {
        let subject_pages;
        if let Some(term) = query_term {
            let query_string = format!("%{}%", term.to_lowercase());
            let query_filter = Func::lower(Expr::col(dublin_metadata_subject_en::Column::Subject))
                .like(&query_string);
            subject_pages = DublinMetadataSubjectEn::find()
                .filter(query_filter)
                .paginate(&self.db_session, per_page);
        } else {
            subject_pages = DublinMetadataSubjectEn::find().paginate(&self.db_session, per_page);
        }
        let num_pages = subject_pages.num_pages().await?;
        Ok((subject_pages.fetch_page(page).await?, num_pages))
    }

    async fn verify_subjects_exist(
        &self,
        subject_ids: Vec<i32>,
        metadata_language: MetadataLanguage,
    ) -> Result<bool, DbErr> {
        let flag = match metadata_language {
            MetadataLanguage::English => {
                let rows = DublinMetadataSubjectEn::find()
                    .filter(dublin_metadata_subject_en::Column::Id.is_in(subject_ids.clone()))
                    .all(&self.db_session)
                    .await?;
                rows.len() == subject_ids.len()
            }
            MetadataLanguage::Arabic => {
                let rows = DublinMetadataSubjectAr::find()
                    .filter(dublin_metadata_subject_ar::Column::Id.is_in(subject_ids.clone()))
                    .all(&self.db_session)
                    .await?;
                rows.len() == subject_ids.len()
            }
        };
        Ok(flag)
    }

    async fn delete_one(
        &self,
        subject_id: i32,
        metadata_language: MetadataLanguage,
    ) -> Result<bool, DbErr> {
        let deletion = match metadata_language {
            MetadataLanguage::English => {
                DublinMetadataSubjectEn::delete_by_id(subject_id)
                    .exec(&self.db_session)
                    .await?;
                true
            }
            MetadataLanguage::Arabic => {
                DublinMetadataSubjectAr::delete_by_id(subject_id)
                    .exec(&self.db_session)
                    .await?;
                true
            }
        };
        Ok(deletion)
    }
}
