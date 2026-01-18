use crate::storage::Storage;
use mediator::Request;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadFileCommand {
    pub org: String,
    pub name: String,
    pub version: String,
    pub filename: String,
    #[serde(skip)]
    pub content: Vec<u8>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadFileResponse {
    pub key: String,
    pub checksum: String,
    pub size: i64,
    pub presigned_url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum UploadFileError {
    #[error("Organization name is required and cannot be empty")]
    OrgRequired,
    #[error("Data source name is required and cannot be empty")]
    NameRequired,
    #[error("Version is required and cannot be empty")]
    VersionRequired,
    #[error("Filename is required and cannot be empty")]
    FilenameRequired,
    #[error("Filename must not exceed 255 characters")]
    FilenameLength,
    #[error("Content is required and cannot be empty")]
    ContentRequired,
    #[error("Storage error: {0}")]
    Storage(#[from] anyhow::Error),
}

impl Request<Result<UploadFileResponse, UploadFileError>> for UploadFileCommand {}

impl crate::cqrs::middleware::Command for UploadFileCommand {}

impl UploadFileCommand {
    pub fn validate(&self) -> Result<(), UploadFileError> {
        if self.org.trim().is_empty() {
            return Err(UploadFileError::OrgRequired);
        }
        if self.name.trim().is_empty() {
            return Err(UploadFileError::NameRequired);
        }
        if self.version.trim().is_empty() {
            return Err(UploadFileError::VersionRequired);
        }
        if self.filename.trim().is_empty() {
            return Err(UploadFileError::FilenameRequired);
        }
        if self.filename.len() > 255 {
            return Err(UploadFileError::FilenameLength);
        }
        if self.content.is_empty() {
            return Err(UploadFileError::ContentRequired);
        }
        Ok(())
    }
}

#[tracing::instrument(skip(storage, command))]
pub async fn handle(
    storage: Storage,
    command: UploadFileCommand,
) -> Result<UploadFileResponse, UploadFileError> {
    command.validate()?;

    let key = storage.build_key(&command.org, &command.name, &command.version, &command.filename);

    let upload_result = storage
        .upload(&key, command.content, command.content_type)
        .await?;

    let presigned_url = storage
        .generate_presigned_url(&key, Duration::from_secs(3600))
        .await?;

    Ok(UploadFileResponse {
        key: upload_result.key,
        checksum: upload_result.checksum,
        size: upload_result.size,
        presigned_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let cmd = UploadFileCommand {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
            content: vec![1, 2, 3],
            content_type: Some("application/octet-stream".to_string()),
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_org() {
        let cmd = UploadFileCommand {
            org: "".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
            content: vec![1, 2, 3],
            content_type: None,
        };
        assert!(matches!(cmd.validate(), Err(UploadFileError::OrgRequired)));
    }

    #[test]
    fn test_validation_empty_name() {
        let cmd = UploadFileCommand {
            org: "uniprot".to_string(),
            name: "".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
            content: vec![1, 2, 3],
            content_type: None,
        };
        assert!(matches!(cmd.validate(), Err(UploadFileError::NameRequired)));
    }

    #[test]
    fn test_validation_empty_version() {
        let cmd = UploadFileCommand {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "".to_string(),
            filename: "data.fasta".to_string(),
            content: vec![1, 2, 3],
            content_type: None,
        };
        assert!(matches!(cmd.validate(), Err(UploadFileError::VersionRequired)));
    }

    #[test]
    fn test_validation_empty_filename() {
        let cmd = UploadFileCommand {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "".to_string(),
            content: vec![1, 2, 3],
            content_type: None,
        };
        assert!(matches!(cmd.validate(), Err(UploadFileError::FilenameRequired)));
    }

    #[test]
    fn test_validation_filename_too_long() {
        let cmd = UploadFileCommand {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "a".repeat(256),
            content: vec![1, 2, 3],
            content_type: None,
        };
        assert!(matches!(cmd.validate(), Err(UploadFileError::FilenameLength)));
    }

    #[test]
    fn test_validation_empty_content() {
        let cmd = UploadFileCommand {
            org: "uniprot".to_string(),
            name: "human-proteins".to_string(),
            version: "1.0.0".to_string(),
            filename: "data.fasta".to_string(),
            content: vec![],
            content_type: None,
        };
        assert!(matches!(cmd.validate(), Err(UploadFileError::ContentRequired)));
    }
}
