use crate::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum FileType {
    #[sea_orm(iden = "file_type")]
    Enum,
    #[sea_orm(iden = "wacz")]
    Wacz,
}

#[derive(DeriveIden)]
enum Accession {
    Table,
    FileType,
    S3Filename,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        manager
            .create_type(
                Type::create()
                    .as_enum(FileType::Enum)
                    .values([FileType::Wacz])
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Accession::Table)
                    .add_column(
                        ColumnDef::new(Accession::FileType)
                            .custom(FileType::Enum)
                            .not_null()
                            .default("wacz"),
                    )
                    .add_column(ColumnDef::new(Accession::S3Filename).string().null())
                    .to_owned(),
            )
            .await?;

        db.execute_unprepared(
            r#"
            DROP VIEW IF EXISTS accessions_with_metadata;
            CREATE VIEW accessions_with_metadata AS
            SELECT
                a.id,
                a.is_private,
                a.crawl_status,
                a.crawl_timestamp,
                a.crawl_id,
                a.org_id,
                a.job_run_id,
                a.seed_url,
                a.dublin_metadata_date,
                a.file_type,
                a.s3_filename,
                dme.title AS title_en,
                dme.description AS description_en,
                dma.title AS title_ar,
                dma.description AS description_ar,
                (
                    SELECT array_agg(dmse.subject)
                    FROM dublin_metadata_subject_en dmse
                    LEFT JOIN dublin_metadata_en_subjects dmes ON dmse.id = dmes.subject_id
                    LEFT JOIN dublin_metadata_en dme ON dme.id = dmes.metadata_id
                    WHERE dme.id = a.dublin_metadata_en
                    -- api validation limits 200 max subjects
                    LIMIT 200
                ) AS subjects_en,
                (
                    SELECT array_agg(dmse.id)
                    FROM dublin_metadata_subject_en dmse
                    LEFT JOIN dublin_metadata_en_subjects dmes ON dmse.id = dmes.subject_id
                    LEFT JOIN dublin_metadata_en dme ON dme.id = dmes.metadata_id
                    WHERE dme.id = a.dublin_metadata_en
                    -- api validation limits 200 max subjects
                    LIMIT 200
                ) AS subjects_en_ids,
                (
                    SELECT array_agg(dmsa.subject)
                    FROM dublin_metadata_subject_ar dmsa
                    LEFT JOIN dublin_metadata_ar_subjects dmas ON dmsa.id = dmas.subject_id
                    LEFT JOIN dublin_metadata_ar dma ON dma.id = dmas.metadata_id
                    WHERE dma.id = a.dublin_metadata_ar
                    -- api validation limits 200 max subjects
                    LIMIT 200
                ) AS subjects_ar,
                (
                    SELECT array_agg(dmsa.id)
                    FROM dublin_metadata_subject_ar dmsa
                    LEFT JOIN dublin_metadata_ar_subjects dmas ON dmsa.id = dmas.subject_id
                    LEFT JOIN dublin_metadata_ar dma ON dma.id = dmas.metadata_id
                    WHERE dma.id = a.dublin_metadata_ar
                    -- api validation limits 200 max subjects
                    LIMIT 200
                ) AS subjects_ar_ids,
                COALESCE((dme.id IS NOT NULL), FALSE) AS has_english_metadata,
                COALESCE((dma.id IS NOT NULL), FALSE) AS has_arabic_metadata,
                a.full_text_en,
                a.full_text_ar
            FROM accession a
            LEFT JOIN dublin_metadata_en dme ON a.dublin_metadata_en = dme.id
            LEFT JOIN dublin_metadata_ar dma ON a.dublin_metadata_ar = dma.id
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("DROP VIEW IF EXISTS accessions_with_metadata;")
            .await?;

        db.execute_unprepared(
            r#"
            CREATE VIEW accessions_with_metadata AS
            SELECT
                a.id,
                a.is_private,
                a.crawl_status,
                a.crawl_timestamp,
                a.crawl_id,
                a.org_id,
                a.job_run_id,
                a.seed_url,
                a.dublin_metadata_date,
                dme.title AS title_en,
                dme.description AS description_en,
                dma.title AS title_ar,
                dma.description AS description_ar,
                (
                    SELECT array_agg(dmse.subject)
                    FROM dublin_metadata_subject_en dmse
                    LEFT JOIN dublin_metadata_en_subjects dmes ON dmse.id = dmes.subject_id
                    LEFT JOIN dublin_metadata_en dme ON dme.id = dmes.metadata_id
                    WHERE dme.id = a.dublin_metadata_en
                    -- api validation limits 200 max subjects
                    LIMIT 200
                ) AS subjects_en,
                (
                    SELECT array_agg(dmse.id)
                    FROM dublin_metadata_subject_en dmse
                    LEFT JOIN dublin_metadata_en_subjects dmes ON dmse.id = dmes.subject_id
                    LEFT JOIN dublin_metadata_en dme ON dme.id = dmes.metadata_id
                    WHERE dme.id = a.dublin_metadata_en
                    -- api validation limits 200 max subjects
                    LIMIT 200
                ) AS subjects_en_ids,
                (
                    SELECT array_agg(dmsa.subject)
                    FROM dublin_metadata_subject_ar dmsa
                    LEFT JOIN dublin_metadata_ar_subjects dmas ON dmsa.id = dmas.subject_id
                    LEFT JOIN dublin_metadata_ar dma ON dma.id = dmas.metadata_id
                    WHERE dma.id = a.dublin_metadata_ar
                    -- api validation limits 200 max subjects
                    LIMIT 200
                ) AS subjects_ar,
                (
                    SELECT array_agg(dmsa.id)
                    FROM dublin_metadata_subject_ar dmsa
                    LEFT JOIN dublin_metadata_ar_subjects dmas ON dmsa.id = dmas.subject_id
                    LEFT JOIN dublin_metadata_ar dma ON dma.id = dmas.metadata_id
                    WHERE dma.id = a.dublin_metadata_ar
                    -- api validation limits 200 max subjects
                    LIMIT 200
                ) AS subjects_ar_ids,
                COALESCE((dme.id IS NOT NULL), FALSE) AS has_english_metadata,
                COALESCE((dma.id IS NOT NULL), FALSE) AS has_arabic_metadata,
                a.full_text_en,
                a.full_text_ar
            FROM accession a
            LEFT JOIN dublin_metadata_en dme ON a.dublin_metadata_en = dme.id
            LEFT JOIN dublin_metadata_ar dma ON a.dublin_metadata_ar = dma.id
            "#,
        )
        .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Accession::Table)
                    .drop_column(Accession::FileType)
                    .drop_column(Accession::S3Filename)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_type(Type::drop().name(FileType::Enum).to_owned())
            .await?;

        Ok(())
    }
}
