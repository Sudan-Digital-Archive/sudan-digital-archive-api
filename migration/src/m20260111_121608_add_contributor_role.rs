use crate::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[allow(dead_code)]
#[derive(DeriveIden)]
enum Role {
    #[sea_orm(iden = "role")]
    Enum,
    #[sea_orm(iden = "admin")]
    Admin,
    #[sea_orm(iden = "researcher")]
    Researcher,
    #[sea_orm(iden = "contributor")]
    Contributor,
}
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_type(
                Type::alter()
                    .name(Role::Enum)
                    .add_value(Role::Contributor)
                    .if_not_exists()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // noop
        Ok(())
    }
}
