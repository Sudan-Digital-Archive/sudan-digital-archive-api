mod app_factory;
mod config;
mod models;
mod repos;
mod routes;
mod services;
#[cfg(test)]
mod test_tools;

use crate::app_factory::{create_app, AppState};
use crate::config::build_app_config;
use crate::repos::accessions_repo::DBAccessionsRepo;
use crate::repos::browsertrix_repo::{BrowsertrixRepo, HTTPBrowsertrixRepo};
use crate::repos::subjects_repo::DBSubjectsRepo;
use crate::services::accessions_service::AccessionsService;
use crate::services::subjects_service::SubjectsService;
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
    // TODO: Double check in docs this is fine to clone
    let accessions_repo = DBAccessionsRepo {
        db_session: db_session.clone(),
    };
    let subjects_repo = DBSubjectsRepo { db_session };
    let mut http_btrix_repo = HTTPBrowsertrixRepo {
        client: reqwest::Client::new(),
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
    };
    let subjects_service = SubjectsService {
        subjects_repo: Arc::new(subjects_repo),
    };
    let app_state = AppState {
        accessions_service,
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
