//! Configuration module for Browsertrix web archiving integration and application settings.
//! Handles environment variables and configuration structures for the archiving service.

use crate::models::common::BrowserProfile;
use http::HeaderValue;
use serde::Serialize;
use std::env;
use uuid::Uuid;

/// Configuration for Browsertrix web archiving service
#[derive(Debug, Clone, Default)]
pub struct BrowsertrixConfig {
    pub username: String,
    pub password: String,
    pub org_id: Uuid,
    pub base_url: String,
    pub login_url: String,
    pub create_crawl_url: String,
}

/// Global application configuration
#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub archive_sender_email: String,
    pub browsertrix: BrowsertrixConfig,
    pub cors_urls: Vec<HeaderValue>,
    pub postgres_url: String,
    pub listener_address: String,
    pub jwt_expiry_hours: i64,
    pub jwt_cookie_domain: String,
    pub postmark_api_base: String,
    pub postmark_api_key: String,
    pub digital_ocean_spaces_endpoint_url: String,
    pub digital_ocean_spaces_bucket: String,
    pub digital_ocean_spaces_access_key: String,
    pub digital_ocean_spaces_secret_key: String,
    pub max_file_upload_size: usize,
}

/// Builds application configuration from environment variables
pub fn build_app_config() -> AppConfig {
    let postgres_url = env::var("POSTGRES_URL").expect("Missing POSTGRES_URL env var");
    let archive_sender_email =
        env::var("ARCHIVE_SENDER_EMAIL").expect("Missing ARCHIVE_SENDER_EMAIL env var");
    let postmark_api_base =
        env::var("POSTMARK_API_BASE").expect("Missing POSTMARK_API_BASE env var");
    let postmark_api_key = env::var("POSTMARK_API_KEY").expect("Missing POSTMARK_API_KEY env var");
    let username = env::var("BROWSERTRIX_USERNAME").expect("Missing BROWSERTRIX_USERNAME env var");
    let password = env::var("BROWSERTRIX_PASSWORD").expect("Missing BROWSERTRIX_PASSWORD env var");
    let org_id = env::var("BROWSERTRIX_ORGID").expect("Missing BROWSERTRIX_ORGID env var");
    let org_uuid = Uuid::parse_str(&org_id).expect("Could not parse browsertrix org id to uuid");
    let base_url = env::var("BROWSERTRIX_BROWSERTRIX_URL")
        .expect("Missing BROWSERTRIX_BROWSERTRIX_URL env var");
    let login_url = format!("{base_url}/auth/jwt/login");
    let create_crawl_url = format!("{base_url}/orgs/{org_uuid}/crawlconfigs/");
    let browsertrix = BrowsertrixConfig {
        username,
        password,
        org_id: org_uuid,
        base_url,
        login_url,
        create_crawl_url,
    };
    let jwt_cookie_domain =
        env::var("JWT_COOKIE_DOMAIN").expect("Missing JWT_COOKIE_DOMAIN env var");
    let cors_urls_env_var = env::var("CORS_URL").expect("Missing CORS_URL env var");
    let cors_urls = cors_urls_env_var
        .split(",")
        .map(|s| {
            HeaderValue::from_str(s)
                .expect("CORS_URL env var should contain comma separated origins")
        })
        .collect();
    let listener_address = env::var("LISTENER_ADDRESS").expect("Missing LISTENER_ADDRESS env var");
    let jwt_expiry_hours = env::var("JWT_EXPIRY_HOURS")
        .expect("Missing JWT_EXPIRY_HOURS env var")
        .parse()
        .expect("JWT_EXPIRY_HOURS should be a number");
    let digital_ocean_spaces_endpoint_url =
        env::var("DO_SPACES_ENDPOINT_URL").expect("Missing DO_SPACES_ENDPOINT_URL env var");
    let digital_ocean_spaces_bucket =
        env::var("DO_SPACES_BUCKET").expect("Missing DO_SPACES_BUCKET env var");
    let digital_ocean_spaces_access_key =
        env::var("DO_SPACES_ACCESS_KEY").expect("Missing DO_SPACES_ACCESS_KEY env var");
    let digital_ocean_spaces_secret_key =
        env::var("DO_SPACES_SECRET_KEY").expect("Missing DO_SPACES_SECRET_KEY env var");
    let max_file_upload_size = 200 * 1024 * 1024;
    AppConfig {
        archive_sender_email,
        browsertrix,
        cors_urls,
        postgres_url,
        listener_address,
        jwt_expiry_hours,
        jwt_cookie_domain,
        postmark_api_base,
        postmark_api_key,
        digital_ocean_spaces_endpoint_url,
        digital_ocean_spaces_bucket,
        digital_ocean_spaces_access_key,
        digital_ocean_spaces_secret_key,
        max_file_upload_size,
    }
}

