use serde::{Deserialize, Serialize};
use std::env;

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
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            endpoint: env::var("S3_ENDPOINT").ok(),
            region: env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            bucket: env::var("S3_BUCKET").unwrap_or_else(|_| "bdp-data".to_string()),
            access_key: env::var("S3_ACCESS_KEY")
                .or_else(|_| env::var("AWS_ACCESS_KEY_ID"))
                .unwrap_or_else(|_| "minioadmin".to_string()),
            secret_key: env::var("S3_SECRET_KEY")
                .or_else(|_| env::var("AWS_SECRET_ACCESS_KEY"))
                .unwrap_or_else(|_| "minioadmin".to_string()),
            path_style: env::var("S3_PATH_STYLE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
        })
    }

    pub fn for_minio(endpoint: impl Into<String>, bucket: impl Into<String>) -> Self {
        Self {
            endpoint: Some(endpoint.into()),
            region: "us-east-1".to_string(),
            bucket: bucket.into(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            path_style: true,
        }
    }

    pub fn for_aws(region: impl Into<String>, bucket: impl Into<String>) -> Self {
        Self {
            endpoint: None,
            region: region.into(),
            bucket: bucket.into(),
            access_key: env::var("AWS_ACCESS_KEY_ID")
                .expect("AWS_ACCESS_KEY_ID must be set"),
            secret_key: env::var("AWS_SECRET_ACCESS_KEY")
                .expect("AWS_SECRET_ACCESS_KEY must be set"),
            path_style: false,
        }
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
        assert_eq!(config.access_key, "minioadmin");
    }

    #[test]
    fn test_for_aws() {
        std::env::set_var("AWS_ACCESS_KEY_ID", "test_key");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test_secret");

        let config = StorageConfig::for_aws("us-west-2", "my-bucket");
        assert_eq!(config.endpoint, None);
        assert_eq!(config.region, "us-west-2");
        assert_eq!(config.bucket, "my-bucket");
        assert!(!config.path_style);
    }
}
