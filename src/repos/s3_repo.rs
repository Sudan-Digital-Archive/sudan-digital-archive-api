use async_trait::async_trait;
use aws_config;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_sdk_s3::operation::put_object::PutObjectError;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use aws_sdk_s3::Client;
use aws_smithy_types::byte_stream::ByteStream;
use bytes::Bytes;
use std::error::Error;
use tracing::info;

// Repository trait for S3-compatible storage operations
#[async_trait]
pub trait S3Repo: Send + Sync {
    /// Creates a new instance of the S3 repository
    async fn new(
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
    async fn get_presigned_url(
        &self,
        object_key: &str,
        expires_in: u64,
    ) -> Result<String, Box<dyn Error>>;


    /// Initiates a multipart upload to S3.
    ///
    /// # Arguments
    /// * `key` - The object key (path) in the S3 bucket
    /// * `content_type` - MIME type of the content (e.g. "application/wacz")
    ///
    /// # Returns
    /// Result containing the upload ID on success
    ///
    /// # Errors
    /// Returns Error if the multipart upload creation fails
    async fn initiate_multipart_upload(
        &self,
        key: &str,
        content_type: &str,
    ) -> Result<String, Box<dyn Error>>;

    /// Uploads a single part in a multipart upload.
    ///
    /// # Arguments
    /// * `key` - The object key (path) in the S3 bucket
    /// * `upload_id` - The ID of the multipart upload
    /// * `part_number` - The part number (1-indexed, must be sequential)
    /// * `bytes` - The data to upload for this part
    ///
    /// # Returns
    /// Result containing a tuple of (ETag, part_number) on success
    ///
    /// # Errors
    /// Returns Error if the part upload fails
    async fn upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: i32,
        bytes: Bytes,
    ) -> Result<(String, i32), Box<dyn Error>>;

    /// Completes a multipart upload.
    ///
    /// # Arguments
    /// * `key` - The object key (path) in the S3 bucket
    /// * `upload_id` - The ID of the multipart upload
    /// * `parts` - Vector of (ETag, part_number) tuples for each uploaded part
    ///
    /// # Returns
    /// Result containing the final ETag on success
    ///
    /// # Errors
    /// Returns Error if the completion fails
    async fn complete_multipart_upload(
        &self,
        key: &str,
        upload_id: &str,
        parts: Vec<(String, i32)>,
    ) -> Result<String, Box<dyn Error>>;

}

/// Implementation for DigitalOcean Spaces (S3-compatible storage)
#[derive(Debug, Clone)]
pub struct DigitalOceanSpacesRepo {
    client: Client,
    bucket: String,
}

// TODO: Add https://github.com/awsdocs/aws-doc-sdk-examples/blob/main/rustv1/examples/s3/src/bin/s3-multipart-upload.rs#L48
#[async_trait]
impl S3Repo for DigitalOceanSpacesRepo {
    async fn new(
        bucket: String,
        endpoint_url: &str,
        access_key: &str,
        secret_key: &str,
    ) -> Result<Self, Box<dyn Error>> {
        if access_key.is_empty() || secret_key.is_empty() {
            return Err("DO Spaces credentials cannot be empty".into());
        }

        std::env::set_var("AWS_ACCESS_KEY_ID", access_key);
        std::env::set_var("AWS_SECRET_ACCESS_KEY", secret_key);

        let s3_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(endpoint_url)
            .region("us-east-1")
            .load()
            .await;

        let client = Client::new(&s3_config);
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
            Ok(output) => output
                .e_tag()
                .map(|tag| tag.replace('\"', ""))
                .ok_or_else(|| "Missing ETag in response".into()),
            Err(err) => match err.into_service_error() {
                PutObjectError::EncryptionTypeMismatch(e) => {
                    Err(format!("Object was created with different encryption: {e:?}").into())
                }
                PutObjectError::InvalidRequest(e) => Err(format!("Invalid request: {e:?}").into()),
                PutObjectError::InvalidWriteOffset(e) => {
                    Err(format!("Invalid write offset: {e:?}").into())
                }
                PutObjectError::TooManyParts(e) => {
                    Err(format!("Too many parts (max 10000): {e:?}").into())
                }
                err => Err(format!("Upload failed: {:#?}", err.code()).into()),
            },
        }
    }

    async fn get_presigned_url(
        &self,
        object_key: &str,
        expires_in: u64,
    ) -> Result<String, Box<dyn Error>> {
        let expires_in = std::time::Duration::from_secs(expires_in);

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
                    return Err(format!("Object not found: {object_key}").into());
                }
                GetObjectError::InvalidObjectState(e) => {
                    return Err(format!(
                        "Object is archived and needs to be restored first: {e:?}"
                    )
                    .into());
                }
                err => return Err(format!("Service error: {err}").into()),
            },
        };

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(object_key)
            .presigned(
                PresigningConfig::expires_in(expires_in)
                    .map_err(|e| format!("Failed to create presigning config: {e}"))?,
            )
            .await
            .map_err(|e| format!("Failed to generate presigned URL: {e}"))?;

        Ok(presigned_request.uri().to_string())
    }


    async fn initiate_multipart_upload(
        &self,
        key: &str,
        content_type: &str,
    ) -> Result<String, Box<dyn Error>> {
        let multipart_upload_res = self
            .client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .send()
            .await
            .map_err(|err| {
                format!(
                    "Failed to create multipart upload: {}",
                    err.into_service_error().code().unwrap_or("unknown")
                )
            })?;

        let upload_id = multipart_upload_res
            .upload_id()
            .ok_or("Missing upload_id after CreateMultipartUpload")?
            .to_string();

        info!("Created multipart upload with id: {}", upload_id);
        Ok(upload_id)
    }

    async fn upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: i32,
        bytes: Bytes,
    ) -> Result<(String, i32), Box<dyn Error>> {
        info!("Uploading part {} ({} bytes) for key: {}", part_number, bytes.len(), key);
        
        let upload_part_res = self
            .client
            .upload_part()
            .key(key)
            .bucket(&self.bucket)
            .upload_id(upload_id)
            .body(ByteStream::from(bytes))
            .part_number(part_number)
            .send()
            .await
            .map_err(|err| {
                format!(
                    "Failed to upload part {}: {}",
                    part_number,
                    err.into_service_error().code().unwrap_or("unknown")
                )
            })?;

        let etag = upload_part_res.e_tag().unwrap_or_default().to_string();
        info!("Part {} uploaded successfully with ETag: {}", part_number, etag);
        Ok((etag, part_number))
    }

    async fn complete_multipart_upload(
        &self,
        key: &str,
        upload_id: &str,
        parts: Vec<(String, i32)>,
    ) -> Result<String, Box<dyn Error>> {
        info!("Completing multipart upload for key: {} with {} parts", key, parts.len());
        
        let completed_parts: Vec<CompletedPart> = parts
            .into_iter()
            .map(|(etag, part_number)| {
                CompletedPart::builder()
                    .e_tag(etag)
                    .part_number(part_number)
                    .build()
            })
            .collect();

        let completed_multipart_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        let result = self
            .client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .multipart_upload(completed_multipart_upload)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(|err| {
                format!(
                    "Failed to complete multipart upload: {}",
                    err.into_service_error().code().unwrap_or("unknown")
                )
            })?;

        let final_etag = result.e_tag().unwrap_or_default().to_string();
        info!("Multipart upload completed for key: {} with ETag: {}", key, final_etag);
        Ok(final_etag)
    }
}
