//! S3-compatible storage operations
//!
//! Provides a high-level interface for interacting with S3-compatible storage
//! backends (AWS S3, MinIO, etc.). Used for storing data source files, tool
//! binaries, and ingestion artifacts.
//!
//! # Overview
//!
//! The [`Storage`] struct provides methods for:
//! - Uploading files and streams
//! - Downloading files and streams
//! - Generating presigned URLs for direct client access
//! - Listing and managing objects
//!
//! # Key Path Conventions
//!
//! Data source files: `data-sources/{org}/{name}/{version}/{filename}`
//! Tool files: `tools/{org}/{name}/{version}/{filename}`

use anyhow::{anyhow, Context, Result};
use aws_sdk_s3::{
    config::{Credentials, Region},
    presigning::PresigningConfig,
    primitives::ByteStream,
    Client,
};
use std::time::Duration;
use tracing::{debug, info, instrument};

pub mod config;

/// S3-compatible storage client
///
/// Wraps the AWS S3 SDK client with convenience methods for BDP operations.
/// Thread-safe and clonable for use across async tasks.
#[derive(Clone)]
pub struct Storage {
    client: Client,
    bucket: String,
}

impl Storage {
    /// Creates a new Storage instance from configuration
    ///
    /// Initializes the S3 client with the provided credentials and endpoint.
    /// The client is configured for either AWS S3 or MinIO based on the
    /// endpoint and path_style settings.
    ///
    /// # Errors
    ///
    /// Returns an error if the S3 client configuration is invalid.
    pub async fn new(config: config::StorageConfig) -> Result<Self> {
        debug!("Initializing storage with config: {:?}", config);

        let credentials =
            Credentials::new(&config.access_key, &config.secret_key, None, None, "bdp-storage");

        let mut s3_config_builder = aws_sdk_s3::Config::builder()
            .credentials_provider(credentials)
            .region(Region::new(config.region.clone()))
            .force_path_style(config.path_style);

        if let Some(endpoint) = &config.endpoint {
            s3_config_builder = s3_config_builder.endpoint_url(endpoint);
        }

        let s3_config = s3_config_builder.build();
        let client = Client::from_conf(s3_config);

        info!("Storage client initialized for bucket: {}", config.bucket);

        Ok(Self {
            client,
            bucket: config.bucket,
        })
    }

