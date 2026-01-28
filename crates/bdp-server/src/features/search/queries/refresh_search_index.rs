use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// Command to refresh the search materialized view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshSearchIndexCommand {
    /// If true, use REFRESH MATERIALIZED VIEW CONCURRENTLY (slower but non-blocking)
    /// If false, use regular REFRESH (faster but blocks reads)
    pub concurrent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshSearchIndexResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RefreshSearchIndexError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<RefreshSearchIndexResponse, RefreshSearchIndexError>>
    for RefreshSearchIndexCommand
{
}

impl crate::cqrs::middleware::Command for RefreshSearchIndexCommand {}

/// Refreshes the search materialized view
///
/// This should be called periodically (e.g., every 5 minutes) to keep the search index
/// up-to-date with the latest data. Use concurrent=true for non-blocking refresh during
/// business hours, or concurrent=false for faster refresh during maintenance windows.
#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    command: RefreshSearchIndexCommand,
) -> Result<RefreshSearchIndexResponse, RefreshSearchIndexError> {
    let start = std::time::Instant::now();

    if command.concurrent {
        // Concurrent refresh - slower but doesn't block reads
        sqlx::query!("SELECT refresh_search_mv_concurrent()")
            .execute(&pool)
            .await?;
    } else {
        // Non-concurrent refresh - faster but blocks reads
        sqlx::query!("SELECT refresh_search_mv()")
            .execute(&pool)
            .await?;
    }

    let duration = start.elapsed();
    let message = format!(
        "Search index refreshed {} in {:.2}s",
        if command.concurrent {
            "concurrently"
        } else {
            ""
        },
        duration.as_secs_f64()
    );

    tracing::info!("{}", message);

    Ok(RefreshSearchIndexResponse {
        success: true,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_creation() {
        let cmd = RefreshSearchIndexCommand { concurrent: true };
        assert!(cmd.concurrent);

        let cmd = RefreshSearchIndexCommand { concurrent: false };
        assert!(!cmd.concurrent);
    }
}
