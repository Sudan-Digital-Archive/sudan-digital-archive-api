use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum ApiKey {
    Table,
    Id,
    UserId,
    KeyHash,
    CreatedAt,
    ExpiresAt,
    IsRevoked,
}

#[derive(DeriveIden)]
enum ArchiveUser {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKey::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ApiKey::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(ApiKey::UserId).uuid().not_null())
                    .col(ColumnDef::new(ApiKey::KeyHash).string().not_null())
                    .col(
                        ColumnDef::new(ApiKey::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(ApiKey::ExpiresAt).timestamp().not_null())
                    .col(
                        ColumnDef::new(ApiKey::IsRevoked)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_api_key_user_id")
                            .from(ApiKey::Table, ApiKey::UserId)
                            .to(ArchiveUser::Table, ArchiveUser::Id),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiKey::Table).to_owned())
            .await?;

        Ok(())
    }
}
