//! Get vector stats query
//!
//! Returns aggregate statistics about the vector embeddings pipeline:
//! current projection run status, entry/embedding counts, and tile prefix.

use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// Query to retrieve vector pipeline statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetVectorStatsQuery;

/// Response containing vector pipeline statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStatsResponse {
    /// UUID of the most recent complete projection run, or null
    pub current_run_id: Option<String>,
    /// Current pipeline status
    pub status: Option<String>,
    /// Total registry entries
    pub entry_count: Option<i64>,
    /// Entries with embeddings
    pub embedded_count: Option<i64>,
    /// Entries with 2D projection coords
    pub projected_count: Option<i64>,
    /// When the last projection completed
    pub projected_at: Option<chrono::DateTime<chrono::Utc>>,
    /// MinIO tile prefix for the current run
    pub tile_prefix: Option<String>,
}

/// Errors that can occur while retrieving vector stats
#[derive(Debug, thiserror::Error)]
pub enum GetVectorStatsError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<VectorStatsResponse, GetVectorStatsError>> for GetVectorStatsQuery {}

impl crate::cqrs::middleware::Query for GetVectorStatsQuery {}

/// Handles the get vector stats query
///
/// Returns the most recent projection run row combined with live counts from
/// `registry_entries` and `entry_embeddings`.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `_query` - The query (no parameters required)
///
/// # Errors
///
/// - `Database` - A database error occurred
#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    _query: GetVectorStatsQuery,
) -> Result<VectorStatsResponse, GetVectorStatsError> {
    // Get most recent run
    let run = sqlx::query!(
        r#"
        SELECT run_id::text, status, entry_count, embedded_count,
               projected_count, projected_at, tile_prefix
        FROM vector_projection_runs
        ORDER BY started_at DESC
        LIMIT 1
        "#
    )
    .fetch_optional(&pool)
    .await?;

    // Total entry count (fast, from registry_entries)
    let total_entries = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM registry_entries"
    )
    .fetch_one(&pool)
    .await?;

    // Embedded count
    let embedded_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM entry_embeddings"
    )
    .fetch_one(&pool)
    .await?;

    Ok(VectorStatsResponse {
        current_run_id: run.as_ref().and_then(|r| r.run_id.clone()),
        status: run.as_ref().map(|r| r.status.clone()),
        entry_count: total_entries,
        embedded_count,
        projected_count: run.as_ref().and_then(|r| r.projected_count),
        projected_at: run.as_ref().and_then(|r| r.projected_at),
        tile_prefix: run.as_ref().and_then(|r| r.tile_prefix.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_stats_returns_nulls_with_no_data(pool: PgPool) -> sqlx::Result<()> {
        let result = handle(pool, GetVectorStatsQuery).await;
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert!(stats.current_run_id.is_none());
        assert!(stats.entry_count.unwrap_or(0) == 0);
        Ok(())
    }
}