    /// Uploads data to S3 and returns upload metadata
    ///
    /// Computes a SHA-256 checksum of the data and uploads it to the
    /// specified key. Returns the key, checksum, and size on success.
    ///
    /// # Arguments
    ///
    /// * `key` - S3 object key (path within the bucket)
    /// * `data` - File content to upload
    /// * `content_type` - Optional MIME content type
    ///
    /// # Errors
    ///
    /// Returns an error if the upload fails.
    #[instrument(skip(self, data))]
    pub async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<String>,
    ) -> Result<UploadResult> {
        let checksum = calculate_sha256(&data);
        let size = data.len() as i64;

        debug!("Uploading {} bytes to s3://{}/{}", size, self.bucket, key);

        let mut request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(data));

        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }

        request.send().await.context("Failed to upload to S3")?;

        info!("Successfully uploaded to s3://{}/{}", self.bucket, key);

        Ok(UploadResult {
            key: key.to_string(),
            checksum,
            size,
        })
    }

    /// Uploads a stream to S3 for large files
    ///
    /// Suitable for large files that shouldn't be loaded entirely into memory.
    /// Returns the object key on success.
    ///
    /// # Arguments
    ///
    /// * `key` - S3 object key
    /// * `stream` - ByteStream of data to upload
    /// * `content_type` - Optional MIME content type
    /// * `size_hint` - Optional content length hint
    ///
    /// # Errors
    ///
    /// Returns an error if the upload fails.
    #[instrument(skip(self, stream))]
    pub async fn upload_stream(
        &self,
        key: &str,
        stream: ByteStream,
        content_type: Option<String>,
        size_hint: Option<i64>,
    ) -> Result<String> {
        debug!("Uploading stream to s3://{}/{} (size hint: {:?})", self.bucket, key, size_hint);

        let mut request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(stream);

        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }

        if let Some(size) = size_hint {
            request = request.content_length(size);
        }

        request
            .send()
            .await
            .context("Failed to upload stream to S3")?;

        info!("Successfully uploaded stream to s3://{}/{}", self.bucket, key);

        Ok(key.to_string())
    }

    /// Downloads an object from S3 into memory
    ///
    /// Suitable for small to medium files. For large files, use
    /// [`download_stream`] instead.
    ///
    /// # Errors
    ///
    /// Returns an error if the object doesn't exist or download fails.
    #[instrument(skip(self))]
    pub async fn download(&self, key: &str) -> Result<Vec<u8>> {
        debug!("Downloading from s3://{}/{}", self.bucket, key);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context(format!("Failed to download from S3: {}", key))?;

        let data = response
            .body
            .collect()
            .await
            .context("Failed to read S3 response body")?
            .into_bytes()
            .to_vec();

        debug!("Downloaded {} bytes from s3://{}/{}", data.len(), self.bucket, key);

        Ok(data)
    }

    /// Downloads an object from S3 as a stream
    ///
    /// Suitable for large files that shouldn't be loaded entirely into memory.
    ///
    /// # Errors
    ///
    /// Returns an error if the object doesn't exist or download fails.
    #[instrument(skip(self))]
    pub async fn download_stream(&self, key: &str) -> Result<ByteStream> {
        debug!("Getting stream from s3://{}/{}", self.bucket, key);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context(format!("Failed to get stream from S3: {}", key))?;

        Ok(response.body)
    }

    /// Deletes an object from S3
    ///
    /// # Errors
    ///
    /// Returns an error if the deletion fails.
    #[instrument(skip(self))]
    pub async fn delete(&self, key: &str) -> Result<()> {
        debug!("Deleting s3://{}/{}", self.bucket, key);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context(format!("Failed to delete from S3: {}", key))?;

        info!("Successfully deleted s3://{}/{}", self.bucket, key);

        Ok(())
    }

    /// Checks if an object exists in S3
    ///
    /// Uses HEAD request to check existence without downloading the object.
    ///
    /// # Returns
    ///
    /// Returns `true` if the object exists, `false` if not found.
    ///
    /// # Errors
    ///
    /// Returns an error if the check fails for reasons other than "not found".
    #[instrument(skip(self))]
    pub async fn exists(&self, key: &str) -> Result<bool> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("NotFound") || e.to_string().contains("404") {
                    Ok(false)
                } else {
                    Err(anyhow!("Failed to check S3 object existence: {}", e))
                }
            },
        }
    }

    /// Gets metadata for an object without downloading it
    ///
    /// Returns size, content type, and last modified timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if the object doesn't exist or metadata retrieval fails.
    #[instrument(skip(self))]
    pub async fn get_metadata(&self, key: &str) -> Result<ObjectMetadata> {
        debug!("Getting metadata for s3://{}/{}", self.bucket, key);

        let response = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context(format!("Failed to get metadata from S3: {}", key))?;

        Ok(ObjectMetadata {
            key: key.to_string(),
            size: response.content_length().unwrap_or(0),
            content_type: response.content_type().map(|s| s.to_string()),
            last_modified: response
                .last_modified()
                .and_then(|dt| chrono::DateTime::parse_from_rfc3339(&dt.to_string()).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
        })
    }

    /// Generates a presigned URL for temporary access
    ///
    /// Creates a URL that allows direct download without authentication
    /// for the specified duration.
    ///
    /// # Arguments
    ///
    /// * `key` - S3 object key
    /// * `expires_in` - How long the URL should be valid
    ///
    /// # Returns
    ///
    /// A presigned URL string.
    ///
    /// # Errors
    ///
    /// Returns an error if URL generation fails.
    #[instrument(skip(self))]
    pub async fn generate_presigned_url(&self, key: &str, expires_in: Duration) -> Result<String> {
        debug!(
            "Generating presigned URL for s3://{}/{} (expires in: {:?})",
            self.bucket, key, expires_in
        );

        let presigning_config = PresigningConfig::expires_in(expires_in)
            .context("Failed to create presigning config")?;

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning_config)
            .await
            .context("Failed to generate presigned URL")?;

        let url = presigned_request.uri().to_string();

        debug!("Generated presigned URL: {}", url);

        Ok(url)
    }

    /// Lists objects with a given prefix
    ///
    /// # Arguments
    ///
    /// * `prefix` - Key prefix to filter by
    /// * `max_keys` - Maximum number of keys to return (optional)
    ///
    /// # Returns
    ///
    /// A vector of object keys matching the prefix.
    ///
    /// # Errors
    ///
    /// Returns an error if listing fails.
    #[instrument(skip(self))]
    pub async fn list(&self, prefix: &str, max_keys: Option<i32>) -> Result<Vec<String>> {
        debug!("Listing objects in s3://{}/{} (max: {:?})", self.bucket, prefix, max_keys);

        let mut request = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix);

        if let Some(max) = max_keys {
            request = request.max_keys(max);
        }

        let response = request.send().await.context("Failed to list S3 objects")?;

        let keys = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(|k| k.to_string()))
            .collect();

        Ok(keys)
    }

    /// Copies an object within the same bucket
    ///
    /// # Arguments
    ///
    /// * `source_key` - Source object key
    /// * `dest_key` - Destination object key
    ///
    /// # Errors
    ///
    /// Returns an error if the copy fails.
    #[instrument(skip(self))]
    pub async fn copy(&self, source_key: &str, dest_key: &str) -> Result<()> {
        debug!(
            "Copying s3://{}/{} to s3://{}/{}",
            self.bucket, source_key, self.bucket, dest_key
        );

        let copy_source = format!("{}/{}", self.bucket, source_key);

        self.client
            .copy_object()
            .bucket(&self.bucket)
            .copy_source(&copy_source)
            .key(dest_key)
            .send()
            .await
            .context("Failed to copy S3 object")?;

        info!(
            "Successfully copied s3://{}/{} to s3://{}/{}",
            self.bucket, source_key, self.bucket, dest_key
        );

        Ok(())
    }

    /// Builds an S3 key for a data source file
    ///
    /// Returns: `data-sources/{org}/{name}/{version}/{filename}`
    pub fn build_key(&self, org: &str, name: &str, version: &str, filename: &str) -> String {
        format!("data-sources/{}/{}/{}/{}", org, name, version, filename)
    }

    /// Builds an S3 key for a tool file
    ///
    /// Returns: `tools/{org}/{name}/{version}/{filename}`
    pub fn build_tool_key(&self, org: &str, name: &str, version: &str, filename: &str) -> String {
        format!("tools/{}/{}/{}/{}", org, name, version, filename)
    }
}

