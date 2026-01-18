//! List jobs query
//!
//! Query to list all jobs from apalis job queue.

use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// Query to list all jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListJobsQuery {
    /// Filter by job type (e.g., "UniProtIngestJob")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_type: Option<String>,
    /// Filter by status (e.g., "Pending", "Running", "Done", "Failed")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Limit number of results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// Job list item
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct JobListItem {
    pub id: String,
    pub job_type: String,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub run_at: DateTime<Utc>,
    pub done_at: Option<DateTime<Utc>>,
    pub lock_at: Option<DateTime<Utc>>,
    pub lock_by: Option<String>,
    pub last_error: Option<String>,
}

/// Response for list jobs query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListJobsResponse {
    pub jobs: Vec<JobListItem>,
    pub total: i64,
}

/// Error type for list jobs query
#[derive(Debug, thiserror::Error)]
pub enum ListJobsError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<ListJobsResponse, ListJobsError>> for ListJobsQuery {}

pub async fn handle(
    pool: PgPool,
    query: ListJobsQuery,
) -> Result<ListJobsResponse, ListJobsError> {
    let limit = query.limit.unwrap_or(100).min(1000); // Max 1000
    let offset = query.offset.unwrap_or(0);

    // Build query based on filters
    let mut sql_query = String::from(
        r#"
        SELECT id, job_type, status, attempts, max_attempts,
               run_at, done_at, lock_at, lock_by, last_error
        FROM apalis.jobs
        WHERE 1=1
        "#,
    );

    if let Some(ref job_type) = query.job_type {
        sql_query.push_str(&format!(" AND job_type = '{}'", job_type));
    }

    if let Some(ref status) = query.status {
        sql_query.push_str(&format!(" AND status = '{}'", status));
    }

    sql_query.push_str(" ORDER BY run_at DESC");
    sql_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    // Get total count
    let mut count_query = String::from("SELECT COUNT(*) FROM apalis.jobs WHERE 1=1");

    if let Some(ref job_type) = query.job_type {
        count_query.push_str(&format!(" AND job_type = '{}'", job_type));
    }

    if let Some(ref status) = query.status {
        count_query.push_str(&format!(" AND status = '{}'", status));
    }

    // Execute queries
    let jobs = sqlx::query_as::<_, JobListItem>(&sql_query)
        .fetch_all(&pool)
        .await?;

    let total: (i64,) = sqlx::query_as(&count_query).fetch_one(&pool).await?;

    Ok(ListJobsResponse {
        jobs,
        total: total.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_jobs_query_defaults() {
        let query = ListJobsQuery {
            job_type: None,
            status: None,
            limit: None,
            offset: None,
        };

        assert!(query.job_type.is_none());
        assert!(query.status.is_none());
    }

    #[test]
    fn test_list_jobs_query_with_filters() {
        let query = ListJobsQuery {
            job_type: Some("UniProtIngestJob".to_string()),
            status: Some("Pending".to_string()),
            limit: Some(50),
            offset: Some(10),
        };

        assert_eq!(query.job_type, Some("UniProtIngestJob".to_string()));
        assert_eq!(query.status, Some("Pending".to_string()));
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.offset, Some(10));
    }
}
