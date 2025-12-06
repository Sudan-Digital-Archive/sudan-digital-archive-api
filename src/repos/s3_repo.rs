use async_trait::async_trait;
use aws_config;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_sdk_s3::operation::put_object::PutObjectError;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;
use bytes::Bytes;
use std::error::Error;

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
    // https://docs.aws.amazon.com/sdk-for-rust/latest/dg/rust_s3_code_examples.html 
    // async fn upload_from_stream(&self, stream: BodyStream) -> Result<String, Box<dyn Error>>;

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

//     // based off docs here https://docs.aws.amazon.com/sdk-for-rust/latest/dg/rust_s3_code_examples.html
// async fn upload_from_stream(
//     &self,
//     mut stream: BodyStream,
// ) -> Result<String, Box<dyn Error>> {
//     // Define constants
//     // TODO: This should not be here put elsewhere 
//     const CHUNK_SIZE: usize = 5 * 1024 * 1024; // 5MB
//     // TODO: Tidy up types and iport this
//     let mut upload_parts: Vec<aws_sdk_s3::types::CompletedPart> = Vec::new();
    
//     // Create a multipart upload
//     let multipart_upload_res = self
//         .client
//         .create_multipart_upload()
//         .bucket(&self.bucket)
//         .key("your_object_key") // TODO: Change this to your desired key from json metadata
//         .content_type("application/octet-stream") // TODO: Change this to your desired key from json metadata
//         .send()
//         .await?;
    
//     let upload_id = multipart_upload_res.upload_id().ok_or("Missing upload_id")?;

//     let mut part_number = 1;
    
//     // Stream chunks from the incoming BodyStream
//     while let Some(chunk) = stream.next().await {
//         match chunk {
//             Ok(data) => {
//                 // TODO: Tidy this up or make sure that if it's under chunk size it still works
//                 // Split the chunk into parts if it exceeds CHUNK_SIZE
//                 let mut offset = 0;
//                 while offset < data.len() {
//                     let end = std::cmp::min(offset + CHUNK_SIZE, data.len());
//                     let part_data = data.slice(offset..end);
                    
//                     // Upload part
//                     let upload_part_res = self
//                         .client
//                         .upload_part()
//                         .bucket(&self.bucket)
//                         .key("your_object_key") // TODO: Think this should be filename from JSON?
//                         .upload_id(upload_id)
//                         .part_number(part_number)
//                         .body(part_data.into())
//                         .send()
//                         .await?;
                    
//                     upload_parts.push(
//                         // TODO: Import this so it's cleaner
//                         aws_sdk_s3::types::CompletedPart::builder()
//                             .e_tag(upload_part_res.e_tag.unwrap_or_default())
//                             .part_number(part_number)
//                             .build(),
//                     );

//                     offset += CHUNK_SIZE;
//                     part_number += 1;
//                 }
//             }
//             Err(err) => return Err(format!("Error reading chunk: {:#?}", err).into()),
//         }
//     }
    
//     // Complete the multipart upload
//     self.client
//         .complete_multipart_upload()
//         .bucket(&self.bucket)
//         .key("your_object_key")// TODO: Think this should be filename from JSON?
//         .upload_id(upload_id)
//         .multipart_upload(aws_sdk_s3::types::CompletedMultipartUpload::builder() // TODO: Import to tidy up
//             .parts(upload_parts)
//             .build())
//         .send()
//         .await?;

//     Ok(upload_id.to_string())
// }

}
