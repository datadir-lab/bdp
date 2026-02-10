//! Record download command
//!
//! Records a completed download in the database: inserts into the downloads
//! table, increments the version download count, and creates an audit log entry.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::audit::{self, AuditAction, CreateAuditEntry, ResourceType};

/// Command to record a completed download
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordDownloadCommand {
    /// Organization slug (e.g., "uniprot")
    pub org: String,
    /// Data source name (e.g., "P01308")
    pub name: String,
    /// Version string (e.g., "1.0")
    pub version: String,
    /// File format (e.g., "fasta")
    pub format: String,
    /// Optional client user agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    /// Optional client IP address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
}

/// Response from recording a download
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordDownloadResponse {
    pub recorded: bool,
}

impl crate::cqrs::middleware::Command for RecordDownloadCommand {}

impl mediator::Request<Result<RecordDownloadResponse, RecordDownloadError>>
    for RecordDownloadCommand
{
}

/// Errors from recording a download
#[derive(Debug, thiserror::Error)]
pub enum RecordDownloadError {
    #[error("Source not found: {0}")]
    NotFound(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

/// Handle the record download command
#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    command: RecordDownloadCommand,
) -> Result<RecordDownloadResponse, RecordDownloadError> {
    // Look up version_id and file_id
    let record = sqlx::query!(
        r#"
        SELECT v.id as version_id, vf.id as file_id
        FROM versions v
        JOIN registry_entries re ON re.id = v.entry_id
        JOIN organizations o ON o.id = re.organization_id
        JOIN version_files vf ON vf.version_id = v.id AND vf.format = $4
        WHERE LOWER(o.slug) = LOWER($1)
          AND LOWER(re.slug) = LOWER($2)
          AND v.version = $3
        "#,
        command.org,
        command.name,
        command.version,
        command.format
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| {
        RecordDownloadError::NotFound(format!(
            "{}:{}-{}@{}",
            command.org, command.name, command.format, command.version
        ))
    })?;

    // Parse IP address string to IpNetwork for the INET column
    let ip_addr: Option<ipnetwork::IpNetwork> =
        command.ip_address.as_deref().and_then(|ip| ip.parse().ok());

    // Insert into downloads table
    sqlx::query!(
        r#"
        INSERT INTO downloads (version_id, file_id, user_agent, ip_address)
        VALUES ($1, $2, $3, $4)
        "#,
        record.version_id,
        record.file_id,
        command.user_agent,
        ip_addr as Option<ipnetwork::IpNetwork>,
    )
    .execute(&pool)
    .await?;

    // Increment download count
    sqlx::query!(
        r#"
        UPDATE versions SET download_count = download_count + 1 WHERE id = $1
        "#,
        record.version_id
    )
    .execute(&pool)
    .await?;

    // Create audit log entry (best-effort)
    let audit_entry = CreateAuditEntry::builder()
        .action(AuditAction::Download)
        .resource_type(ResourceType::Download)
        .resource_id(Some(record.version_id))
        .metadata(serde_json::json!({
            "org": command.org,
            "name": command.name,
            "version": command.version,
            "format": command.format,
        }))
        .try_build();

    match audit_entry {
        Ok(entry) => {
            if let Err(e) = audit::create_audit_entry(&pool, entry).await {
                tracing::warn!("Failed to create audit log for download: {}", e);
            }
        },
        Err(e) => {
            tracing::warn!("Failed to build audit entry for download: {}", e);
        },
    }

    tracing::info!(
        org = %command.org,
        name = %command.name,
        version = %command.version,
        format = %command.format,
        "Download recorded"
    );

    Ok(RecordDownloadResponse { recorded: true })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_command_serialization() {
        let cmd = RecordDownloadCommand {
            org: "uniprot".to_string(),
            name: "P01308".to_string(),
            version: "1.0".to_string(),
            format: "fasta".to_string(),
            user_agent: Some("bdp-cli/0.1.0".to_string()),
            ip_address: None,
        };

        let json = serde_json::to_string(&cmd).expect("serialize");
        assert!(json.contains("uniprot"));
        assert!(json.contains("P01308"));
    }

    #[sqlx::test]
    async fn test_handle_not_found(pool: PgPool) -> sqlx::Result<()> {
        let cmd = RecordDownloadCommand {
            org: "nonexistent".to_string(),
            name: "nothing".to_string(),
            version: "0.0".to_string(),
            format: "txt".to_string(),
            user_agent: None,
            ip_address: None,
        };

        let result = handle(pool, cmd).await;
        assert!(matches!(result, Err(RecordDownloadError::NotFound(_))));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_records_download(pool: PgPool) -> sqlx::Result<()> {
        // Seed test data
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
            "INSERT INTO registry_entries (id, organization_id, slug, name, entry_type) VALUES ($1, $2, $3, $4, $5)",
            entry_id, org_id, "test-entry", "Test Entry", "data_source"
        )
        .execute(&pool)
        .await?;

        let version_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO versions (id, entry_id, version, download_count) VALUES ($1, $2, $3, $4)",
            version_id,
            entry_id,
            "1.0",
            0i64
        )
        .execute(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes) VALUES ($1, $2, $3, $4, $5)",
            version_id, "fasta", "test/entry.fasta", "abc123", 100i64
        )
        .execute(&pool)
        .await?;

        let cmd = RecordDownloadCommand {
            org: "test-org".to_string(),
            name: "test-entry".to_string(),
            version: "1.0".to_string(),
            format: "fasta".to_string(),
            user_agent: Some("bdp-cli/0.1.0".to_string()),
            ip_address: None,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        assert!(result.unwrap().recorded);

        // Verify download count was incremented
        let count: i64 =
            sqlx::query_scalar!("SELECT download_count FROM versions WHERE id = $1", version_id)
                .fetch_one(&pool)
                .await?
                .unwrap_or(0);

        assert_eq!(count, 1);

        Ok(())
    }
}
