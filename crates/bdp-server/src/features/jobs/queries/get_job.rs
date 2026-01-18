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
    pub attempts: i32,
    pub max_attempts: i32,
    pub run_at: DateTime<Utc>,
    pub done_at: Option<DateTime<Utc>>,
    pub lock_at: Option<DateTime<Utc>>,
    pub lock_by: Option<String>,
    pub last_error: Option<String>,
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

pub async fn handle(
    pool: PgPool,
    query: GetJobQuery,
) -> Result<JobDetails, GetJobError> {
    let job = sqlx::query_as::<_, JobDetails>(
        r#"
        SELECT id, job_type, status, attempts, max_attempts,
               run_at, done_at, lock_at, lock_by, last_error
        FROM apalis.jobs
        WHERE id = $1
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
