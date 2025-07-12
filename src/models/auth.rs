use crate::auth::JWT_KEYS;
use ::entity::sea_orm_active_enums::Role;
use axum::response::{IntoResponse, Response};
use axum::{
    extract::FromRequestParts, http::request::Parts, http::StatusCode, Json, RequestPartsExt,
};
use axum_extra::extract::CookieJar;
use jsonwebtoken::errors::ErrorKind::ExpiredSignature;
use jsonwebtoken::{decode, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;
#[derive(Debug)]
pub enum AuthError {
    InvalidToken,
    TokenExpired,
}
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token expired"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct JWTClaims {
    pub sub: String,
    pub exp: usize,
    pub role: Role,
}
impl fmt::Display for JWTClaims {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Sub: {}\nExp: {}\nRole: {:?}",
            self.sub, self.exp, self.role
        )
    }
}

impl<S> FromRequestParts<S> for JWTClaims
where
    S: Send + Sync,
{
    type Rejection = AuthError;
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let cookie_jar = parts
            .extract::<CookieJar>()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        let token = cookie_jar
            .get("jwt")
            .map(|cookie| cookie.value().to_string())
            .ok_or(AuthError::InvalidToken)?;

        let mut validation = Validation::default();
        validation.validate_exp = true;

        let token_data =
            decode::<JWTClaims>(&token, &JWT_KEYS.decoding, &validation).map_err(|e| {
                match e.kind() {
                    ExpiredSignature => AuthError::TokenExpired,
                    _ => AuthError::InvalidToken,
                }
            })?;

        let claims = token_data.claims;
        Ok(claims)
    }
}
