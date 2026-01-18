use chrono::{DateTime, NaiveDate, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishVersionCommand {
    pub data_source_id: Uuid,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishVersionResponse {
    pub id: Uuid,
    pub entry_id: Uuid,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    pub download_count: i64,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum PublishVersionError {
    #[error("Version is required and cannot be empty")]
    VersionRequired,
    #[error("Version must be between 1 and 64 characters")]
    VersionLength,
    #[error("Data source with ID '{0}' not found")]
    DataSourceNotFound(Uuid),
    #[error("Version '{1}' already exists for data source '{0}'")]
    DuplicateVersion(Uuid, String),
    #[error("Size bytes must be non-negative")]
    InvalidSize,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<PublishVersionResponse, PublishVersionError>> for PublishVersionCommand {}

impl crate::cqrs::middleware::Command for PublishVersionCommand {}

impl PublishVersionCommand {
    pub fn validate(&self) -> Result<(), PublishVersionError> {
        if self.version.is_empty() {
            return Err(PublishVersionError::VersionRequired);
        }
        if self.version.len() > 64 {
            return Err(PublishVersionError::VersionLength);
        }
        if let Some(size) = self.size_bytes {
            if size < 0 {
                return Err(PublishVersionError::InvalidSize);
            }
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    command: PublishVersionCommand,
) -> Result<PublishVersionResponse, PublishVersionError> {
    command.validate()?;

    let data_source_exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM registry_entries re
            JOIN data_sources ds ON re.id = ds.id
            WHERE re.id = $1
        )
        "#,
        command.data_source_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(false);

    if !data_source_exists {
        return Err(PublishVersionError::DataSourceNotFound(
            command.data_source_id,
        ));
    }

    let result = sqlx::query_as!(
        VersionRecord,
        r#"
        INSERT INTO versions (
            entry_id, version, external_version, release_date,
            size_bytes, additional_metadata, download_count
        )
        VALUES ($1, $2, $3, $4, $5, $6, 0)
        RETURNING id, entry_id, version, external_version, release_date,
                  size_bytes, download_count as "download_count!",
                  published_at as "published_at!", updated_at as "updated_at!"
        "#,
        command.data_source_id,
        command.version,
        command.external_version,
        command.release_date,
        command.size_bytes,
        command.additional_metadata
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.is_unique_violation() {
                return PublishVersionError::DuplicateVersion(
                    command.data_source_id,
                    command.version.clone(),
                );
            }
        }
        PublishVersionError::Database(e)
    })?;

    Ok(PublishVersionResponse {
        id: result.id,
        entry_id: result.entry_id,
        version: result.version,
        external_version: result.external_version,
        release_date: result.release_date,
        size_bytes: result.size_bytes,
        download_count: result.download_count,
        published_at: result.published_at,
    })
}

#[derive(Debug)]
struct VersionRecord {
    id: Uuid,
    entry_id: Uuid,
    version: String,
    external_version: Option<String>,
    release_date: Option<NaiveDate>,
    size_bytes: Option<i64>,
    download_count: i64,
    published_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let cmd = PublishVersionCommand {
            data_source_id: Uuid::new_v4(),
            version: "1.0".to_string(),
            external_version: Some("2025_01".to_string()),
            release_date: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            size_bytes: Some(1024),
            additional_metadata: None,
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_version() {
        let cmd = PublishVersionCommand {
            data_source_id: Uuid::new_v4(),
            version: "".to_string(),
            external_version: None,
            release_date: None,
            size_bytes: None,
            additional_metadata: None,
        };
        assert!(matches!(
            cmd.validate(),
            Err(PublishVersionError::VersionRequired)
        ));
    }

    #[test]
    fn test_validation_invalid_size() {
        let cmd = PublishVersionCommand {
            data_source_id: Uuid::new_v4(),
            version: "1.0".to_string(),
            external_version: None,
            release_date: None,
            size_bytes: Some(-100),
            additional_metadata: None,
        };
        assert!(matches!(
            cmd.validate(),
            Err(PublishVersionError::InvalidSize)
        ));
    }

    #[sqlx::test]
    async fn test_handle_publishes_version(pool: PgPool) -> sqlx::Result<()> {
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

        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "test-protein",
            "Test Protein"
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

        let cmd = PublishVersionCommand {
            data_source_id: entry_id,
            version: "1.0".to_string(),
            external_version: Some("2025_01".to_string()),
            release_date: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            size_bytes: Some(2048),
            additional_metadata: None,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.version, "1.0");
        assert_eq!(response.external_version, Some("2025_01".to_string()));
        assert_eq!(response.size_bytes, Some(2048));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_duplicate_version(pool: PgPool) -> sqlx::Result<()> {
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

        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "test-protein",
            "Test Protein"
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

        let cmd1 = PublishVersionCommand {
            data_source_id: entry_id,
            version: "1.0".to_string(),
            external_version: None,
            release_date: None,
            size_bytes: None,
            additional_metadata: None,
        };
        let _ = handle(pool.clone(), cmd1).await.unwrap();

        let cmd2 = PublishVersionCommand {
            data_source_id: entry_id,
            version: "1.0".to_string(),
            external_version: None,
            release_date: None,
            size_bytes: None,
            additional_metadata: None,
        };
        let result = handle(pool.clone(), cmd2).await;
        assert!(matches!(
            result,
            Err(PublishVersionError::DuplicateVersion(_, _))
        ));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_data_source_not_found(pool: PgPool) -> sqlx::Result<()> {
        let cmd = PublishVersionCommand {
            data_source_id: Uuid::new_v4(),
            version: "1.0".to_string(),
            external_version: None,
            release_date: None,
            size_bytes: None,
            additional_metadata: None,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(matches!(
            result,
            Err(PublishVersionError::DataSourceNotFound(_))
        ));
        Ok(())
    }
}
