use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Redefine the functions as STABLE and add COALESCE for null safety.
        // The functions were previously defined as IMMUTABLE, which is incorrect because they read from tables.
        // IMMUTABLE functions' results should not change with database state, but these do.
        // STABLE is the correct volatility for functions that read from the database but do not modify it.
        // This ensures that the generated columns are updated correctly when the underlying metadata changes.
        db.execute_unprepared(
            r#"
            CREATE OR REPLACE FUNCTION get_dublin_metadata_en_text(metadata_id INT)
            RETURNS TEXT AS $$
            BEGIN
                RETURN (
                    SELECT COALESCE(title, '') || ' ' || COALESCE(description, '')
                    FROM dublin_metadata_en
                    WHERE id = metadata_id
                );
            END;
            $$ LANGUAGE plpgsql STABLE;

            CREATE OR REPLACE FUNCTION get_dublin_metadata_ar_text(metadata_id INT)
            RETURNS TEXT AS $$
            BEGIN
                RETURN (
                    SELECT COALESCE(title, '') || ' ' || COALESCE(description, '')
                    FROM dublin_metadata_ar
                    WHERE id = metadata_id
                );
            END;
            $$ LANGUAGE plpgsql STABLE;
            "#,
        )
        .await?;

        // Rebuilding the index will re-compute the stored values in the generated columns
        // using the updated (and correctly defined) functions.
        db.execute_unprepared(
            r#"
            REINDEX INDEX idx_gin_accession_full_text_en;
            REINDEX INDEX idx_gin_accession_full_text_ar;
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Revert the functions to their previous IMMUTABLE state.
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

        // Rebuilding the index will re-compute the stored values in the generated columns
        // using the reverted functions.
        db.execute_unprepared(
            r#"
            REINDEX INDEX idx_gin_accession_full_text_en;
            REINDEX INDEX idx_gin_accession_full_text_ar;
            "#,
        )
        .await?;

        Ok(())
    }
}
