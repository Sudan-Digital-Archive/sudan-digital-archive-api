use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DbErr};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct DBAuthRepo {
    pub db_session: DatabaseConnection,
}

#[async_trait]
pub trait AuthRepo: Send + Sync {
    async fn get_user_by_email(&self, email: String) -> Result<Uuid, DbErr>;
}

#[async_trait]
impl AuthRepo for DBAuthRepo {
    async fn get_user_by_email(&self, email: String) -> Result<Uuid, DbErr> {
        todo!()
    }
}
