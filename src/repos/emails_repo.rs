use async_trait::async_trait;
use reqwest::{Client, Error};
use serde::Serialize;

#[derive(Default)]
pub struct PostmarkEmailsRepo {
    pub client: Client,
    pub api_key: String,
    pub archive_sender_email: String,
    pub postmark_api_base: String,
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
    async fn send_email(&self, to: String, subject: String, email: String) -> Result<(), Error>;
}

#[async_trait]
impl EmailsRepo for PostmarkEmailsRepo {
    async fn send_email(&self, to: String, subject: String, email: String) -> Result<(), Error> {
        let message = EmailMessage {
            from: self.archive_sender_email.clone(),
            to,
            subject,
            html_body: email,
        };
        let resp = self
            .client
            .post(format!("{}/email", self.postmark_api_base))
            .header("X-Postmark-Server-Token", self.api_key.clone())
            .json(&message)
            .send()
            .await?;
        match resp.error_for_status() {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }
}
