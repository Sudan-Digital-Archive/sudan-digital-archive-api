use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;


#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(crate::m20241224_163000_accessions::DublinMetadataEn::Table)
                    .modify_column( ColumnDef::new(crate::m20241224_163000_accessions::DublinMetadataEn::Description)
                                        .string()
                                        .null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(crate::m20241224_163000_accessions::DublinMetadataAr::Table)
                    .modify_column( ColumnDef::new(crate::m20241224_163000_accessions::DublinMetadataAr::Description)
                        .string()
                        .null())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(crate::m20241224_163000_accessions::DublinMetadataEn::Table)
                    .modify_column( ColumnDef::new(crate::m20241224_163000_accessions::DublinMetadataEn::Description)
                        .string()
                        .not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(crate::m20241224_163000_accessions::DublinMetadataAr::Table)
                    .modify_column( ColumnDef::new(crate::m20241224_163000_accessions::DublinMetadataAr::Description)
                        .string()
                        .not_null())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
