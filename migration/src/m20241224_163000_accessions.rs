use crate::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;
#[derive(DeriveIden)]
enum Accession {
    Table,
    Id,
    DublinMetadataEn,
    DublinMetadataAr,
    CrawlStatus,
    CrawlTimestamp,
    DublinMetadataDate,
    CrawlId,
    OrgId,
    JobRunId,
    SeedURL,
}

#[derive(DeriveIden)]
pub(crate) enum DublinMetadataEn {
    Table,
    Id,
    Title,
    Subject,
    Description,
}

#[derive(DeriveIden)]
pub(crate) enum DublinMetadataAr {
    Table,
    Id,
    Title,
    Subject,
    Description,
}

#[derive(DeriveIden)]
enum CrawlStatus {
    #[sea_orm(iden = "crawl_status")]
    Enum,
    #[sea_orm(iden = "pending")]
    Pending,
    #[sea_orm(iden = "complete")]
    Complete,
    #[sea_orm(iden = "error")]
    Error,
    #[sea_orm(iden = "bad_crawl")]
    BadCrawl,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // hard to find docs info on enums - see
        // https://www.sea-ql.org/SeaORM/docs/generate-entity/enumeration/
        manager
            .create_type(
                Type::create()
                    .as_enum(CrawlStatus::Enum)
                    .values([
                        CrawlStatus::BadCrawl,
                        CrawlStatus::Complete,
                        CrawlStatus::Error,
                        CrawlStatus::Pending,
                    ])
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(DublinMetadataEn::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DublinMetadataEn::Id)
                            .integer()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(DublinMetadataEn::Title).string().not_null())
                    .col(
                        ColumnDef::new(DublinMetadataEn::Subject)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DublinMetadataEn::Description)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(DublinMetadataAr::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DublinMetadataAr::Id)
                            .integer()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(DublinMetadataAr::Title).string().not_null())
                    .col(
                        ColumnDef::new(DublinMetadataAr::Subject)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DublinMetadataAr::Description)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Accession::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Accession::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Accession::DublinMetadataEn).integer().null())
                    .col(ColumnDef::new(Accession::DublinMetadataAr).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("dublin_metadata_en")
                            .from(Accession::Table, Accession::DublinMetadataEn)
                            .to(DublinMetadataEn::Table, DublinMetadataEn::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("dublin_metadata_ar")
                            .from(Accession::Table, Accession::DublinMetadataAr)
                            .to(DublinMetadataAr::Table, DublinMetadataAr::Id),
                    )
                    .col(
                        ColumnDef::new(Accession::CrawlStatus)
                            .custom(CrawlStatus::Enum)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Accession::CrawlTimestamp)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Accession::DublinMetadataDate)
                            .timestamp()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Accession::CrawlId).uuid().not_null())
                    .col(ColumnDef::new(Accession::OrgId).uuid().not_null())
                    .col(ColumnDef::new(Accession::JobRunId).string().not_null())
                    .col(ColumnDef::new(Accession::SeedURL).string().not_null())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Accession::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(DublinMetadataEn::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(DublinMetadataAr::Table).to_owned())
            .await?;
        manager
            .drop_type(Type::drop().name(CrawlStatus::Enum).to_owned())
            .await?;
        Ok(())
    }
}
