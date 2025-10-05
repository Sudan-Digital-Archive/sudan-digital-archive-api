use aws_sdk_s3::{Client, Config, error::ProvideErrorMetadata};
use aws_types::region::Region;
use bytes::Bytes;
use std::error::Error;
use std::fmt;
use async_trait::async_trait;

/// Custom error type for S3 operations
#[derive(Debug)]
pub struct S3Error(String);

impl S3Error {
    pub fn new(value: impl Into<String>) -> Self {
        S3Error(value.into())
    }

    pub fn add_message(self, message: impl Into<String>) -> Self {
        S3Error(format!("{}: {}", message.into(), self.0))
    }
}

impl<T: ProvideErrorMetadata> From<T> for S3Error {
    fn from(value: T) -> Self {
        S3Error(format!(
            "{}: {}",
            value
                .code()
                .map(String::from)
                .unwrap_or("unknown code".into()),
            value
                .message()
                .map(String::from)
                .unwrap_or("missing reason".into()),
        ))
    }
}

impl Error for S3Error {}

impl fmt::Display for S3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Repository trait for S3-compatible storage operations
#[async_trait]
pub trait S3Repo: Send + Sync {
    /// Creates a new instance of the S3 repository
    async fn new(region: &str, bucket: String) -> Result<Self, S3Error> where Self: Sized;

    /// Uploads bytes to an S3 bucket
    ///
    /// # Arguments
    /// * `key` - The object key (path) in the S3 bucket
    /// * `bytes` - The byte content to upload, note 5GB or over will fail since that needs multipart upload
    /// * `content_type` - MIME type of the content (e.g. "application/pdf")
    ///
    /// # Returns
    /// Result containing the object's ETag on success
    ///
    /// # Errors
    /// Returns S3Error if the upload fails or if the ETag is missing
    async fn upload_from_bytes(
        &self,
        key: &str,
        bytes: Bytes,
        content_type: &str,
    ) -> Result<String, S3Error>;

    /// Generates a presigned URL for an S3 object that allows temporary access without credentials
    ///
    /// # Arguments
    /// * `object_key` - The key (path) of the object in the S3 bucket
    /// * `expires_in` - Duration in seconds until the presigned URL expires
    ///
    /// # Returns
    /// A presigned URL that can be used to access the object for the specified duration
    ///
    /// # Errors
    /// Returns S3Error if:
    /// * The presigning configuration fails
    /// * The presigned URL generation fails
    /// * The expiration time is invalid
    async fn get_presigned_url(&self, object_key: &str, expires_in: u64) -> Result<String, S3Error>;
}

/// Implementation for DigitalOcean Spaces (S3-compatible storage)
#[derive(Debug, Clone)]
pub struct DigitalOceanSpacesRepo {
    client: Client,
    bucket: String,
}

#[async_trait]
impl S3Repo for DigitalOceanSpacesRepo {
    async fn new(region: &str, bucket: String) -> Result<Self, S3Error> {
        let region = Region::new(region.to_string());
        let config = Config::builder()
            .region(region)
            .build();
        let client = Client::from_conf(config);
        
        Ok(Self { client, bucket })
    }

    async fn upload_from_bytes(
        &self,
        key: &str,
        bytes: Bytes,
        content_type: &str,
    ) -> Result<String, S3Error> {
        let result = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(bytes.into())
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| S3Error::from(e).add_message("Failed to upload object"))?;

        result
            .e_tag()
            .map(|tag| tag.replace('\"', ""))
            .ok_or_else(|| S3Error::new("Missing ETag in response"))
    }

    async fn get_presigned_url(&self, object_key: &str, expires_in: u64) -> Result<String, S3Error> {
        let expires_in = std::time::Duration::from_secs(expires_in);
        
        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(object_key)
            .presigned(aws_sdk_s3::presigning::PresigningConfig::expires_in(expires_in)
                .map_err(|e| S3Error::new(format!("Failed to create presigning config: {}", e)))?)
            .await
            .map_err(|e| S3Error::from(e).add_message("Failed to generate presigned URL"))?;

        Ok(presigned_request.uri().to_string())
    }
}
