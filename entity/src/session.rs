//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "session")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub expiry_time: DateTime,
    pub user_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::archive_user::Entity",
        from = "Column::UserId",
        to = "super::archive_user::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    ArchiveUser,
}

impl Related<super::archive_user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ArchiveUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
