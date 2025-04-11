use crate::auth::JWT_KEYS;
use axum::response::{IntoResponse, Response};
use axum::{
    extract::FromRequestParts, http::request::Parts, http::StatusCode, Json, RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
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
}
impl fmt::Display for JWTClaims {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Email: {}\nExpiry: {}", self.sub, self.exp)
    }
}

impl<S> FromRequestParts<S> for JWTClaims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        let mut validation = Validation::default();
        validation.validate_exp = true; 

        let token_data = decode::<JWTClaims>(
            bearer.token(),
            &JWT_KEYS.decoding,
            &validation,
        )
        .map_err(|e| match e.kind() {
            ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::InvalidToken,
        })?;

        let claims = token_data.claims;
        Ok(claims)
    }
}
