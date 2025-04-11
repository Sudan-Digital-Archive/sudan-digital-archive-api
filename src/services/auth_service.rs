use sea_orm::DbErr;

use crate::auth::JWT_KEYS;
use crate::models::auth::JWTClaims;
use crate::models::request::{AuthorizeRequest, LoginRequest};
use crate::repos::{auth_repo::AuthRepo, emails_repo::EmailsRepo};
use chrono::NaiveDateTime;
use jsonwebtoken::errors::Error;
use jsonwebtoken::{encode, Header};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthService {
    pub auth_repo: Arc<dyn AuthRepo>,
    pub emails_repo: Arc<dyn EmailsRepo>,
    pub jwt_cookie_domain: String,
}

impl AuthService {
    pub async fn log_user_in(
        self,
        login_request: LoginRequest,
    ) -> Result<Option<(Uuid, Uuid)>, DbErr> {
        let user_id = self
            .auth_repo
            .get_user_by_email(login_request.email)
            .await?;
        match user_id {
            Some(user_id) => {
                let session_id = self.auth_repo.create_session(user_id).await?;
                Ok(Some((session_id, user_id)))
            }
            None => Ok(None),
        }
    }

    pub async fn delete_expired_sessions(self) {
        self.auth_repo.delete_expired_sessions().await
    }

    pub async fn send_login_email(self, session_id: Uuid, user_id: Uuid, user_email: String) {
        // TODO: Delete me
        info!(%session_id, %user_id, "session id, user id");
        let result = self
            .emails_repo
            .send_email(format!(
                "Your magic token is {} for user {}",
                session_id, user_id
            ))
            .await;
        match result {
            Ok(_) => info!("Magic link email sent successfully for user {}", user_email),
            Err(err) => error!(%err, "Couldn't send email to user {}", user_email),
        }
    }

    pub async fn get_session_expiry(
        self,
        authorize_request: AuthorizeRequest,
    ) -> Result<Option<NaiveDateTime>, DbErr> {
        self.auth_repo.get_session_expiry(authorize_request).await
    }

    pub fn build_auth_cookie_string(
        self,
        user_id: Uuid,
        expiry_time: NaiveDateTime,
    ) -> Result<String, Error> {
        let claims = JWTClaims {
            sub: user_id.to_string(),
            exp: expiry_time.and_utc().timestamp() as usize,
        };
        let jwt = encode(&Header::default(), &claims, &JWT_KEYS.encoding)?;
        let max_age = expiry_time.and_utc().timestamp().to_string();
        // TODO: Find a smarter way of doing this lol
        // Uncomment for local dev
        //let cookie_string = format!("jwt={}; Max-Age={}", jwt, max_age);
        let cookie_string = format!(
            "jwt={}; HttpOnly; Secure; Domain={}; Max-Age={}; SameSite=Strict",
            jwt, self.jwt_cookie_domain, max_age
        );
        Ok(cookie_string)
    }
}