/// Single URL seed configuration for Browsertrix crawl
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OneSeed {
    url: String,
    scope_type: String,
}

/// Configuration for URL crawling behavior and scope
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedsConfig {
    seeds: Vec<OneSeed>,
    scope_type: String,
    extra_hops: i32,
    use_sitemap: bool,
    fail_on_failed_seed: bool,
    behavior_timeout: Option<i32>,
    page_load_timeout: Option<i32>,
    page_extra_delay: Option<i32>,
    post_load_delay: i32,
    user_agent: Option<String>,
    limit: Option<i32>,
    lang: String,
    exclude: Vec<String>,
    behaviors: String,
}

/// Complete crawl configuration for Browsertrix service
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowsertrixCrawlConfig {
    job_type: String,
    name: String,
    description: Option<String>,
    scale: i8,
    profileid: String,
    run_now: bool,
    schedule: String,
    crawl_timeout: i32,
    max_crawl_size: i32,
    tags: Vec<String>,
    auto_add_collections: Vec<String>,
    config: SeedsConfig,
    crawler_channel: String,
    proxy_id: Option<String>,
}

impl BrowsertrixCrawlConfig {
    /// Creates a new crawl configuration for a single URL with default settings
    pub fn new(url: String, browser_profile: Option<BrowserProfile>) -> Self {
        let one_seed = OneSeed {
            url,
            scope_type: "page".to_string(),
        };
        let seeds_config = SeedsConfig {
            seeds: vec![one_seed],
            scope_type: "page".to_string(),
            extra_hops: 0,
            use_sitemap: false,
            fail_on_failed_seed: false,
            behavior_timeout: None,
            page_load_timeout: None,
            page_extra_delay: None,
            post_load_delay: 120,
            user_agent: None,
            limit: None,
            lang: "en".to_string(),
            exclude: vec![],
            behaviors: "autoscroll,autoplay,autofetch,siteSpecific".to_string(),
        };
        let mut profileid = "".to_string();
        if let Some(browser_profile_name) = browser_profile {
            // profile ids here are from Browsertrix API
            // to get them you need to do list profiles
            profileid = match browser_profile_name {
                BrowserProfile::Facebook => "b1cd3192-a554-41e1-9509-0cbff3b3df16".to_string(),
            };
        }
        BrowsertrixCrawlConfig {
            job_type: "custom".to_string(),
            name: "".to_string(),
            description: None,
            scale: 1,
            profileid,
            run_now: true,
            schedule: "".to_string(),
            crawl_timeout: 0,
            max_crawl_size: 1000000000,
            tags: vec![],
            auto_add_collections: vec![],
            config: seeds_config,
            crawler_channel: "default".to_string(),
            proxy_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crawl_config_new_different_urls() {
        let config1 = BrowsertrixCrawlConfig::new("https://example.com".to_string(), None);
        let config2 = BrowsertrixCrawlConfig::new(
            "https://different.com".to_string(),
            Some(BrowserProfile::Facebook),
        );

        assert_eq!(config1.config.seeds[0].url, "https://example.com");
        assert_eq!(config2.config.seeds[0].url, "https://different.com");
        assert_ne!(config1.config.seeds[0].url, config2.config.seeds[0].url);
    }
}
