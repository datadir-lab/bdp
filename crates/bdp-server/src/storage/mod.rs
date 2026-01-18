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

#[derive(Clone)]
pub struct Storage {
    client: Client,
    bucket: String,
}

impl Storage {
    pub async fn new(config: config::StorageConfig) -> Result<Self> {
        debug!("Initializing storage with config: {:?}", config);

        let credentials = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "bdp-storage",
        );

        let mut s3_config_builder = aws_sdk_s3::Config::builder()
            .credentials_provider(credentials)
            .region(Region::new(config.region.clone()))
            .force_path_style(config.path_style);

        if let Some(endpoint) = &config.endpoint {
            s3_config_builder = s3_config_builder.endpoint_url(endpoint);
        }

        let s3_config = s3_config_builder.build();
        let client = Client::from_conf(s3_config);

        info!(
            "Storage client initialized for bucket: {}",
            config.bucket
        );

        Ok(Self {
            client,
            bucket: config.bucket,
        })
    }

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

    #[instrument(skip(self, stream))]
    pub async fn upload_stream(
        &self,
        key: &str,
        stream: ByteStream,
        content_type: Option<String>,
        size_hint: Option<i64>,
    ) -> Result<String> {
        debug!(
            "Uploading stream to s3://{}/{} (size hint: {:?})",
            self.bucket, key, size_hint
        );

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

        request.send().await.context("Failed to upload stream to S3")?;

        info!(
            "Successfully uploaded stream to s3://{}/{}",
            self.bucket, key
        );

        Ok(key.to_string())
    }

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
            }
        }
    }

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

    #[instrument(skip(self))]
    pub async fn generate_presigned_url(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> Result<String> {
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

    #[instrument(skip(self))]
    pub async fn list(&self, prefix: &str, max_keys: Option<i32>) -> Result<Vec<String>> {
        debug!(
            "Listing objects in s3://{}/{} (max: {:?})",
            self.bucket, prefix, max_keys
        );

        let mut request = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix);

        if let Some(max) = max_keys {
            request = request.max_keys(max);
        }

        let response = request
            .send()
            .await
            .context("Failed to list S3 objects")?;

        let keys = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(|k| k.to_string()))
            .collect();

        Ok(keys)
    }

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

    pub fn build_key(&self, org: &str, name: &str, version: &str, filename: &str) -> String {
        format!("data-sources/{}/{}/{}/{}", org, name, version, filename)
    }

    pub fn build_tool_key(&self, org: &str, name: &str, version: &str, filename: &str) -> String {
        format!("tools/{}/{}/{}/{}", org, name, version, filename)
    }
}

#[derive(Debug, Clone)]
pub struct UploadResult {
    pub key: String,
    pub checksum: String,
    pub size: i64,
}

#[derive(Debug, Clone)]
pub struct ObjectMetadata {
    pub key: String,
    pub size: i64,
    pub content_type: Option<String>,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
}

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
        assert_eq!(
            key,
            "data-sources/uniprot/human-insulin/1.0.0/data.fasta"
        );
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
        assert_eq!(
            checksum,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }
}
