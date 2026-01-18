//! Get sync status query
//!
//! Query to get organization sync status.

use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Query to list all sync statuses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSyncStatusQuery {
    /// Filter by organization ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Uuid>,
    /// Filter by status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Query to get sync status for a specific organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSyncStatusQuery {
    pub organization_id: Uuid,
}

/// Sync status item
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncStatusItem {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_version: Option<String>,
    pub last_external_version: Option<String>,
    pub status: String,
    pub total_entries: i64,
    pub last_job_id: Option<Uuid>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response for list sync status query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSyncStatusResponse {
    pub statuses: Vec<SyncStatusItem>,
}

/// Error type for sync status queries
#[derive(Debug, thiserror::Error)]
pub enum SyncStatusError {
    #[error("Sync status not found")]
    NotFound,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<ListSyncStatusResponse, SyncStatusError>> for ListSyncStatusQuery {}

pub async fn handle_list(
    pool: PgPool,
    query: ListSyncStatusQuery,
) -> Result<ListSyncStatusResponse, SyncStatusError> {
    let mut sql_query = String::from(
        r#"
        SELECT id, organization_id, last_sync_at, last_version,
               last_external_version, status, total_entries,
               last_job_id, last_error, created_at, updated_at
        FROM organization_sync_status
        WHERE 1=1
        "#,
    );

    if let Some(org_id) = query.organization_id {
        sql_query.push_str(&format!(" AND organization_id = '{}'", org_id));
    }

    if let Some(ref status) = query.status {
        sql_query.push_str(&format!(" AND status = '{}'", status));
    }

    sql_query.push_str(" ORDER BY updated_at DESC");

    let statuses = sqlx::query_as::<_, SyncStatusItem>(&sql_query)
        .fetch_all(&pool)
        .await?;

    Ok(ListSyncStatusResponse { statuses })
}

impl Request<Result<SyncStatusItem, SyncStatusError>> for GetSyncStatusQuery {}

pub async fn handle_get(
    pool: PgPool,
    query: GetSyncStatusQuery,
) -> Result<SyncStatusItem, SyncStatusError> {
    let status = sqlx::query_as::<_, SyncStatusItem>(
        r#"
        SELECT id, organization_id, last_sync_at, last_version,
               last_external_version, status, total_entries,
               last_job_id, last_error, created_at, updated_at
        FROM organization_sync_status
        WHERE organization_id = $1
        "#,
    )
    .bind(query.organization_id)
    .fetch_optional(&pool)
    .await?
    .ok_or(SyncStatusError::NotFound)?;

    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_sync_status_query_defaults() {
        let query = ListSyncStatusQuery {
            organization_id: None,
            status: None,
        };

        assert!(query.organization_id.is_none());
        assert!(query.status.is_none());
    }

    #[test]
    fn test_list_sync_status_query_with_filters() {
        let org_id = Uuid::new_v4();
        let query = ListSyncStatusQuery {
            organization_id: Some(org_id),
            status: Some("running".to_string()),
        };

        assert_eq!(query.organization_id, Some(org_id));
        assert_eq!(query.status, Some("running".to_string()));
    }

    #[test]
    fn test_get_sync_status_query() {
        let org_id = Uuid::new_v4();
        let query = GetSyncStatusQuery {
            organization_id: org_id,
        };

        assert_eq!(query.organization_id, org_id);
    }
}
