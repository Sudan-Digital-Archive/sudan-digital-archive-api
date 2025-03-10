//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.0

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Deserialize, Serialize)]
#[sea_orm(table_name = "dublin_metadata_subject_ar")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub subject: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::dublin_metadata_ar_subjects::Entity")]
    DublinMetadataArSubjects,
}

impl Related<super::dublin_metadata_ar_subjects::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DublinMetadataArSubjects.def()
    }
}

impl Related<super::dublin_metadata_ar::Entity> for Entity {
    fn to() -> RelationDef {
        super::dublin_metadata_ar_subjects::Relation::DublinMetadataAr.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::dublin_metadata_ar_subjects::Relation::DublinMetadataSubjectAr
                .def()
                .rev(),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}
