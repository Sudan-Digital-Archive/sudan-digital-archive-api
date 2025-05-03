use async_trait::async_trait;
use reqwest::{Client, Error};
use serde::Serialize;

#[derive(Default)]
pub struct PostmarkEmailsRepo {
    pub client: Client,
    pub api_key: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmailMessage {
    from: String,
    to: String,
    subject: String,
    html_body: String,
}
#[async_trait]
pub trait EmailsRepo: Send + Sync {
    async fn send_email(&self, email: String) -> Result<(), Error>;
}

#[async_trait]
impl EmailsRepo for PostmarkEmailsRepo {
    async fn send_email(&self, email: String) -> Result<(), Error> {
        let test_message = EmailMessage {
            from: "test@example.com".to_string(),
            to: "someuser@example.com".to_string(),
            subject: "your login email".to_string(),
            html_body: email,
        };
        let resp = self
            .client
            .post("https://api.postmarkapp.com/email")
            // hardcode  for testing - careful not to unset since they are strict
            .header("X-Postmark-Server-Token", "POSTMARK_API_TEST")
            .json(&test_message)
            .send()
            .await?;
        match resp.error_for_status() {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }
}
