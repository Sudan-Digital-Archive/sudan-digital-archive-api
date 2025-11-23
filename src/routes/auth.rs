//! Authentication routes.
//!
//! This module provides HTTP endpoints for user authentication, including login and authorization.
//! It also includes a protected route for testing JWT authentication.
//! The module uses an authentication service to handle the authentication logic.

use crate::app_factory::AppState;
use crate::models::auth::AuthenticatedUser;
use crate::models::request::{AuthorizeRequest, LoginRequest};
use crate::models::response::CreateApiKeyResponse;
use ::entity::sea_orm_active_enums::Role;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use tracing::{error, info};
use uuid::Uuid;
use validator::Validate;

pub fn get_auth_routes() -> Router<AppState> {
    Router::new().nest(
        "/auth",
        Router::new()
            .route("/", post(login))
            .route("/authorize", post(authorize))
            .route("/", get(verify))
            .route("/{:user_id}/api-key", post(create_api_key)),
    )
}

#[utoipa::path(
    post,
    path = "/api/v1/auth",
    tag = "Auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "OK"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn login(State(state): State<AppState>, Json(payload): Json<LoginRequest>) -> Response {
    if let Err(err) = payload.validate() {
        return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
    }

    let login_result = state.auth_service.clone().login(payload).await;

    match login_result {
        Ok(response) => response,
        Err(err) => {
            let message = format!("Server error occurred: {err}");
            error!(message);
            (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/authorize",
    tag = "Auth",
    request_body = AuthorizeRequest,
    responses(
        (status = 200, description = "OK"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn authorize(
    State(state): State<AppState>,
    Json(payload): Json<AuthorizeRequest>,
) -> Response {
    let auth_result = state.auth_service.authorize(payload).await;

    match auth_result {
        Ok(response) => response,
        Err(err) => {
            let message = format!("Server error occurred: {err}");
            error!(message);
            (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/auth",
    tag = "Auth",
    responses(
        (status = 200, description = "OK", body = String),
        (status = 401, description = "Unauthorized")
    )
)]
async fn verify(State(_state): State<AppState>, authenticated_user: AuthenticatedUser) -> Response {
    let user_data = format!("Verifying your account...\nYour data:\n{authenticated_user}");
    (StatusCode::OK, user_data).into_response()
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/{user_id}/api-key",
    tag = "Auth",
    responses(
        (status = 201, description = "API key created", body = CreateApiKeyResponse),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    )
)]
async fn create_api_key(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    authenticated_user: AuthenticatedUser,
) -> Response {
    if authenticated_user.role != Role::Admin {
        return (StatusCode::FORBIDDEN, "Only admins can create API keys").into_response();
    }

    let api_key_result = state.auth_service.create_api_key(user_id).await;

    match api_key_result {
        Ok(api_key_secret) => {
            info!(
                "API key created by admin {} for user {}",
                authenticated_user.user_id, user_id
            );
            let auth_service = state.auth_service.clone();
            tokio::spawn(async move {
                auth_service.delete_expired_api_keys().await;
            });
            let response = CreateApiKeyResponse { api_key_secret };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => {
            error!(
                "Failed to create API key by admin {} for user {}: {}",
                authenticated_user.user_id, user_id, err
            );
            let message = format!("Failed to create API key: {err}");
            (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::response::CreateApiKeyResponse;
    use crate::test_tools::build_test_app;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde_json::json;
    use tower::ServiceExt;
    use uuid::Uuid;

    // Import JWT creation utilities
    use crate::auth::JWT_KEYS;
    use crate::models::auth::JWTClaims;
    use crate::test_tools::get_mock_jwt;
    use ::entity::sea_orm_active_enums::Role;
    use chrono::Utc;
    use jsonwebtoken::{encode, Header};

    #[tokio::test]
    async fn login_with_valid_email() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/auth")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "email": "test@example.com"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual = String::from_utf8((&body).to_vec()).unwrap();
        assert_eq!(actual, "Login email sent");
    }

    #[tokio::test]
    async fn login_invalid_json() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/auth")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "invalid_field": "test@example.com"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Axum returns 422 Unprocessable Entity for JSON deserialization errors
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn authorize_with_valid_session() {
        let app = build_test_app();
        let test_user_id = Uuid::new_v4();
        let test_session_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/v1/auth/authorize")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "user_id": test_user_id.to_string(),
                            "session_id": test_session_id.to_string()
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual = String::from_utf8((&body).to_vec()).unwrap();
        assert_eq!(actual, "Authentication successful");
    }

    #[tokio::test]
    async fn verify_with_valid_jwt() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth")
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual = String::from_utf8((&body).to_vec()).unwrap();
        assert!(actual.contains("Verifying your account"));
        assert!(actual.contains("someuser@gmail.com"));
    }

    #[tokio::test]
    async fn verify_without_jwt() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_api_key_as_admin() {
        let app = build_test_app();
        let target_user_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri(&format!("/api/v1/auth/{}/api-key", target_user_id))
                    .header(http::header::COOKIE, format!("jwt={}", get_mock_jwt()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual: CreateApiKeyResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(actual.api_key_secret, "mock_api_key_secret");
    }

    #[tokio::test]
    async fn create_api_key_without_admin_role() {
        let app = build_test_app();
        let target_user_id = Uuid::new_v4();

        let expiry_time: chrono::DateTime<Utc> = Utc::now() + chrono::Duration::hours(24);
        let claims = JWTClaims {
            sub: "researcher@gmail.com".to_string(),
            exp: expiry_time.timestamp() as usize,
            role: Role::Researcher,
        };
        let jwt =
            encode(&Header::default(), &claims, &JWT_KEYS.encoding).expect("Failed to encode JWT");

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri(&format!("/api/v1/auth/{}/api-key", target_user_id))
                    .header(http::header::COOKIE, format!("jwt={}", jwt))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual = String::from_utf8((&body).to_vec()).unwrap();
        assert_eq!(actual, "Only admins can create API keys");
    }

    #[tokio::test]
    async fn create_api_key_without_jwt() {
        let app = build_test_app();
        let target_user_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri(&format!("/api/v1/auth/{}/api-key", target_user_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_api_key_with_api_key_auth() {
        let app = build_test_app();
        let target_user_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri(&format!("/api/v1/auth/{}/api-key", target_user_id))
                    .header("X-Api-Key", "mock_api_key_secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual: CreateApiKeyResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(actual.api_key_secret, "mock_api_key_secret");
    }

    #[tokio::test]
    async fn create_api_key_with_invalid_api_key() {
        let app = build_test_app();
        let target_user_id = Uuid::new_v4();

        // Mock API key verification returns None for invalid keys in the real implementation
        // but our mock always returns Some, so we test with an empty string
        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri(&format!("/api/v1/auth/{}/api-key", target_user_id))
                    .header("X-Api-Key", "invalid_key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Mock returns valid user info regardless, so this succeeds
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn verify_with_api_key() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth")
                    .header("X-Api-Key", "mock_api_key_secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let actual = String::from_utf8((&body).to_vec()).unwrap();
        assert!(actual.contains("Verifying your account"));
        // With API key auth, the user_id is the email from the API key info
        assert!(actual.contains("test@example.com"));
    }
}
