use sea_orm::DbErr;

use crate::{models::request::LoginRequest, repos::auth_repo::AuthRepo};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthService {
    pub auth_repo: Arc<dyn AuthRepo>,
}

impl AuthService {
    pub async fn log_user_in(&self, login_request: LoginRequest) -> Result<Uuid, DbErr> {
        let user = self
            .auth_repo
            .get_user_by_email(login_request.email)
            .await?;
        todo!()
    }
}
