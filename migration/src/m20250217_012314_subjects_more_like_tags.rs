use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum DublinMetadataSubjectEn {
    Table,
    Id,
    Subject,
}

#[derive(DeriveIden)]
enum DublinMetadataSubjectAr {
    Table,
    Id,
    Subject,
}

#[derive(DeriveIden)]
enum DublinMetadataEn {
    Table,
    Subject,
    SubjectId,
}

#[derive(DeriveIden)]
enum DublinMetadataAr {
    Table,
    Subject,
    SubjectId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DublinMetadataSubjectEn::Table)
                    .if_not_exists()
                    .col(pk_auto(DublinMetadataSubjectEn::Id))
                    .col(string(DublinMetadataSubjectEn::Subject))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(DublinMetadataSubjectAr::Table)
                    .if_not_exists()
                    .col(pk_auto(DublinMetadataSubjectAr::Id))
                    .col(string(DublinMetadataSubjectAr::Subject))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DublinMetadataEn::Table)
                    .drop_column(DublinMetadataEn::Subject)
                    .add_column_if_not_exists(
                        ColumnDef::new(DublinMetadataEn::SubjectId).integer().null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DublinMetadataAr::Table)
                    .drop_column(DublinMetadataAr::Subject)
                    .add_column_if_not_exists(
                        ColumnDef::new(DublinMetadataAr::SubjectId).integer().null(),
                    )
                    .to_owned(),
            )
            .await?;
        let foreign_key_en_subject = TableForeignKey::new()
            .name("dublin_metadata_subject_en")
            .from_tbl(DublinMetadataEn::Table)
            .from_col(DublinMetadataEn::SubjectId)
            .to_tbl(DublinMetadataSubjectEn::Table)
            .to_col(DublinMetadataSubjectEn::Id)
            .to_owned();
        let foreign_key_ar_subject = TableForeignKey::new()
            .name("dublin_metadata_subject_ar")
            .from_tbl(DublinMetadataAr::Table)
            .from_col(DublinMetadataAr::SubjectId)
            .to_tbl(DublinMetadataSubjectAr::Table)
            .to_col(DublinMetadataSubjectAr::Id)
            .to_owned();
        manager
            .alter_table(
                Table::alter()
                    .table(DublinMetadataEn::Table)
                    .add_foreign_key(&foreign_key_en_subject)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DublinMetadataAr::Table)
                    .add_foreign_key(&foreign_key_ar_subject)
                    .to_owned(),
            )
            .await?;
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            CREATE VIEW accessions_with_metadata AS
            SELECT
                a.id,
                a.crawl_status,
                a.crawl_timestamp,
                a.crawl_id,
                a.org_id,
                a.job_run_id,
                a.seed_url,
                a.dublin_metadata_date,
                dme.title AS title_en,
                dme.description AS description_en,
                (
                    SELECT array_agg(dmse.subject)
                    FROM dublin_metadata_subject_en dmse
                    WHERE dme.id = dmse.id
                    LIMIT 200
                ) AS subjects_en,
                dma.title AS title_ar,
                dma.description AS description_ar,
                (
                    SELECT array_agg(dmsa.subject)
                    FROM dublin_metadata_subject_ar dmsa
                    WHERE dma.id = dmsa.id
                    LIMIT 200
                ) AS subjects_ar,
                COALESCE((dme.id IS NOT NULL), FALSE) AS has_english_metadata,
                COALESCE((dma.id IS NOT NULL), FALSE) AS has_arabic_metadata
            FROM accession a
            LEFT JOIN dublin_metadata_en dme ON a.id = dme.id
            LEFT JOIN dublin_metadata_ar dma ON a.id = dma.id
        "#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP VIEW accessions_with_metadata")
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DublinMetadataEn::Table)
                    .drop_foreign_key(Alias::new("dublin_metadata_subject_en"))
                    .drop_column(DublinMetadataEn::SubjectId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(DublinMetadataAr::Table)
                    .drop_foreign_key(Alias::new("dublin_metadata_subject_ar"))
                    .drop_column(DublinMetadataAr::SubjectId)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(DublinMetadataSubjectEn::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(DublinMetadataSubjectAr::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(DublinMetadataEn::Table)
                    .add_column(
                        ColumnDef::new(DublinMetadataEn::Subject)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(DublinMetadataAr::Table)
                    .add_column(
                        ColumnDef::new(DublinMetadataAr::Subject)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
