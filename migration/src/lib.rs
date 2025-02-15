
pub use sea_orm_migration::prelude::*;
mod m20241224_163000_accessions;
mod m20250212_014525_optional_metadata_description;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241224_163000_accessions::Migration),
            Box::new(m20250212_014525_optional_metadata_description::Migration),
        ]
    }
}
