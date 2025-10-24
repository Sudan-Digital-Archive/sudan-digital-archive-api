use async_trait::async_trait;
use aws_sdk_s3::{
    Client, Config
};
use aws_sdk_s3::operation::put_object::PutObjectError;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_types::region::Region;
use bytes::Bytes;
use std::error::Error;
use aws_sdk_s3::error::ProvideErrorMetadata;

/// Repository trait for S3-compatible storage operations
#[async_trait]
pub trait S3Repo: Send + Sync {
    /// Creates a new instance of the S3 repository
    async fn new(
        region: &str,
        bucket: String,
        endpoint_url: &str,
        access_key: &str,
        secret_key: &str,
    ) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;

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
    /// Returns Error if the upload fails or if the ETag is missing
    async fn upload_from_bytes(
        &self,
        key: &str,
        bytes: Bytes,
        content_type: &str,
    ) -> Result<String, Box<dyn Error>>;

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
    /// Returns S3RepoError if:
    /// * The presigning configuration fails
    /// * The presigned URL generation fails
    /// * The expiration time is invalid
    /// * The object doesn't exist
    async fn get_presigned_url(&self, object_key: &str, expires_in: u64)
        -> Result<String, Box<dyn Error>>;
}

/// Implementation for DigitalOcean Spaces (S3-compatible storage)
#[derive(Debug, Clone)]
pub struct DigitalOceanSpacesRepo {
    client: Client,
    bucket: String,
}

#[async_trait]
impl S3Repo for DigitalOceanSpacesRepo {
    async fn new(
        region: &str,
        bucket: String,
        endpoint_url: &str,
        access_key: &str,
        secret_key: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let region = Region::new(region.to_string());

        if access_key.is_empty() || secret_key.is_empty() {
            return Err("DO Spaces credentials cannot be empty".into());
        }

        std::env::set_var("AWS_ACCESS_KEY_ID", access_key);
        std::env::set_var("AWS_SECRET_ACCESS_KEY", secret_key);

        let config = Config::builder()
            .region(region)
            .endpoint_url(endpoint_url)
            .behavior_version_latest()
            .build();
        let client = Client::from_conf(config);

        Ok(Self { client, bucket })
    }

    async fn upload_from_bytes(
        &self,
        key: &str,
        bytes: Bytes,
        content_type: &str,
    ) -> Result<String, Box<dyn Error>> {
        let result = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(bytes.into())
            .content_type(content_type)
            .send()
            .await;

        match result {
            Ok(output) => {
                output
                    .e_tag()
                    .map(|tag| tag.replace('\"', ""))
                    .ok_or_else(|| "Missing ETag in response".into())
            }
            Err(err) => match err.into_service_error() {
                PutObjectError::EncryptionTypeMismatch(e) => {
                    Err(format!("Object was created with different encryption: {:?}", e).into())
                }
                PutObjectError::InvalidRequest(e) => {
                    Err(format!("Invalid request: {:?}", e).into())
                }
                PutObjectError::InvalidWriteOffset(e) => {
                    Err(format!("Invalid write offset: {:?}", e).into())
                }
                PutObjectError::TooManyParts(e) => {
                    Err(format!("Too many parts (max 10000): {:?}", e).into())
                }
                err => Err(format!("Upload failed: {:#?}", err.code()).into())
            }
        }
    }

    async fn get_presigned_url(
        &self,
        object_key: &str,
        expires_in: u64,
    ) -> Result<String, Box<dyn Error>> {
        let expires_in = std::time::Duration::from_secs(expires_in);

        // First check if the object exists
        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(object_key)
            .send()
            .await
        {
            Ok(_) => (),
            Err(err) => match err.into_service_error() {
                GetObjectError::NoSuchKey(_) => {
                    return Err(format!("Object not found: {}", object_key).into());
                }
                GetObjectError::InvalidObjectState(e) => {
                    return Err(format!("Object is archived and needs to be restored first: {:?}", e).into());
                }
                err => return Err(format!("Service error: {}", err).into()),
            },
        };

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(object_key)
            .presigned(
                aws_sdk_s3::presigning::PresigningConfig::expires_in(expires_in)
                    .map_err(|e| format!("Failed to create presigning config: {}", e))?
            )
            .await
            .map_err(|e| format!("Failed to generate presigned URL: {}", e))?;

        Ok(presigned_request.uri().to_string())
    }
}
