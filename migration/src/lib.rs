pub use sea_orm_migration::prelude::*;
mod m20241224_163000_accessions;
mod m20250212_014525_optional_metadata_description;
mod m20250217_012314_subjects_more_like_tags;
mod m20250310_013018_add_auth;
mod m20250712_072835_add_researcher_role;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241224_163000_accessions::Migration),
            Box::new(m20250212_014525_optional_metadata_description::Migration),
            Box::new(m20250217_012314_subjects_more_like_tags::Migration),
            Box::new(m20250310_013018_add_auth::Migration),
            Box::new(m20250712_072835_add_researcher_role::Migration),
        ]
    }
}
