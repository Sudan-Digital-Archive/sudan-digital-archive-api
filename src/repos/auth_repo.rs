use ::entity::session::ActiveModel as SessionActiveModel;
use ::entity::session::Entity as Session;
use ::entity::user::Entity as User;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use entity::{session, user};
use sea_orm::{ActiveModelTrait, ActiveValue};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct DBAuthRepo {
    pub db_session: DatabaseConnection,
    pub expiry_hours: i64,
}

#[async_trait]
pub trait AuthRepo: Send + Sync {
    async fn get_user_by_email(&self, email: String) -> Result<Option<Uuid>, DbErr>;
    async fn create_session(&self, user_id: Uuid) -> Result<Uuid, DbErr>;
    async fn send_magic_link_email(&self, session_id: Uuid) -> Result<(), DbErr>;
    async fn delete_expired_sessions(&self);
}

#[async_trait]
impl AuthRepo for DBAuthRepo {
    async fn get_user_by_email(&self, email: String) -> Result<Option<Uuid>, DbErr> {
        let user = User::find()
            .filter(user::Column::Email.eq(email))
            .one(&self.db_session)
            .await?;
        match user {
            Some(user) => Ok(Some(user.id)),
            None => Ok(None),
        }
    }
    async fn create_session(&self, user_id: Uuid) -> Result<Uuid, DbErr> {
        let session_id = Uuid::new_v4();
        let now = Utc::now();
        let expiry_time = now + Duration::hours(self.expiry_hours);
        let session = SessionActiveModel {
            id: ActiveValue::Set(session_id),
            expiry_time: ActiveValue::Set(expiry_time.naive_utc()),
            user_id: ActiveValue::Set(user_id),
        };
        let session = session.insert(&self.db_session).await?;
        Ok(session.id)
    }
    async fn delete_expired_sessions(&self) {
        let now = Utc::now().naive_utc();
        let delete_result = Session::delete_many()
            .filter(session::Column::ExpiryTime.lte(now))
            .exec(&self.db_session)
            .await;
        match delete_result {
            Ok(_) => {
                info!("Successfully deleted expired sessions.");
            }
            Err(err) => {
                error!(%err, "Error deleting expired sessions");
            }
        }
    }
}
