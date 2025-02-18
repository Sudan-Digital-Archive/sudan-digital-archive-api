use crate::models::request::CreateSubjectRequest;

use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DbErr};

#[derive(Debug, Clone, Default)]
pub struct DBSubjectsRepo {
    pub db_session: DatabaseConnection,
}

#[async_trait]
pub trait SubjectsRepo: Send + Sync {

    async fn write_one(
        &self,
        create_subject_request: CreateSubjectRequest,
    ) -> Result<(), DbErr>;


    async fn list(
        &self,
        query: String,
    ) -> Result<(Vec<String>), DbErr>;

}

#[async_trait]
impl SubjectsRepo for DBSubjectsRepo {
    async fn write_one(&self, create_subject_request: CreateSubjectRequest) -> Result<(), DbErr> {
        todo!()
    }

    async fn list(&self, query: String) -> Result<(Vec<String>), DbErr> {
        todo!()
    }
}
