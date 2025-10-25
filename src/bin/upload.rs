#![allow(clippy::result_large_err)]

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{config::Region, Client};
use std::error::Error;
use std::path::{Path, PathBuf};
use aws_config::BehaviorVersion;

/// Upload a file to an S3 bucket.
///
/// # Arguments
///
/// * `client` - an S3 client configured appropriately for the environment.
/// * `bucket` - the bucket name that the object will be uploaded to.
/// * `filepath` - a reference to a path that will be read and uploaded to S3.
/// * `key` - the string key that the object will be uploaded as inside the bucket.
async fn upload_to_s3(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    filepath: &Path,
    key: &str,
) -> Result<String, Box<dyn Error>> {
    let body = aws_sdk_s3::primitives::ByteStream::from_path(filepath).await?;

    let resp = client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .send()
        .await?;

    Ok(resp
        .version_id()
        .unwrap_or_else(|| "(no version id)")
        .to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Example values - in practice these would come from your application configuration
    let bucket = std::env::var("DO_SPACES_BUCKET").expect("DO_SPACES_BUCKET must be set");
    let filename = PathBuf::from(std::env::args().nth(1).expect("Please provide a file path"));
    let key = filename
        .file_name()
        .expect("Invalid filename")
        .to_str()
        .expect("Invalid UTF-8 in filename")
        .to_string();

    if !filename.exists() {
        return Err("File does not exist".into());
    }

let shared_config  = aws_config::defaults(BehaviorVersion::latest())
.endpoint_url("https://lon1.digitaloceanspaces.com")
    .region("us-east-1")
    .load()
    .await;
    let client = Client::new(&shared_config);

    match upload_to_s3(&client, &bucket, &filename, &key).await {
        Ok(version_id) => {
            println!("Successfully uploaded file. Version ID: {}", version_id);
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to upload file: {}", e);
            Err(e)
        }
    }
}
