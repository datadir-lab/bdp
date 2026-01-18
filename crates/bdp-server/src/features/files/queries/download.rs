use crate::storage::Storage;
use mediator::Request;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFileQuery {
    pub org: String,
    pub name: String,
    pub version: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFileResponse {
    pub presigned_url: String,
    pub expires_in: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadFileError {
    #[error("Organization name is required and cannot be empty")]
    OrgRequired,
    #[error("Data source name is required and cannot be empty")]
    NameRequired,
    #[error("Version is required and cannot be empty")]
    VersionRequired,
    #[error("Filename is required and cannot be empty")]
    FilenameRequired,
    #[error("File not found")]
    NotFound,
    #[error("Storage error: {0}")]
    Storage(#[from] anyhow::Error),
}

impl Request<Result<DownloadFileResponse, DownloadFileError>> for DownloadFileQuery {}

impl crate::cqrs::middleware::Query for DownloadFileQuery {}

impl DownloadFileQuery {
    pub fn validate(&self) -> Result<(), DownloadFileError> {
        if self.org.trim().is_empty() {
            return Err(DownloadFileError::OrgRequired);
        }
        if self.name.trim().is_empty() {
            return Err(DownloadFileError::NameRequired);
        }
        if self.version.trim().is_empty() {
            return Err(DownloadFileError::VersionRequired);
        }
        if self.filename.trim().is_empty() {
            return Err(DownloadFileError::FilenameRequired);
        }
        Ok(())
    }
}

#[tracing::instrument(skip(storage))]
pub async fn handle(
    storage: Storage,
    query: DownloadFileQuery,
) -> Result<DownloadFileResponse, DownloadFileError> {
    query.validate()?;

    let key = storage.build_key(&query.org, &query.name, &query.version, &query.filename);

    let exists = storage.exists(&key).await?;
    if !exists {
        return Err(DownloadFileError::NotFound);
    }

    let expires_in = 3600u64;
    let presigned_url = storage
        .generate_presigned_url(&key, Duration::from_secs(expires_in))
        .await?;

    Ok(DownloadFileResponse {
        presigned_url,
        expires_in,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let query = DownloadFileQuery {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_org() {
        let query = DownloadFileQuery {
            org: "".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
        };
        assert!(matches!(
            query.validate(),
            Err(DownloadFileError::OrgRequired)
        ));
    }

    #[test]
    fn test_validation_empty_name() {
        let query = DownloadFileQuery {
            org: "uniprot".to_string(),
            name: "".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
        };
        assert!(matches!(
            query.validate(),
            Err(DownloadFileError::NameRequired)
        ));
    }

    #[test]
    fn test_validation_empty_version() {
        let query = DownloadFileQuery {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "".to_string(),
            filename: "data.fasta".to_string(),
        };
        assert!(matches!(
            query.validate(),
            Err(DownloadFileError::VersionRequired)
        ));
    }

    #[test]
    fn test_validation_empty_filename() {
        let query = DownloadFileQuery {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "".to_string(),
        };
        assert!(matches!(
            query.validate(),
            Err(DownloadFileError::FilenameRequired)
        ));
    }
}
