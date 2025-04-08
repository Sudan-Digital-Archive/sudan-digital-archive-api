use sea_orm::DbErr;

use crate::models::request::LoginRequest;
use crate::repos::{auth_repo::AuthRepo, emails_repo::EmailsRepo};
use std::sync::Arc;
use tracing::{error,info};
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthService {
    pub auth_repo: Arc<dyn AuthRepo>,
    pub emails_repo: Arc<dyn EmailsRepo>,
}

impl AuthService {
    pub async fn log_user_in(self, login_request: LoginRequest) -> Result<Option<Uuid>, DbErr> {
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

    pub async fn delete_expired_sessions(self) {
        self.auth_repo.delete_expired_sessions().await
    }
    pub async fn send_login_email(self, token: Uuid, user_email: String) {
        let result = self
            .emails_repo
            .send_email(format!("Your magic token is {}", token))
            .await;
        match result {
            Ok(_) => info!("Magic link email sent successfully for user {}", user_email),
            Err(err) => error!(%err, "Couldn't send email to user {}", user_email),
        }
    }
}
