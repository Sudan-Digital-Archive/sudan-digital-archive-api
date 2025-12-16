use async_trait::async_trait;
use aws_config;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_sdk_s3::operation::put_object::PutObjectError;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use aws_sdk_s3::Client;
use aws_smithy_types::byte_stream::ByteStream;
use axum::extract::multipart::Field;
use bytes::Bytes;
use futures::StreamExt;
use std::error::Error;
use std::pin::Pin;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tracing::{error, info};
//In bytes, minimum chunk size of 5MB. Increase CHUNK_SIZE to send larger chunks.
const CHUNK_SIZE: usize = 1024 * 1024 * 5;

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

    /// Uploads chunks to an S3 bucket
    ///
    /// # Arguments
    /// * `key` - The object key (path) in the S3 bucket
    /// * `chunks` - Vector of byte chunks to upload
    /// * `content_type` - MIME type of the content (e.g. "application/pdf")
    ///
    /// # Returns
    /// Result containing the object's upload ID on success
    ///
    /// # Errors
    /// Returns Error if the upload fails
    async fn upload_from_chunks(
        &self,
        key: &str,
        chunks: Vec<Bytes>,
        content_type: &str,
    ) -> Result<String, Box<dyn Error>>;

    /// Uploads a stream to an S3 bucket
    ///
    /// # Arguments
    /// * `key` - The object key (path) in the S3 bucket
    /// * `reader` - The async reader for the stream
    /// * `content_type` - MIME type of the content (e.g. "application/pdf")
    ///
    /// # Returns
    /// Result containing the object's upload ID on success
    ///
    /// # Errors
    /// Returns Error if the upload fails
    async fn upload_from_stream(
        &self,
        key: &str,
        reader: Pin<&mut (dyn AsyncRead + Send)>,
        content_type: &str,
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

    async fn upload_from_chunks(
        &self,
        key: &str,
        chunks: Vec<Bytes>,
        content_type: &str,
    ) -> Result<String, Box<dyn Error>> {
        info!("Starting multipart upload from chunks for key: {}, content_type: {}", key, content_type);
        
        let mut buffer = Vec::with_capacity(CHUNK_SIZE);
        let mut part_number = 1;
        let mut upload_parts: Vec<aws_sdk_s3::types::CompletedPart> = Vec::new();
        let mut total_size = 0;

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
            .ok_or("Missing upload_id after CreateMultipartUpload")?;

        info!("Created multipart upload with id: {}", upload_id);

        // Process each chunk
        for chunk in chunks {
            buffer.extend_from_slice(&chunk);
            total_size += chunk.len();
            info!("Added {} bytes to buffer, total so far: {}", chunk.len(), total_size);

            // Upload a part if buffer has reached chunk size
            if buffer.len() >= CHUNK_SIZE {
                info!("Uploading part {} with {} bytes", part_number, buffer.len());
                let upload_part_res = self
                    .client
                    .upload_part()
                    .key(key)
                    .bucket(&self.bucket)
                    .upload_id(upload_id)
                    .body(ByteStream::from(buffer.split_off(0)))
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

                upload_parts.push(
                    CompletedPart::builder()
                        .e_tag(upload_part_res.e_tag.unwrap_or_default())
                        .part_number(part_number)
                        .build(),
                );

                part_number += 1;
            }
        }

        // Upload any remaining data in buffer as the final part
        if !buffer.is_empty() {
            info!("Uploading final part {} with {} bytes", part_number, buffer.len());
            let upload_part_res = self
                .client
                .upload_part()
                .key(key)
                .bucket(&self.bucket)
                .upload_id(upload_id)
                .body(ByteStream::from(buffer))
                .part_number(part_number)
                .send()
                .await
                .map_err(|err| {
                    format!(
                        "Failed to upload final part {}: {}",
                        part_number,
                        err.into_service_error().code().unwrap_or("unknown")
                    )
                })?;

            upload_parts.push(
                CompletedPart::builder()
                    .e_tag(upload_part_res.e_tag.unwrap_or_default())
                    .part_number(part_number)
                    .build(),
            );
        }

        let completed_multipart_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(upload_parts))
            .build();

        self.client
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

        info!("Multipart chunks upload completed successfully for key: {}, total size: {} bytes", key, total_size);
        Ok(upload_id.to_string())
    }

    // based off docs here https://docs.aws.amazon.com/sdk-for-rust/latest/dg/rust_s3_code_examples.html
    async fn upload_from_stream(
        &self,
        key: &str,
        mut reader: Pin<&mut (dyn AsyncRead + Send)>,
        content_type: &str,
    ) -> Result<String, Box<dyn Error>> {
        info!("Starting multipart upload for key: {}, content_type: {}", key, content_type);
        let mut buffer = Vec::with_capacity(CHUNK_SIZE);
        let mut total_size = 0;
        let mut part_number = 1;
        let mut upload_parts: Vec<aws_sdk_s3::types::CompletedPart> = Vec::new();

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
            .ok_or("Missing upload_id after CreateMultipartUpload")?;

        info!("Created multipart upload with id: {}", upload_id);

        loop {
            info!("Reading chunk {}, current buffer size: {}", part_number, buffer.len());
            let bytes_read = reader
                .read_buf(&mut buffer)
                .await
                .map_err(|err| {
                    let err_msg = format!("Failed to read from stream: {}", err);
                    error!("{}", err_msg);
                    err_msg
                })?;
            
            info!("Read {} bytes from stream, total so far: {}", bytes_read, total_size + bytes_read);
            
            if bytes_read == 0 {
                // EOF reached - check if file is small enough for simple upload
                if total_size <= CHUNK_SIZE {
                    info!("File is small enough for simple upload, uploading {} bytes", total_size);
                    return self
                        .upload_from_bytes(key, buffer.into(), content_type)
                        .await;
                }
                break;
            }

            total_size += bytes_read;

            // Only upload a part if buffer has reached chunk size
            if buffer.len() >= CHUNK_SIZE {
                info!("Uploading part {} with {} bytes", part_number, buffer.len());
                let upload_part_res = self
                    .client
                    .upload_part()
                    .key(key)
                    .bucket(&self.bucket)
                    .upload_id(upload_id)
                    .body(ByteStream::from(buffer.split_off(0)))
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

                upload_parts.push(
                    CompletedPart::builder()
                        .e_tag(upload_part_res.e_tag.unwrap_or_default())
                        .part_number(part_number)
                        .build(),
                );

                part_number += 1;
            }
        }

        // Upload any remaining data in buffer as the final part
        if !buffer.is_empty() {
            info!("Uploading final part {} with {} bytes", part_number, buffer.len());
            let upload_part_res = self
                .client
                .upload_part()
                .key(key)
                .bucket(&self.bucket)
                .upload_id(upload_id)
                .body(ByteStream::from(buffer))
                .part_number(part_number)
                .send()
                .await
                .map_err(|err| {
                    format!(
                        "Failed to upload final part {}: {}",
                        part_number,
                        err.into_service_error().code().unwrap_or("unknown")
                    )
                })?;

            upload_parts.push(
                CompletedPart::builder()
                    .e_tag(upload_part_res.e_tag.unwrap_or_default())
                    .part_number(part_number)
                    .build(),
            );
        }

        let completed_multipart_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(upload_parts))
            .build();

        self.client
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

        info!("Multipart upload completed successfully for key: {}, total size: {} bytes", key, total_size);
        Ok(upload_id.to_string())
    }
}
