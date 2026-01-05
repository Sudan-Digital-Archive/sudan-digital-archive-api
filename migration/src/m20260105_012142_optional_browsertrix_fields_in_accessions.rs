use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Drop view before altering table
        db.execute_unprepared("DROP VIEW IF EXISTS accessions_with_metadata;")
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Accession::Table)
                    .modify_column(ColumnDef::new(Accession::CrawlId).uuid().null())
                    .modify_column(ColumnDef::new(Accession::OrgId).uuid().null())
                    .modify_column(ColumnDef::new(Accession::JobRunId).string().null())
                    .to_owned(),
            )
            .await?;

        // Recreate view
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
                a.dublin_metadata_format,
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

        // Drop view before reverting table changes
        db.execute_unprepared("DROP VIEW IF EXISTS accessions_with_metadata;")
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Accession::Table)
                    .modify_column(ColumnDef::new(Accession::CrawlId).uuid().not_null())
                    .modify_column(ColumnDef::new(Accession::OrgId).uuid().not_null())
                    .modify_column(ColumnDef::new(Accession::JobRunId).string().not_null())
                    .to_owned(),
            )
            .await?;

        // Recreate view
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
                a.dublin_metadata_format,
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
}

#[derive(DeriveIden)]
enum Accession {
    Table,
    CrawlId,
    OrgId,
    JobRunId,
}
