use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFileInput {
    pub format: String,
    pub s3_key: String,
    pub checksum: String,
    pub size_bytes: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddVersionFilesCommand {
    pub version_id: Uuid,
    pub files: Vec<VersionFileInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddVersionFilesResponse {
    pub version_id: Uuid,
    pub files_added: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum AddVersionFilesError {
    #[error("Files list cannot be empty")]
    FilesRequired,
    #[error("Format is required and cannot be empty")]
    FormatRequired,
    #[error("Invalid format: {0}. Must be one of: fasta, json, xml, dat, tar.gz")]
    InvalidFormat(String),
    #[error("S3 key is required and cannot be empty")]
    S3KeyRequired,
    #[error("S3 key must not exceed 1000 characters")]
    S3KeyTooLong,
    #[error("Checksum is required and cannot be empty")]
    ChecksumRequired,
    #[error("Checksum must be between 1 and 64 characters")]
    ChecksumLength,
    #[error("Size in bytes must be greater than 0")]
    InvalidSize,
    #[error("Invalid compression format: {0}. Must be one of: gzip, bzip2, none")]
    InvalidCompression(String),
    #[error("Version with ID '{0}' not found")]
    VersionNotFound(Uuid),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<AddVersionFilesResponse, AddVersionFilesError>> for AddVersionFilesCommand {}

impl crate::cqrs::middleware::Command for AddVersionFilesCommand {}

const VALID_FORMATS: &[&str] = &["fasta", "json", "xml", "dat", "tar.gz"];
const VALID_COMPRESSIONS: &[&str] = &["gzip", "bzip2", "none"];

impl VersionFileInput {
    pub fn validate(&self) -> Result<(), AddVersionFilesError> {
        if self.format.trim().is_empty() {
            return Err(AddVersionFilesError::FormatRequired);
        }
        if !VALID_FORMATS.contains(&self.format.as_str()) {
            return Err(AddVersionFilesError::InvalidFormat(self.format.clone()));
        }
        if self.s3_key.trim().is_empty() {
            return Err(AddVersionFilesError::S3KeyRequired);
        }
        if self.s3_key.len() > 1000 {
            return Err(AddVersionFilesError::S3KeyTooLong);
        }
        if self.checksum.trim().is_empty() {
            return Err(AddVersionFilesError::ChecksumRequired);
        }
        if self.checksum.len() > 64 || self.checksum.is_empty() {
            return Err(AddVersionFilesError::ChecksumLength);
        }
        if self.size_bytes <= 0 {
            return Err(AddVersionFilesError::InvalidSize);
        }
        if let Some(ref compression) = self.compression {
            if !VALID_COMPRESSIONS.contains(&compression.as_str()) {
                return Err(AddVersionFilesError::InvalidCompression(compression.clone()));
            }
        }
        Ok(())
    }
}

impl AddVersionFilesCommand {
    pub fn validate(&self) -> Result<(), AddVersionFilesError> {
        if self.files.is_empty() {
            return Err(AddVersionFilesError::FilesRequired);
        }
        for file in &self.files {
            file.validate()?;
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    command: AddVersionFilesCommand,
) -> Result<AddVersionFilesResponse, AddVersionFilesError> {
    command.validate()?;

    // Check if version exists
    let version_exists: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM versions WHERE id = $1)",
        command.version_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(false);

    if !version_exists {
        return Err(AddVersionFilesError::VersionNotFound(command.version_id));
    }

    let mut files_added = 0;

    for file in command.files {
        sqlx::query!(
            r#"
            INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes, compression)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (version_id, format)
            DO UPDATE SET
                s3_key = EXCLUDED.s3_key,
                checksum = EXCLUDED.checksum,
                size_bytes = EXCLUDED.size_bytes,
                compression = EXCLUDED.compression
            "#,
            command.version_id,
            file.format,
            file.s3_key,
            file.checksum,
            file.size_bytes,
            file.compression
        )
        .execute(&pool)
        .await?;

        files_added += 1;
    }

    Ok(AddVersionFilesResponse {
        version_id: command.version_id,
        files_added,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_file_input_validation_success() {
        let file = VersionFileInput {
            format: "fasta".to_string(),
            s3_key: "proteins/uniprot/P01308/1.0/P01308.fasta".to_string(),
            checksum: "abc123".to_string(),
            size_bytes: 1024,
            compression: Some("gzip".to_string()),
        };
        assert!(file.validate().is_ok());
    }

    #[test]
    fn test_version_file_input_validation_minimal() {
        let file = VersionFileInput {
            format: "json".to_string(),
            s3_key: "test.json".to_string(),
            checksum: "a".to_string(),
            size_bytes: 1,
            compression: None,
        };
        assert!(file.validate().is_ok());
    }

    #[test]
    fn test_version_file_input_validation_all_formats() {
        for format in VALID_FORMATS {
            let file = VersionFileInput {
                format: format.to_string(),
                s3_key: "test.file".to_string(),
                checksum: "abc123".to_string(),
                size_bytes: 100,
                compression: None,
            };
            assert!(file.validate().is_ok(), "Format {} should be valid", format);
        }
    }

    #[test]
    fn test_version_file_input_validation_empty_format() {
        let file = VersionFileInput {
            format: "".to_string(),
            s3_key: "test.file".to_string(),
            checksum: "abc123".to_string(),
            size_bytes: 100,
            compression: None,
        };
        assert!(matches!(
            file.validate(),
            Err(AddVersionFilesError::FormatRequired)
        ));
    }

    #[test]
    fn test_version_file_input_validation_invalid_format() {
        let file = VersionFileInput {
            format: "invalid".to_string(),
            s3_key: "test.file".to_string(),
            checksum: "abc123".to_string(),
            size_bytes: 100,
            compression: None,
        };
        assert!(matches!(
            file.validate(),
            Err(AddVersionFilesError::InvalidFormat(_))
        ));
    }

    #[test]
    fn test_version_file_input_validation_empty_s3_key() {
        let file = VersionFileInput {
            format: "fasta".to_string(),
            s3_key: "".to_string(),
            checksum: "abc123".to_string(),
            size_bytes: 100,
            compression: None,
        };
        assert!(matches!(
            file.validate(),
            Err(AddVersionFilesError::S3KeyRequired)
        ));
    }

    #[test]
    fn test_version_file_input_validation_s3_key_too_long() {
        let file = VersionFileInput {
            format: "fasta".to_string(),
            s3_key: "a".repeat(1001),
            checksum: "abc123".to_string(),
            size_bytes: 100,
            compression: None,
        };
        assert!(matches!(
            file.validate(),
            Err(AddVersionFilesError::S3KeyTooLong)
        ));
    }

    #[test]
    fn test_version_file_input_validation_empty_checksum() {
        let file = VersionFileInput {
            format: "fasta".to_string(),
            s3_key: "test.fasta".to_string(),
            checksum: "".to_string(),
            size_bytes: 100,
            compression: None,
        };
        assert!(matches!(
            file.validate(),
            Err(AddVersionFilesError::ChecksumRequired)
        ));
    }

    #[test]
    fn test_version_file_input_validation_checksum_too_long() {
        let file = VersionFileInput {
            format: "fasta".to_string(),
            s3_key: "test.fasta".to_string(),
            checksum: "a".repeat(65),
            size_bytes: 100,
            compression: None,
        };
        assert!(matches!(
            file.validate(),
            Err(AddVersionFilesError::ChecksumLength)
        ));
    }

    #[test]
    fn test_version_file_input_validation_zero_size() {
        let file = VersionFileInput {
            format: "fasta".to_string(),
            s3_key: "test.fasta".to_string(),
            checksum: "abc123".to_string(),
            size_bytes: 0,
            compression: None,
        };
        assert!(matches!(
            file.validate(),
            Err(AddVersionFilesError::InvalidSize)
        ));
    }

    #[test]
    fn test_version_file_input_validation_negative_size() {
        let file = VersionFileInput {
            format: "fasta".to_string(),
            s3_key: "test.fasta".to_string(),
            checksum: "abc123".to_string(),
            size_bytes: -1,
            compression: None,
        };
        assert!(matches!(
            file.validate(),
            Err(AddVersionFilesError::InvalidSize)
        ));
    }

    #[test]
    fn test_version_file_input_validation_invalid_compression() {
        let file = VersionFileInput {
            format: "fasta".to_string(),
            s3_key: "test.fasta".to_string(),
            checksum: "abc123".to_string(),
            size_bytes: 100,
            compression: Some("invalid".to_string()),
        };
        assert!(matches!(
            file.validate(),
            Err(AddVersionFilesError::InvalidCompression(_))
        ));
    }

    #[test]
    fn test_version_file_input_validation_all_compressions() {
        for compression in VALID_COMPRESSIONS {
            let file = VersionFileInput {
                format: "fasta".to_string(),
                s3_key: "test.fasta".to_string(),
                checksum: "abc123".to_string(),
                size_bytes: 100,
                compression: Some(compression.to_string()),
            };
            assert!(
                file.validate().is_ok(),
                "Compression {} should be valid",
                compression
            );
        }
    }

    #[test]
    fn test_command_validation_empty_files() {
        let cmd = AddVersionFilesCommand {
            version_id: Uuid::new_v4(),
            files: vec![],
        };
        assert!(matches!(
            cmd.validate(),
            Err(AddVersionFilesError::FilesRequired)
        ));
    }

    #[test]
    fn test_command_validation_success() {
        let cmd = AddVersionFilesCommand {
            version_id: Uuid::new_v4(),
            files: vec![VersionFileInput {
                format: "fasta".to_string(),
                s3_key: "test.fasta".to_string(),
                checksum: "abc123".to_string(),
                size_bytes: 100,
                compression: None,
            }],
        };
        assert!(cmd.validate().is_ok());
    }

    #[sqlx::test]
    async fn test_handle_adds_version_files(pool: PgPool) -> sqlx::Result<()> {
        // Setup: Create organization, registry entry, data source, and version
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let entry_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO registry_entries (id, organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            "#,
            entry_id,
            org_id,
            "test-ds",
            "Test DS"
        )
        .execute(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id,
            "protein"
        )
        .execute(&pool)
        .await?;

        let version_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO versions (id, data_source_id, version_string, status)
            VALUES ($1, $2, $3, $4)
            "#,
            version_id,
            entry_id,
            "1.0",
            "published"
        )
        .execute(&pool)
        .await?;

        let cmd = AddVersionFilesCommand {
            version_id,
            files: vec![VersionFileInput {
                format: "fasta".to_string(),
                s3_key: "test.fasta".to_string(),
                checksum: "abc123".to_string(),
                size_bytes: 1024,
                compression: Some("gzip".to_string()),
            }],
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.version_id, version_id);
        assert_eq!(response.files_added, 1);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_adds_multiple_files(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let entry_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO registry_entries (id, organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            "#,
            entry_id,
            org_id,
            "test-ds",
            "Test DS"
        )
        .execute(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id,
            "protein"
        )
        .execute(&pool)
        .await?;

        let version_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO versions (id, data_source_id, version_string, status)
            VALUES ($1, $2, $3, $4)
            "#,
            version_id,
            entry_id,
            "1.0",
            "published"
        )
        .execute(&pool)
        .await?;

        let cmd = AddVersionFilesCommand {
            version_id,
            files: vec![
                VersionFileInput {
                    format: "fasta".to_string(),
                    s3_key: "test.fasta".to_string(),
                    checksum: "abc123".to_string(),
                    size_bytes: 1024,
                    compression: Some("gzip".to_string()),
                },
                VersionFileInput {
                    format: "json".to_string(),
                    s3_key: "test.json".to_string(),
                    checksum: "def456".to_string(),
                    size_bytes: 2048,
                    compression: None,
                },
            ],
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.files_added, 2);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_upsert_updates_existing_file(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let entry_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO registry_entries (id, organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            "#,
            entry_id,
            org_id,
            "test-ds",
            "Test DS"
        )
        .execute(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id,
            "protein"
        )
        .execute(&pool)
        .await?;

        let version_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO versions (id, data_source_id, version_string, status)
            VALUES ($1, $2, $3, $4)
            "#,
            version_id,
            entry_id,
            "1.0",
            "published"
        )
        .execute(&pool)
        .await?;

        // First insert
        let cmd1 = AddVersionFilesCommand {
            version_id,
            files: vec![VersionFileInput {
                format: "fasta".to_string(),
                s3_key: "old.fasta".to_string(),
                checksum: "old123".to_string(),
                size_bytes: 500,
                compression: None,
            }],
        };
        handle(pool.clone(), cmd1).await.unwrap();

        // Update with same format
        let cmd2 = AddVersionFilesCommand {
            version_id,
            files: vec![VersionFileInput {
                format: "fasta".to_string(),
                s3_key: "new.fasta".to_string(),
                checksum: "new456".to_string(),
                size_bytes: 1000,
                compression: Some("gzip".to_string()),
            }],
        };
        let result = handle(pool.clone(), cmd2).await;
        assert!(result.is_ok());

        // Verify update
        let file = sqlx::query!(
            "SELECT s3_key, checksum, size_bytes FROM version_files WHERE version_id = $1 AND format = $2",
            version_id,
            "fasta"
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(file.s3_key, "new.fasta");
        assert_eq!(file.checksum, "new456");
        assert_eq!(file.size_bytes, 1000);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_version_not_found(pool: PgPool) -> sqlx::Result<()> {
        let cmd = AddVersionFilesCommand {
            version_id: Uuid::new_v4(),
            files: vec![VersionFileInput {
                format: "fasta".to_string(),
                s3_key: "test.fasta".to_string(),
                checksum: "abc123".to_string(),
                size_bytes: 100,
                compression: None,
            }],
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(matches!(
            result,
            Err(AddVersionFilesError::VersionNotFound(_))
        ));
        Ok(())
    }
}