/// Result of a successful upload operation
#[derive(Debug, Clone)]
pub struct UploadResult {
    /// S3 object key
    pub key: String,
    /// SHA-256 checksum of the uploaded data
    pub checksum: String,
    /// Size of the uploaded data in bytes
    pub size: i64,
}

/// Metadata for an S3 object
#[derive(Debug, Clone)]
pub struct ObjectMetadata {
    /// S3 object key
    pub key: String,
    /// Size in bytes
    pub size: i64,
    /// MIME content type (if set)
    pub content_type: Option<String>,
    /// Last modification timestamp
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
}

/// Calculates SHA-256 checksum for data verification
fn calculate_sha256(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_key() {
        let storage = Storage {
            client: Client::from_conf(aws_sdk_s3::Config::builder().build()),
            bucket: "test-bucket".to_string(),
        };

        let key = storage.build_key("uniprot", "human-insulin", "1.0.0", "data.fasta");
        assert_eq!(key, "data-sources/uniprot/human-insulin/1.0.0/data.fasta");
    }

    #[test]
    fn test_build_tool_key() {
        let storage = Storage {
            client: Client::from_conf(aws_sdk_s3::Config::builder().build()),
            bucket: "test-bucket".to_string(),
        };

        let key = storage.build_tool_key("ncbi", "blast", "2.14.0", "blast-linux.tar.gz");
        assert_eq!(key, "tools/ncbi/blast/2.14.0/blast-linux.tar.gz");
    }

    #[test]
    fn test_calculate_sha256() {
        let data = b"Hello, World!";
        let checksum = calculate_sha256(data);
        assert_eq!(checksum, "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");
    }
}
