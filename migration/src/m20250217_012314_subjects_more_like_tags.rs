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
    Id,
    Subject,
}

#[derive(DeriveIden)]
enum DublinMetadataAr {
    Table,
    Id,
    Subject,
}

#[derive(DeriveIden)]
enum DublinMetadataEnSubjects {
    Table,
    MetadataId,
    SubjectId,
}

#[derive(DeriveIden)]
enum DublinMetadataArSubjects {
    Table,
    MetadataId,
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
                    .col(string(DublinMetadataSubjectEn::Subject).unique_key())
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(DublinMetadataSubjectAr::Table)
                    .if_not_exists()
                    .col(pk_auto(DublinMetadataSubjectAr::Id))
                    .col(string(DublinMetadataSubjectAr::Subject).unique_key())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DublinMetadataEn::Table)
                    .drop_column(DublinMetadataEn::Subject)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DublinMetadataAr::Table)
                    .drop_column(DublinMetadataAr::Subject)
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(DublinMetadataEnSubjects::Table)
                    .if_not_exists()
                    .primary_key(
                        Index::create()
                            .name("link_subjects_en")
                            .col(DublinMetadataEnSubjects::MetadataId)
                            .col(DublinMetadataEnSubjects::SubjectId),
                    )
                    .col(
                        ColumnDef::new(DublinMetadataEnSubjects::MetadataId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DublinMetadataEnSubjects::SubjectId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("dublin_metadata_id_en")
                            .from(
                                DublinMetadataEnSubjects::Table,
                                DublinMetadataEnSubjects::MetadataId,
                            )
                            .to(DublinMetadataEn::Table, DublinMetadataEn::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("dublin_metadata_subject_en")
                            .from(
                                DublinMetadataEnSubjects::Table,
                                DublinMetadataEnSubjects::SubjectId,
                            )
                            .to(DublinMetadataSubjectEn::Table, DublinMetadataSubjectEn::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(DublinMetadataArSubjects::Table)
                    .if_not_exists()
                    .primary_key(
                        Index::create()
                            .name("link_subjects_ar")
                            .col(DublinMetadataArSubjects::MetadataId)
                            .col(DublinMetadataArSubjects::SubjectId),
                    )
                    .col(
                        ColumnDef::new(DublinMetadataArSubjects::MetadataId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DublinMetadataArSubjects::SubjectId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("dublin_metadata_id_ar")
                            .from(
                                DublinMetadataArSubjects::Table,
                                DublinMetadataArSubjects::MetadataId,
                            )
                            .to(DublinMetadataAr::Table, DublinMetadataAr::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("dublin_metadata_subject_ar")
                            .from(
                                DublinMetadataArSubjects::Table,
                                DublinMetadataArSubjects::SubjectId,
                            )
                            .to(DublinMetadataSubjectAr::Table, DublinMetadataSubjectAr::Id),
                    )
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
                JOIN dublin_metadata_en_subjects dmes ON dmse.id = dmes.subject_id
                WHERE dmes.metadata_id = dme.id
                -- api validation limits 200 max subjects
                LIMIT 200
                ) AS subjects_en,
                dma.title AS title_ar,
                dma.description AS description_ar,
                (
                    SELECT array_agg(dmsa.subject)
                    FROM dublin_metadata_subject_ar dmsa
                    JOIN dublin_metadata_ar_subjects dmas ON dmsa.id = dmas.subject_id
                    WHERE dmas.metadata_id = dma.id
                    -- api validation limits 200 max subjects
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
                    .add_column_if_not_exists(
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
                    .add_column_if_not_exists(
                        ColumnDef::new(DublinMetadataAr::Subject)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(DublinMetadataEnSubjects::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(DublinMetadataArSubjects::Table)
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
        Ok(())
    }
}
