//! Storage configuration
//!
//! Configuration for S3-compatible storage backends including AWS S3 and MinIO.
//! Supports loading from environment variables or direct construction.

use serde::{Deserialize, Serialize};
use std::env;

// ============================================================================
// Storage Configuration Constants
// ============================================================================

/// Default S3 region when not specified via environment variable.
pub const DEFAULT_S3_REGION: &str = "us-east-1";

/// Default S3 bucket name when not specified via environment variable.
pub const DEFAULT_S3_BUCKET: &str = "bdp-data";

/// Default MinIO access key for local development.
/// In production, this should always be set via environment variable.
pub const DEFAULT_MINIO_ACCESS_KEY: &str = "minioadmin";

/// Default MinIO secret key for local development.
/// In production, this should always be set via environment variable.
pub const DEFAULT_MINIO_SECRET_KEY: &str = "minioadmin";

/// Configuration for S3-compatible storage backends
///
/// Supports both AWS S3 and MinIO (local S3-compatible storage).
/// Configuration can be loaded from environment variables or constructed
/// directly using the helper methods.
///
/// # Environment Variables
///
/// The following environment variables are checked (in order of precedence):
/// - `STORAGE_S3_ENDPOINT` / `S3_ENDPOINT` - Custom endpoint for MinIO/compatible storage
/// - `STORAGE_S3_REGION` / `S3_REGION` - AWS region (default: "us-east-1")
/// - `STORAGE_S3_BUCKET` / `S3_BUCKET` - Bucket name (default: "bdp-data")
/// - `STORAGE_S3_ACCESS_KEY` / `S3_ACCESS_KEY` / `AWS_ACCESS_KEY_ID` - Access key
/// - `STORAGE_S3_SECRET_KEY` / `S3_SECRET_KEY` / `AWS_SECRET_ACCESS_KEY` - Secret key
/// - `STORAGE_S3_PATH_STYLE` / `S3_PATH_STYLE` - Use path-style addressing (for MinIO)
///
/// # Examples
///
/// ```rust,ignore
/// use bdp_server::storage::StorageConfig;
///
/// // For local development with MinIO
/// let config = StorageConfig::for_minio("http://localhost:9000", "bdp-data");
///
/// // For production with AWS S3
/// let config = StorageConfig::for_aws("us-west-2", "my-bucket")?;
///
/// // From environment variables
/// let config = StorageConfig::from_env()?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub endpoint: Option<String>,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub path_style: bool,
}

impl StorageConfig {
    /// Creates a StorageConfig from environment variables
    ///
    /// Reads configuration from environment variables with fallback defaults
    /// suitable for local development with MinIO.
    ///
    /// # Errors
    ///
    /// This function currently does not return errors as all values have defaults.
    /// The Result type is retained for API compatibility and future error handling.
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            endpoint: env::var("STORAGE_S3_ENDPOINT")
                .or_else(|_| env::var("S3_ENDPOINT"))
                .ok(),
            region: env::var("STORAGE_S3_REGION")
                .or_else(|_| env::var("S3_REGION"))
                .unwrap_or_else(|_| DEFAULT_S3_REGION.to_string()),
            bucket: env::var("STORAGE_S3_BUCKET")
                .or_else(|_| env::var("S3_BUCKET"))
                .unwrap_or_else(|_| DEFAULT_S3_BUCKET.to_string()),
            access_key: env::var("STORAGE_S3_ACCESS_KEY")
                .or_else(|_| env::var("S3_ACCESS_KEY"))
                .or_else(|_| env::var("AWS_ACCESS_KEY_ID"))
                .unwrap_or_else(|_| DEFAULT_MINIO_ACCESS_KEY.to_string()),
            secret_key: env::var("STORAGE_S3_SECRET_KEY")
                .or_else(|_| env::var("S3_SECRET_KEY"))
                .or_else(|_| env::var("AWS_SECRET_ACCESS_KEY"))
                .unwrap_or_else(|_| DEFAULT_MINIO_SECRET_KEY.to_string()),
            path_style: env::var("STORAGE_S3_PATH_STYLE")
                .or_else(|_| env::var("S3_PATH_STYLE"))
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
        })
    }

    /// Creates a StorageConfig for local MinIO development
    ///
    /// Configures path-style addressing and default MinIO credentials.
    /// Suitable for local development with a MinIO container.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - MinIO endpoint URL (e.g., "http://localhost:9000")
    /// * `bucket` - Bucket name to use
    pub fn for_minio(endpoint: impl Into<String>, bucket: impl Into<String>) -> Self {
        Self {
            endpoint: Some(endpoint.into()),
            region: DEFAULT_S3_REGION.to_string(),
            bucket: bucket.into(),
            access_key: DEFAULT_MINIO_ACCESS_KEY.to_string(),
            secret_key: DEFAULT_MINIO_SECRET_KEY.to_string(),
            path_style: true,
        }
    }

    /// Create a StorageConfig for AWS S3
    ///
    /// # Errors
    /// Returns an error if AWS_ACCESS_KEY_ID or AWS_SECRET_ACCESS_KEY environment variables are not set
    pub fn for_aws(region: impl Into<String>, bucket: impl Into<String>) -> anyhow::Result<Self> {
        let access_key = env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| anyhow::anyhow!("AWS_ACCESS_KEY_ID environment variable must be set"))?;
        let secret_key = env::var("AWS_SECRET_ACCESS_KEY").map_err(|_| {
            anyhow::anyhow!("AWS_SECRET_ACCESS_KEY environment variable must be set")
        })?;

        Ok(Self {
            endpoint: None,
            region: region.into(),
            bucket: bucket.into(),
            access_key,
            secret_key,
            path_style: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_for_minio() {
        let config = StorageConfig::for_minio("http://localhost:9000", "test-bucket");
        assert_eq!(config.endpoint, Some("http://localhost:9000".to_string()));
        assert_eq!(config.bucket, "test-bucket");
        assert!(config.path_style);
        assert_eq!(config.access_key, DEFAULT_MINIO_ACCESS_KEY);
    }

    #[test]
    fn test_for_aws() {
        std::env::set_var("AWS_ACCESS_KEY_ID", "test_key");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test_secret");

        let config = StorageConfig::for_aws("us-west-2", "my-bucket").unwrap();
        assert_eq!(config.endpoint, None);
        assert_eq!(config.region, "us-west-2");
        assert_eq!(config.bucket, "my-bucket");
        assert!(!config.path_style);
    }
}
