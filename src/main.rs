mod app_factory;
mod auth;
mod config;
mod models;
mod open_api_spec;
mod repos;
mod routes;
mod services;
#[cfg(test)]
mod test_tools;

use crate::app_factory::{create_app, AppState};
use crate::config::build_app_config;
use crate::repos::accessions_repo::DBAccessionsRepo;
use crate::repos::auth_repo::DBAuthRepo;
use crate::repos::browsertrix_repo::{BrowsertrixRepo, HTTPBrowsertrixRepo};
use crate::repos::emails_repo::PostmarkEmailsRepo;
use crate::repos::subjects_repo::DBSubjectsRepo;
use crate::services::accessions_service::AccessionsService;
use crate::services::auth_service::AuthService;
use crate::services::subjects_service::SubjectsService;
use reqwest::Client;
use sea_orm::Database;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::info;

#[tokio::main]
async fn main() {
    let app_config = build_app_config();
    let db_session = Database::connect(app_config.postgres_url)
        .await
        .expect("Could not connect to db");
    let accessions_repo = DBAccessionsRepo {
        db_session: db_session.clone(),
    };
    let auth_repo = DBAuthRepo {
        db_session: db_session.clone(),
        expiry_hours: app_config.jwt_expiry_hours,
    };
    let emails_repo = PostmarkEmailsRepo {
        client: Client::new(),
        archive_sender_email: app_config.archive_sender_email,
        api_key: app_config.postmark_api_key,
        postmark_api_base: app_config.postmark_api_base,
    };
    let subjects_repo = DBSubjectsRepo { db_session };
    let mut http_btrix_repo = HTTPBrowsertrixRepo {
        client: Client::new(),
        login_url: app_config.browsertrix.login_url,
        username: app_config.browsertrix.username,
        password: app_config.browsertrix.password,
        base_url: app_config.browsertrix.base_url,
        org_id: app_config.browsertrix.org_id,
        access_token: Arc::new(RwLock::new(String::new())),
        create_crawl_url: app_config.browsertrix.create_crawl_url,
    };
    http_btrix_repo.initialize().await;
    let accessions_service = AccessionsService {
        accessions_repo: Arc::new(accessions_repo),
        browsertrix_repo: Arc::new(http_btrix_repo),
        emails_repo: Arc::new(emails_repo.clone()),
    };
    let auth_service = AuthService {
        auth_repo: Arc::new(auth_repo),
        emails_repo: Arc::new(emails_repo),
        jwt_cookie_domain: app_config.jwt_cookie_domain,
    };
    let subjects_service = SubjectsService {
        subjects_repo: Arc::new(subjects_repo),
    };
    let app_state = AppState {
        accessions_service,
        auth_service,
        subjects_service,
    };
    let app = create_app(app_state, app_config.cors_urls, false);

    let addr: SocketAddr = app_config
        .listener_address
        .parse()
        .expect("Should be in address format like 0.0.0.0:5000");

    info!("listening on {}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
