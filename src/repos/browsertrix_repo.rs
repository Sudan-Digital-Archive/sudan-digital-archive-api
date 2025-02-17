use crate::config::BrowsertrixCrawlConfig;
use crate::models::request::CreateCrawlRequest;
use crate::models::response::{
    AuthResponse, CreateCrawlResponse, GetCrawlResponse, GetWaczUrlResponse,
};
use async_trait::async_trait;
use reqwest::{Client, Error, RequestBuilder, Response, StatusCode};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct HTTPBrowsertrixRepo {
    pub username: String,
    pub password: String,
    pub org_id: Uuid,
    pub base_url: String,
    pub client: Client,
    pub login_url: String,
    pub create_crawl_url: String,
    pub access_token: Arc<RwLock<String>>,
}

#[async_trait]
pub trait BrowsertrixRepo: Send + Sync {
    fn get_org_id(&self) -> Uuid;
    async fn refresh_auth(&self);

    async fn get_wacz_url(&self, job_run_id: &str) -> Result<String, Error>;
    async fn make_request(&self, req: RequestBuilder) -> Result<Response, Error>;
    async fn authenticate(&self) -> Result<String, Error>;
    async fn initialize(&mut self);
    async fn create_crawl(
        &self,
        create_crawl_request: CreateCrawlRequest,
    ) -> Result<CreateCrawlResponse, Error>;
    async fn get_crawl_status(&self, crawl_id: Uuid) -> Result<String, Error>;
}

#[async_trait]
impl BrowsertrixRepo for HTTPBrowsertrixRepo {
    fn get_org_id(&self) -> Uuid {
        self.org_id
    }

    async fn refresh_auth(&self) {
        let new_access_token = self
            .authenticate()
            .await
            .expect("Error logging into Browsertrix");
        let mut access_token = self.access_token.write().await;
        *access_token = new_access_token.clone();
    }
    async fn get_wacz_url(&self, job_run_id: &str) -> Result<String, Error> {
        let get_wacz_url = format!(
            "{}/orgs/{}/crawls/{job_run_id}/replay.json",
            self.base_url, self.org_id
        );
        let req = self
            .client
            .get(get_wacz_url.clone())
            .bearer_auth(self.access_token.read().await.clone());
        let get_wacz_url_resp = self.make_request(req).await?;
        let get_wacz_url_resp_json: GetWaczUrlResponse = get_wacz_url_resp.json().await?;
        let wacz_url = &get_wacz_url_resp_json.resources[0].path;
        Ok(wacz_url.to_string())
    }

    async fn make_request(&self, req: RequestBuilder) -> Result<Response, Error> {
        let mut resp = req
            .try_clone()
            .expect("Requests should not be made with streams fool")
            .send()
            .await?;
        if resp.status() == StatusCode::UNAUTHORIZED {
            info!("Got 403 HTTP code, reauthenticating...");
            self.refresh_auth().await;
            resp = req.send().await?;
        }
        Ok(resp)
    }
    async fn authenticate(&self) -> Result<String, Error> {
        let mut params = HashMap::new();
        params.insert("username", self.username.clone());
        params.insert("password", self.password.clone());
        let auth_resp = self
            .client
            .post(self.login_url.clone())
            .form(&params)
            .send()
            .await?;
        let auth_json_resp: AuthResponse = auth_resp.json().await?;
        Ok(auth_json_resp.access_token)
    }

    async fn initialize(&mut self) {
        let new_access_token = self
            .authenticate()
            .await
            .expect("Error logging into Browsertrix");
        let mut access_token = self.access_token.write().await;
        *access_token = new_access_token;
    }

    async fn create_crawl(
        &self,
        create_crawl_request: CreateCrawlRequest,
    ) -> Result<CreateCrawlResponse, Error> {
        let json_payload = BrowsertrixCrawlConfig::new(
            create_crawl_request.url,
            create_crawl_request.browser_profile,
        );
        let create_crawl_req = self
            .client
            .post(self.create_crawl_url.clone())
            .json(&json_payload)
            .bearer_auth(self.access_token.read().await.clone());
        let create_crawl_resp = self.make_request(create_crawl_req).await?;
        let create_crawl_resp_json: CreateCrawlResponse = create_crawl_resp.json().await?;
        Ok(create_crawl_resp_json)
    }

    async fn get_crawl_status(&self, crawl_id: Uuid) -> Result<String, Error> {
        let get_crawl_status_url = format!(
            "{}/orgs/{}/crawlconfigs/{crawl_id}",
            self.base_url, self.org_id
        );
        let get_crawl_req = self
            .client
            .get(get_crawl_status_url.clone())
            .bearer_auth(self.access_token.read().await.clone());
        let get_crawl_resp = self.make_request(get_crawl_req).await?;
        let get_crawl_resp_json: GetCrawlResponse = get_crawl_resp.json().await?;
        Ok(get_crawl_resp_json.last_crawl_state)
    }
}
