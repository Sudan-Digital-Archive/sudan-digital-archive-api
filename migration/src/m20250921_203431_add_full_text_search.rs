use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Create IMMUTABLE functions to be used in generated columns
        db.execute_unprepared(
            r#"
            CREATE OR REPLACE FUNCTION get_dublin_metadata_en_text(metadata_id INT)
            RETURNS TEXT AS $$
            BEGIN
                RETURN (SELECT title || ' ' || description FROM dublin_metadata_en WHERE id = metadata_id);
            END;
            $$ LANGUAGE plpgsql IMMUTABLE;

            CREATE OR REPLACE FUNCTION get_dublin_metadata_ar_text(metadata_id INT)
            RETURNS TEXT AS $$
            BEGIN
                RETURN (SELECT title || ' ' || description FROM dublin_metadata_ar WHERE id = metadata_id);
            END;
            $$ LANGUAGE plpgsql IMMUTABLE;
            "#,
        )
        .await?;

        // First add the full-text search columns to the accession table
        db.execute_unprepared(
            r#"
            ALTER TABLE accession
            ADD COLUMN full_text_en tsvector GENERATED ALWAYS AS (
                to_tsvector('english', get_dublin_metadata_en_text(dublin_metadata_en))
            ) STORED,
            ADD COLUMN full_text_ar tsvector GENERATED ALWAYS AS (
                to_tsvector('arabic', get_dublin_metadata_ar_text(dublin_metadata_ar))
            ) STORED;
            "#,
        )
        .await?;

        // Create the GIN indexes on the accession table
        manager
            .create_index(
                Index::create()
                    .name("idx_gin_accession_full_text_en")
                    .table(Accession::Table)
                    .col(Accession::FullTextEn)
                    .full_text()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gin_accession_full_text_ar")
                    .table(Accession::Table)
                    .col(Accession::FullTextAr)
                    .full_text()
                    .to_owned(),
            )
            .await?;

        // Update the view to use the full-text fields from the accession table
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

        // Drop the indexes first
        manager
            .drop_index(Index::drop().name("idx_gin_accession_full_text_en").to_owned())
            .await?;
        
        manager
            .drop_index(Index::drop().name("idx_gin_accession_full_text_ar").to_owned())
            .await?;

        // Drop the full-text columns from accession table
        db.execute_unprepared(
            r#"
            ALTER TABLE accession
            DROP COLUMN full_text_en,
            DROP COLUMN full_text_ar;
            "#,
        )
        .await?;

        // Drop the functions
        db.execute_unprepared(
            r#"
            DROP FUNCTION IF EXISTS get_dublin_metadata_en_text(INT);
            DROP FUNCTION IF EXISTS get_dublin_metadata_ar_text(INT);
            "#,
        )
        .await?;

        // Recreate the view without the full-text columns
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
                COALESCE((dma.id IS NOT NULL), FALSE) AS has_arabic_metadata
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
pub enum Accession {
    #[sea_orm(iden = "accession")]
    Table,
    #[sea_orm(iden = "full_text_en")]
    FullTextEn,
    #[sea_orm(iden = "full_text_ar")]
    FullTextAr,
}
