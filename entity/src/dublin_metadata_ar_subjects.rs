//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.0

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Deserialize, Serialize)]
#[sea_orm(table_name = "dublin_metadata_ar_subjects")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub metadata_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub subject_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::dublin_metadata_ar::Entity",
        from = "Column::MetadataId",
        to = "super::dublin_metadata_ar::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    DublinMetadataAr,
    #[sea_orm(
        belongs_to = "super::dublin_metadata_subject_ar::Entity",
        from = "Column::SubjectId",
        to = "super::dublin_metadata_subject_ar::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    DublinMetadataSubjectAr,
}

impl Related<super::dublin_metadata_ar::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DublinMetadataAr.def()
    }
}

impl Related<super::dublin_metadata_subject_ar::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DublinMetadataSubjectAr.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
