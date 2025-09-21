use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            DROP VIEW accessions_with_metadata;
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
                setweight(to_tsvector('english', coalesce(dme.title, '')), 'A') ||
                setweight(to_tsvector('english', coalesce(dme.description, '')), 'B')
                AS full_text_en,
                setweight(to_tsvector('arabic', coalesce(dma.title, '')), 'A') ||
                setweight(to_tsvector('arabic', coalesce(dma.description, '')), 'B')
                AS full_text_ar
            FROM accession a
            LEFT JOIN dublin_metadata_en dme ON a.dublin_metadata_en = dme.id
            LEFT JOIN dublin_metadata_ar dma ON a.dublin_metadata_ar = dma.id
            "#,
        )
        .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gin_full_text_en")
                    .table(AccessionsWithMetadata::Table)
                    .col(AccessionsWithMetadata::FullTextEn)
                    .full_text()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gin_full_text_ar")
                    .table(AccessionsWithMetadata::Table)
                    .col(AccessionsWithMetadata::FullTextAr)
                    .full_text()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_gin_full_text_en")
                    .table(AccessionsWithMetadata::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_gin_full_text_ar")
                    .table(AccessionsWithMetadata::Table)
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            DROP VIEW accessions_with_metadata;
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
pub enum AccessionsWithMetadata {
    #[sea_orm(iden = "accessions_with_metadata")]
    Table,
    #[sea_orm(iden = "full_text_en")]
    FullTextEn,
    #[sea_orm(iden = "full_text_ar")]
    FullTextAr,
}
