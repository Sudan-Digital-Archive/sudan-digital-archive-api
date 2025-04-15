use crate::auth::JWT_KEYS;
use crate::models::auth::JWTClaims;
use crate::models::request::{AuthorizeRequest, LoginRequest};
use crate::repos::{auth_repo::AuthRepo, emails_repo::EmailsRepo};
use ::entity::archive_user::Model as ArchiveUserModel;
use ::entity::sea_orm_active_enums::Role;
use axum::http::{
    header::{HeaderMap, HeaderValue, SET_COOKIE},
    StatusCode,
};
use axum::response::{IntoResponse, Response};
use chrono::NaiveDateTime;
use jsonwebtoken::errors::Error;
use jsonwebtoken::{encode, Header};
use sea_orm::DbErr;
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
        role: Role,
        expiry_time: NaiveDateTime,
    ) -> Result<String, Error> {
        let claims = JWTClaims {
            sub: user_id.to_string(),
            exp: expiry_time.and_utc().timestamp() as usize,
            role,
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

    pub async fn get_user(&self, user_id: Uuid) -> Result<Option<ArchiveUserModel>, DbErr> {
        self.auth_repo.get_one(user_id).await
    }

    pub async fn authorize(&self, payload: AuthorizeRequest) -> Result<Response, String> {
        let session_expiry_time_result = self
            .clone()
            .get_session_expiry(payload.clone())
            .await
            .map_err(|err| format!("Failed to get session expiry: {}", err))?;

        match session_expiry_time_result {
            Some(sesh_exists) => {
                let user_result = self
                    .get_user(payload.user_id)
                    .await
                    .map_err(|err| format!("Failed to get user: {}", err))?;

                match user_result {
                    Some(user) => {
                        let cookie_string_result = self
                            .clone()
                            .build_auth_cookie_string(payload.user_id, user.role, sesh_exists)
                            .map_err(|err| format!("Failed to build cookie string: {}", err))?;

                        let mut headers = HeaderMap::new();
                        let header_value_result = HeaderValue::from_str(&cookie_string_result)
                            .map_err(|err| format!("Failed to create cookie header: {}", err))?;

                        headers.insert(SET_COOKIE, header_value_result);
                        Ok((StatusCode::OK, headers, "Authentication successful").into_response())
                    }
                    None => {
                        let message = "User not found".to_string();
                        info!(message);
                        Ok((StatusCode::NOT_FOUND, message).into_response())
                    }
                }
            }
            None => {
                let message = "Session does not exist for user".to_string();
                info!(message);
                Ok((StatusCode::NOT_FOUND, message).into_response())
            }
        }
    }

    pub async fn login(self, payload: LoginRequest) -> Result<Response, String> {
        let login_result = self
            .clone()
            .log_user_in(payload.clone())
            .await
            .map_err(|err| format!("Database error: {}", err))?;

        match login_result {
            Some((session_id, user_id)) => {
                info!(
                    "Sending login email to user with email {} and deleting expired sessions",
                    payload.email
                );
                let email = payload.email.clone();
                tokio::spawn(async move {
                    self.clone().delete_expired_sessions().await;
                    self.clone()
                        .send_login_email(session_id, user_id, email)
                        .await;
                });
                Ok((StatusCode::OK, "Login email sent").into_response())
            }
            None => {
                let message = format!("User with email {} not found", payload.email);
                info!(message);
                Ok((StatusCode::NOT_FOUND, message).into_response())
            }
        }
    }
}
