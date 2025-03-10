use sea_orm::DbErr;

use crate::{models::request::LoginRequest, repos::auth_repo::AuthRepo};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthService {
    pub auth_repo: Arc<dyn AuthRepo>,
}

impl AuthService {
    pub async fn log_user_in(&self, login_request: LoginRequest) -> Result<Option<Uuid>, DbErr> {
        let user_id = self
            .auth_repo
            .get_user_by_email(login_request.email)
            .await?;
        match user_id {
            Some(user_id) => {
                let session_id = self.auth_repo.create_session(user_id).await?;
                Ok(Some(session_id))
            }
            None => Ok(None),
        }
    }
}
