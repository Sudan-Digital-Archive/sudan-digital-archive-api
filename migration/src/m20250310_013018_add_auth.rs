use crate::extension::postgres::Type;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Accession {
    Table,
    IsPrivate,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Email,
    IsActive,
    Role,
}

#[derive(DeriveIden)]
enum Role {
    #[sea_orm(iden = "role")]
    Enum,
    #[sea_orm(iden = "admin")]
    Admin,
}

#[derive(DeriveIden)]
enum Session {
    Table,
    Id,
    ExpiryTime,
    UserId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(Role::Enum)
                    .values([Role::Admin])
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(pk_auto(User::Id))
                    .col(ColumnDef::new(User::Email).string().not_null().unique_key())
                    .col(
                        ColumnDef::new(User::IsActive)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(User::Role).custom(Role::Enum).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Session::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Session::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Session::ExpiryTime).timestamp().not_null())
                    .col(ColumnDef::new(Session::UserId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_session_user")
                            .from(Session::Table, Session::UserId)
                            .to(User::Table, User::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Accession::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(Accession::IsPrivate)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(Role::Enum).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Session::Table).to_owned())
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Accession::Table)
                    .drop_column(Accession::IsPrivate)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
