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
    #[error("Format must not exceed 50 characters")]
    FormatLength,
    #[error("S3 key is required and cannot be empty")]
    S3KeyRequired,
    #[error("Checksum is required and cannot be empty")]
    ChecksumRequired,
    #[error("Checksum must not exceed 64 characters")]
    ChecksumLength,
    #[error("Size bytes must be non-negative")]
    InvalidSize,
    #[error("Compression type must not exceed 20 characters")]
    CompressionLength,
    #[error("Version with ID '{0}' not found")]
    VersionNotFound(Uuid),
    #[error("Duplicate format '{0}' for version")]
    DuplicateFormat(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<AddVersionFilesResponse, AddVersionFilesError>> for AddVersionFilesCommand {}

impl crate::cqrs::middleware::Command for AddVersionFilesCommand {}

impl AddVersionFilesCommand {
    pub fn validate(&self) -> Result<(), AddVersionFilesError> {
        if self.files.is_empty() {
            return Err(AddVersionFilesError::FilesRequired);
        }

        for file in &self.files {
            if file.format.trim().is_empty() {
                return Err(AddVersionFilesError::FormatRequired);
            }
            if file.format.len() > 50 {
                return Err(AddVersionFilesError::FormatLength);
            }
            if file.s3_key.trim().is_empty() {
                return Err(AddVersionFilesError::S3KeyRequired);
            }
            if file.checksum.trim().is_empty() {
                return Err(AddVersionFilesError::ChecksumRequired);
            }
            if file.checksum.len() > 64 {
                return Err(AddVersionFilesError::ChecksumLength);
            }
            if file.size_bytes < 0 {
                return Err(AddVersionFilesError::InvalidSize);
            }
            if let Some(ref compression) = file.compression {
                if compression.len() > 20 {
                    return Err(AddVersionFilesError::CompressionLength);
                }
            }
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
    let version_exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM versions WHERE id = $1
        )
        "#,
        command.version_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(false);

    if !version_exists {
        return Err(AddVersionFilesError::VersionNotFound(command.version_id));
    }

    let mut tx = pool.begin().await?;
    let mut files_added = 0;

    for file in &command.files {
        // Insert or update version file
        sqlx::query!(
            r#"
            INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes, compression)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (version_id, format) DO UPDATE SET
                s3_key = EXCLUDED.s3_key,
                checksum = EXCLUDED.checksum,
                size_bytes = EXCLUDED.size_bytes,
                compression = EXCLUDED.compression,
                created_at = NOW()
            "#,
            command.version_id,
            file.format,
            file.s3_key,
            file.checksum,
            file.size_bytes,
            file.compression
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.is_foreign_key_violation() {
                    return AddVersionFilesError::VersionNotFound(command.version_id);
                }
            }
            AddVersionFilesError::Database(e)
        })?;

        files_added += 1;
    }

    tx.commit().await?;

    Ok(AddVersionFilesResponse {
        version_id: command.version_id,
        files_added,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let cmd = AddVersionFilesCommand {
            version_id: Uuid::new_v4(),
            files: vec![
                VersionFileInput {
                    format: "fasta".to_string(),
                    s3_key: "data-sources/uniprot/p01308/1.0/P01308.fasta".to_string(),
                    checksum: "abc123".to_string(),
                    size_bytes: 1024,
                    compression: None,
                },
                VersionFileInput {
                    format: "json".to_string(),
                    s3_key: "data-sources/uniprot/p01308/1.0/P01308.json".to_string(),
                    checksum: "def456".to_string(),
                    size_bytes: 2048,
                    compression: Some("gzip".to_string()),
                },
            ],
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_files() {
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
    fn test_validation_empty_format() {
        let cmd = AddVersionFilesCommand {
            version_id: Uuid::new_v4(),
            files: vec![VersionFileInput {
                format: "".to_string(),
                s3_key: "key".to_string(),
                checksum: "checksum".to_string(),
                size_bytes: 100,
                compression: None,
            }],
        };
        assert!(matches!(
            cmd.validate(),
            Err(AddVersionFilesError::FormatRequired)
        ));
    }

    #[test]
    fn test_validation_negative_size() {
        let cmd = AddVersionFilesCommand {
            version_id: Uuid::new_v4(),
            files: vec![VersionFileInput {
                format: "fasta".to_string(),
                s3_key: "key".to_string(),
                checksum: "checksum".to_string(),
                size_bytes: -100,
                compression: None,
            }],
        };
        assert!(matches!(
            cmd.validate(),
            Err(AddVersionFilesError::InvalidSize)
        ));
    }

    #[sqlx::test]
    async fn test_handle_adds_version_files(pool: PgPool) -> sqlx::Result<()> {
        // Setup: create org, entry, data source, and version
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "uniprot",
            "UniProt",
            true
        )
        .execute(&pool)
        .await?;

        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "p01308",
            "Insulin"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id,
            "protein"
        )
        .execute(&pool)
        .await?;

        let version_id = sqlx::query_scalar!(
            r#"
            INSERT INTO versions (entry_id, version, external_version)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            entry_id,
            "1.0",
            Some("2024_01")
        )
        .fetch_one(&pool)
        .await?;

        let cmd = AddVersionFilesCommand {
            version_id,
            files: vec![
                VersionFileInput {
                    format: "fasta".to_string(),
                    s3_key: "data-sources/uniprot/p01308/1.0/P01308.fasta".to_string(),
                    checksum: "abc123".to_string(),
                    size_bytes: 1024,
                    compression: None,
                },
                VersionFileInput {
                    format: "json".to_string(),
                    s3_key: "data-sources/uniprot/p01308/1.0/P01308.json".to_string(),
                    checksum: "def456".to_string(),
                    size_bytes: 2048,
                    compression: Some("gzip".to_string()),
                },
            ],
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.files_added, 2);

        // Verify files were added
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM version_files WHERE version_id = $1",
            version_id
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(count, Some(2));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_version_not_found(pool: PgPool) -> sqlx::Result<()> {
        let cmd = AddVersionFilesCommand {
            version_id: Uuid::new_v4(),
            files: vec![VersionFileInput {
                format: "fasta".to_string(),
                s3_key: "key".to_string(),
                checksum: "checksum".to_string(),
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

    #[sqlx::test]
    async fn test_handle_upserts_on_conflict(pool: PgPool) -> sqlx::Result<()> {
        // Setup
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "uniprot",
            "UniProt",
            true
        )
        .execute(&pool)
        .await?;

        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "p01308",
            "Insulin"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id,
            "protein"
        )
        .execute(&pool)
        .await?;

        let version_id = sqlx::query_scalar!(
            r#"
            INSERT INTO versions (entry_id, version, external_version)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            entry_id,
            "1.0",
            Some("2024_01")
        )
        .fetch_one(&pool)
        .await?;

        // First insert
        let cmd1 = AddVersionFilesCommand {
            version_id,
            files: vec![VersionFileInput {
                format: "fasta".to_string(),
                s3_key: "old_key".to_string(),
                checksum: "old_checksum".to_string(),
                size_bytes: 1000,
                compression: None,
            }],
        };
        let _ = handle(pool.clone(), cmd1).await.unwrap();

        // Update with new data
        let cmd2 = AddVersionFilesCommand {
            version_id,
            files: vec![VersionFileInput {
                format: "fasta".to_string(),
                s3_key: "new_key".to_string(),
                checksum: "new_checksum".to_string(),
                size_bytes: 2000,
                compression: Some("gzip".to_string()),
            }],
        };
        let result = handle(pool.clone(), cmd2).await;
        assert!(result.is_ok());

        // Verify update
        let record = sqlx::query!(
            "SELECT s3_key, checksum, size_bytes FROM version_files WHERE version_id = $1 AND format = 'fasta'",
            version_id
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(record.s3_key, "new_key");
        assert_eq!(record.checksum, "new_checksum");
        assert_eq!(record.size_bytes, 2000);

        // Should still have only 1 record
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM version_files WHERE version_id = $1",
            version_id
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(count, Some(1));
        Ok(())
    }
}
