//! Get job query
//!
//! Query to get a single job by ID.

use chrono::{DateTime, Utc};
use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// Query to get a job by ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetJobQuery {
    pub job_id: String,
}

/// Job details
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct JobDetails {
    pub id: String,
    pub job_type: String,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub total_records: Option<i64>,
    pub records_processed: i64,
    pub records_stored: i64,
    pub records_failed: i64,
}

/// Error type for get job query
#[derive(Debug, thiserror::Error)]
pub enum GetJobError {
    #[error("Job not found")]
    NotFound,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<JobDetails, GetJobError>> for GetJobQuery {}

pub async fn handle(pool: PgPool, query: GetJobQuery) -> Result<JobDetails, GetJobError> {
    let job = sqlx::query_as::<_, JobDetails>(
        r#"
        SELECT id::text, job_type, status, started_at, completed_at, created_at,
               total_records, records_processed, records_stored, records_failed
        FROM ingestion_jobs
        WHERE id::text = $1
        "#,
    )
    .bind(&query.job_id)
    .fetch_optional(&pool)
    .await?
    .ok_or(GetJobError::NotFound)?;

    Ok(job)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_job_query() {
        let query = GetJobQuery {
            job_id: "test-job-id".to_string(),
        };

        assert_eq!(query.job_id, "test-job-id");
    }
}
