use crate::models::request::AuthorizeRequest;
use ::entity::api_key::ActiveModel as ApiKeyActiveModel;
use ::entity::api_key::Entity as ApiKey;
use ::entity::archive_user::Entity as ArchiveUser;
use ::entity::archive_user::Model as ArchiveUserModel;
use ::entity::sea_orm_active_enums::Role;
use ::entity::session::ActiveModel as SessionActiveModel;
use ::entity::session::Entity as Session;
use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use chrono::{Duration, NaiveDateTime, Utc};
use entity::{api_key, archive_user, session};
use rand::Rng;
use sea_orm::{ActiveModelTrait, ActiveValue};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{error, info};
use uuid::Uuid;

/// Response containing user email and role from API key verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyUserInfo {
    pub email: String,
    pub role: Role,
}
#[derive(Debug, Clone, Default)]
pub struct DBAuthRepo {
    pub db_session: DatabaseConnection,
    pub expiry_hours: i64,
}

#[async_trait]
pub trait AuthRepo: Send + Sync {
    async fn get_user_by_email(&self, email: String) -> Result<Option<Uuid>, DbErr>;
    async fn create_session(&self, user_id: Uuid) -> Result<Uuid, DbErr>;
    async fn delete_expired_sessions(&self);
    async fn get_session_expiry(
        &self,
        authorize_request: AuthorizeRequest,
    ) -> Result<Option<NaiveDateTime>, DbErr>;
    async fn get_one(&self, user_id: Uuid) -> Result<Option<ArchiveUserModel>, DbErr>;
    async fn create_api_key_for_user(&self, user_id: Uuid) -> Result<String, DbErr>;
    async fn verify_api_key(&self, api_key: String) -> Result<Option<ApiKeyUserInfo>, DbErr>;
    async fn delete_expired_api_keys(&self);
}

#[async_trait]
impl AuthRepo for DBAuthRepo {
    async fn get_user_by_email(&self, email: String) -> Result<Option<Uuid>, DbErr> {
        let user = ArchiveUser::find()
            .filter(archive_user::Column::Email.eq(email))
            .filter(archive_user::Column::IsActive.eq(true))
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
    async fn get_session_expiry(
        &self,
        authorize_request: AuthorizeRequest,
    ) -> Result<Option<NaiveDateTime>, DbErr> {
        let session = Session::find()
            .filter(session::Column::UserId.eq(authorize_request.user_id))
            .filter(session::Column::Id.eq(authorize_request.session_id))
            .filter(session::Column::ExpiryTime.gt(Utc::now().naive_utc()))
            .one(&self.db_session)
            .await?;
        match session {
            Some(found_session) => Ok(Some(found_session.expiry_time)),
            None => Ok(None),
        }
    }

    async fn get_one(&self, user_id: Uuid) -> Result<Option<ArchiveUserModel>, DbErr> {
        let user = ArchiveUser::find()
            .filter(archive_user::Column::Id.eq(user_id))
            .filter(archive_user::Column::IsActive.eq(true))
            .one(&self.db_session)
            .await?;
        match user {
            Some(user) => Ok(Some(user)),
            None => Ok(None),
        }
    }

    async fn create_api_key_for_user(&self, user_id: Uuid) -> Result<String, DbErr> {
        // Generate a cryptographically secure random 32-byte secret
        let mut secret_bytes = [0u8; 32];
        {
            let mut rng = rand::thread_rng();
            rng.fill(&mut secret_bytes);
        }

        // Hash the secret using SHA256
        let mut hasher = Sha256::new();
        hasher.update(&secret_bytes);
        let key_hash = hasher.finalize();
        let key_hash_hex = format!("{:x}", key_hash);

        // Create the API key record in the database
        let api_key_id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + Duration::days(90);

        let api_key = ApiKeyActiveModel {
            id: ActiveValue::Set(api_key_id),
            user_id: ActiveValue::Set(user_id),
            key_hash: ActiveValue::Set(key_hash_hex),
            created_at: ActiveValue::Set(now.naive_utc()),
            expires_at: ActiveValue::Set(expires_at.naive_utc()),
            is_revoked: ActiveValue::Set(false),
        };

        api_key.insert(&self.db_session).await?;

        // URL-safe encode the secret and return it to the user
        let encoded_secret = URL_SAFE.encode(&secret_bytes);
        Ok(encoded_secret)
    }

    async fn verify_api_key(&self, api_key: String) -> Result<Option<ApiKeyUserInfo>, DbErr> {
        // Decode the URL-safe encoded API key back to bytes
        let secret_bytes = match URL_SAFE.decode(&api_key) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(None),
        };

        // Hash the decoded secret using SHA256
        let mut hasher = Sha256::new();
        hasher.update(&secret_bytes);
        let key_hash = hasher.finalize();
        let key_hash_hex = format!("{:x}", key_hash);

        // Look up the key hash in the database
        let api_key_record = ApiKey::find()
            .filter(api_key::Column::KeyHash.eq(key_hash_hex))
            .filter(api_key::Column::IsRevoked.eq(false))
            .filter(api_key::Column::ExpiresAt.gt(Utc::now().naive_utc()))
            .one(&self.db_session)
            .await?;

        match api_key_record {
            Some(key_record) => {
                // Get the user associated with this API key, including their role
                let user = ArchiveUser::find()
                    .filter(archive_user::Column::Id.eq(key_record.user_id))
                    .filter(archive_user::Column::IsActive.eq(true))
                    .one(&self.db_session)
                    .await?;

                match user {
                    Some(user) => Ok(Some(ApiKeyUserInfo {
                        email: user.email,
                        role: user.role,
                    })),
                    None => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    async fn delete_expired_api_keys(&self) {
        let now = Utc::now().naive_utc();
        let delete_result = ApiKey::delete_many()
            .filter(api_key::Column::ExpiresAt.lte(now))
            .exec(&self.db_session)
            .await;

        match delete_result {
            Ok(_) => {
                info!("Successfully deleted expired API keys.");
            }
            Err(err) => {
                error!(%err, "Error deleting expired API keys");
            }
        }
    }
}
