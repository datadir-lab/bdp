//! E2E test observability and debugging helpers
//!
//! This module provides utilities for monitoring, debugging, and inspecting
//! the state of the system during E2E tests.
//!
//! # Example
//!
//! ```no_run
//! use bdp_server::e2e::E2EEnvironment;
//!
//! #[tokio::test]
//! async fn test_with_observability() {
//!     let env = E2EEnvironment::new().await.unwrap();
//!     let obs = env.observability();
//!
//!     // Print current state
//!     obs.print_pipeline_status().await.unwrap();
//!
//!     // Get database statistics
//!     let stats = obs.get_db_stats().await.unwrap();
//!     println!("Database stats: {:?}", stats);
//!
//!     // List all jobs
//!     let jobs = obs.query_apalis_jobs().await.unwrap();
//!     for job in jobs {
//!         println!("Job: {:?}", job);
//!     }
//!
//!     env.cleanup().await;
//! }
//! ```

use anyhow::{Context, Result};
use aws_sdk_s3::Client as S3Client;
use sqlx::PgPool;
use tracing::info;

/// E2E test observability helper
///
/// Provides methods for monitoring and debugging the system state during tests.
pub struct E2EObservability {
    /// Database connection pool
    db: PgPool,
    /// S3 client
    s3: S3Client,
    /// S3 bucket name
    bucket: String,
}

impl E2EObservability {
    /// Create a new observability helper
    pub fn new(db: PgPool, s3: S3Client, bucket: String) -> Self {
        Self { db, s3, bucket }
    }

    /// Get container logs (placeholder)
    ///
    /// In a full implementation, this would fetch Docker container logs
    /// using the Docker API or testcontainers API.
    ///
    /// # Arguments
    ///
    /// * `container_name` - Name of the container to get logs from
    ///
    /// # Note
    ///
    /// This is currently a placeholder. Testcontainers doesn't expose
    /// container logs directly through its API.
    pub async fn get_container_logs(&self, container_name: &str) -> Result<String> {
        info!("Container logs requested for: {}", container_name);
        Ok(format!("Container logs for {} (not yet implemented)", container_name))
    }

    /// Query all jobs from apalis.jobs table
    ///
    /// Returns a list of all jobs in the system with their current status.
    pub async fn query_apalis_jobs(&self) -> Result<Vec<ApalisJob>> {
        let jobs = sqlx::query_as!(
            ApalisJobRow,
            r#"
            SELECT
                id,
                job_type,
                status,
                attempts,
                max_attempts,
                run_at,
                done_at,
                lock_at,
                lock_by,
                last_error
            FROM apalis.jobs
            ORDER BY run_at DESC
            "#
        )
        .fetch_all(&self.db)
        .await
        .context("Failed to query apalis jobs")?;

        Ok(jobs
            .into_iter()
            .map(|row| ApalisJob {
                id: row.id,
                job_type: row.job_type,
                status: row.status,
                attempts: row.attempts,
                max_attempts: row.max_attempts,
                run_at: row.run_at,
                done_at: row.done_at,
                lock_at: row.lock_at,
                lock_by: row.lock_by,
                last_error: row.last_error,
            })
            .collect())
    }

    /// List all S3 objects in the test bucket
    ///
    /// Returns a list of all S3 object keys and their metadata.
    pub async fn list_s3_contents(&self) -> Result<Vec<S3Object>> {
        let mut objects = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut request = self.s3.list_objects_v2().bucket(&self.bucket);

            if let Some(token) = continuation_token.as_ref() {
                request = request.continuation_token(token);
            }

            let response = request.send().await.context("Failed to list S3 objects")?;

            if let Some(contents) = response.contents() {
                for obj in contents {
                    objects.push(S3Object {
                        key: obj.key().unwrap_or("").to_string(),
                        size: obj.size().unwrap_or(0),
                        last_modified: obj
                            .last_modified()
                            .map(|dt| dt.to_string())
                            .unwrap_or_default(),
                        etag: obj.e_tag().unwrap_or("").to_string(),
                    });
                }
            }

            if response.is_truncated().unwrap_or(false) {
                continuation_token = response.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(objects)
    }

    /// Get database statistics
    ///
    /// Returns counts for all major tables in the system.
    pub async fn get_db_stats(&self) -> Result<DbStats> {
        let proteins = sqlx::query!("SELECT COUNT(*) as count FROM protein_metadata")
            .fetch_one(&self.db)
            .await?
            .count
            .unwrap_or(0);

        let organisms = sqlx::query!("SELECT COUNT(*) as count FROM organisms")
            .fetch_one(&self.db)
            .await?
            .count
            .unwrap_or(0);

        let version_files = sqlx::query!("SELECT COUNT(*) as count FROM version_files")
            .fetch_one(&self.db)
            .await?
            .count
            .unwrap_or(0);

        let data_sources = sqlx::query!("SELECT COUNT(*) as count FROM data_sources")
            .fetch_one(&self.db)
            .await?
            .count
            .unwrap_or(0);

        let organizations = sqlx::query!("SELECT COUNT(*) as count FROM organizations")
            .fetch_one(&self.db)
            .await?
            .count
            .unwrap_or(0);

        let registry_entries = sqlx::query!("SELECT COUNT(*) as count FROM registry_entries")
            .fetch_one(&self.db)
            .await?
            .count
            .unwrap_or(0);

        let versions = sqlx::query!("SELECT COUNT(*) as count FROM versions")
            .fetch_one(&self.db)
            .await?
            .count
            .unwrap_or(0);

        let jobs = sqlx::query!("SELECT COUNT(*) as count FROM apalis.jobs")
            .fetch_one(&self.db)
            .await?
            .count
            .unwrap_or(0);

        Ok(DbStats {
            proteins,
            organisms,
            version_files,
            data_sources,
            organizations,
            registry_entries,
            versions,
            jobs,
        })
    }

    /// Print pipeline status to console
    ///
    /// Pretty-prints the current state of the pipeline including:
    /// - Database table counts
    /// - Job status
    /// - S3 object counts
    pub async fn print_pipeline_status(&self) -> Result<()> {
        println!("\n========================================");
        println!("        BDP Pipeline Status");
        println!("========================================\n");

        // Database stats
        let db_stats = self.get_db_stats().await?;
        println!("Database Tables:");
        println!("  - Proteins:         {}", db_stats.proteins);
        println!("  - Organisms:        {}", db_stats.organisms);
        println!("  - Version Files:    {}", db_stats.version_files);
        println!("  - Data Sources:     {}", db_stats.data_sources);
        println!("  - Organizations:    {}", db_stats.organizations);
        println!("  - Registry Entries: {}", db_stats.registry_entries);
        println!("  - Versions:         {}", db_stats.versions);
        println!("  - Jobs:             {}", db_stats.jobs);

        // Job details
        println!("\nRecent Jobs:");
        let jobs = self.query_apalis_jobs().await?;
        if jobs.is_empty() {
            println!("  (no jobs)");
        } else {
            for job in jobs.iter().take(5) {
                println!(
                    "  - {} [{}] attempts={}/{}",
                    job.id, job.status, job.attempts, job.max_attempts
                );
                if let Some(error) = &job.last_error {
                    println!("    Error: {}", error);
                }
            }
        }

        // S3 stats
        println!("\nS3 Storage:");
        let s3_objects = self.list_s3_contents().await?;
        println!("  - Total Objects: {}", s3_objects.len());

        let total_size: i64 = s3_objects.iter().map(|obj| obj.size).sum();
        let size_mb = total_size as f64 / (1024.0 * 1024.0);
        println!("  - Total Size:    {:.2} MB", size_mb);

        if !s3_objects.is_empty() {
            println!("  - Sample objects:");
            for obj in s3_objects.iter().take(5) {
                let size_kb = obj.size as f64 / 1024.0;
                println!("    - {} ({:.1} KB)", obj.key, size_kb);
            }
        }

        println!("\n========================================\n");

        Ok(())
    }

    /// Tail server logs (placeholder)
    ///
    /// In a full implementation, this would fetch recent server logs.
    ///
    /// # Arguments
    ///
    /// * `lines` - Number of log lines to fetch
    ///
    /// # Note
    ///
    /// This is currently a placeholder for future implementation.
    pub async fn tail_server_logs(&self, lines: usize) -> Result<Vec<String>> {
        info!("Requested {} lines of server logs", lines);
        Ok(vec![format!("Server logs (last {} lines - not yet implemented)", lines)])
    }

    /// Get sync status for all organizations
    ///
    /// Returns the current sync status for each organization.
    pub async fn get_all_sync_statuses(&self) -> Result<Vec<SyncStatusInfo>> {
        let statuses = sqlx::query_as!(
            SyncStatusRow,
            r#"
            SELECT
                oss.id,
                oss.organization_id,
                o.name as organization_name,
                oss.last_sync_at,
                oss.last_version,
                oss.last_external_version,
                oss.status,
                oss.total_entries,
                oss.last_job_id,
                oss.last_error
            FROM organization_sync_status oss
            JOIN organizations o ON o.id = oss.organization_id
            ORDER BY oss.updated_at DESC
            "#
        )
        .fetch_all(&self.db)
        .await
        .context("Failed to query sync statuses")?;

        Ok(statuses
            .into_iter()
            .map(|row| SyncStatusInfo {
                id: row.id,
                organization_id: row.organization_id,
                organization_name: row.organization_name,
                last_sync_at: row.last_sync_at,
                last_version: row.last_version,
                last_external_version: row.last_external_version,
                status: row.status,
                total_entries: row.total_entries.unwrap_or(0),
                last_job_id: row.last_job_id,
                last_error: row.last_error,
            })
            .collect())
    }

    /// Export database state to JSON (for debugging)
    ///
    /// Exports key database tables to JSON format for analysis.
    pub async fn export_db_state(&self) -> Result<String> {
        let stats = self.get_db_stats().await?;
        let jobs = self.query_apalis_jobs().await?;
        let sync_statuses = self.get_all_sync_statuses().await?;

        let state = serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "stats": {
                "proteins": stats.proteins,
                "organisms": stats.organisms,
                "version_files": stats.version_files,
                "data_sources": stats.data_sources,
                "organizations": stats.organizations,
                "registry_entries": stats.registry_entries,
                "versions": stats.versions,
                "jobs": stats.jobs,
            },
            "jobs": jobs,
            "sync_statuses": sync_statuses,
        });

        serde_json::to_string_pretty(&state).context("Failed to serialize state")
    }

    /// Wait for a condition to be true
    ///
    /// Polls a condition function until it returns true or the timeout is reached.
    ///
    /// # Arguments
    ///
    /// * `condition_name` - Name for logging
    /// * `check` - Async function that returns true when condition is met
    /// * `timeout` - Maximum time to wait
    /// * `poll_interval` - Time between checks
    pub async fn wait_for_condition<F, Fut>(
        &self,
        condition_name: &str,
        check: F,
        timeout: std::time::Duration,
        poll_interval: std::time::Duration,
    ) -> Result<()>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<bool>>,
    {
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                anyhow::bail!("Timeout waiting for condition: {}", condition_name);
            }

            if check().await? {
                info!("Condition met: {}", condition_name);
                return Ok(());
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Get job information by ID
    ///
    /// Queries the apalis.jobs table for a specific job and returns its details.
    pub async fn get_job_info(&self, job_id: &str) -> Result<JobInfo> {
        let row = sqlx::query!(
            r#"
            SELECT id, job_type, status, attempts, max_attempts, run_at, done_at, last_error
            FROM apalis.jobs
            WHERE id = $1
            "#,
            job_id
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to query job info")?;

        Ok(JobInfo {
            id: row.id,
            job_type: row.job_type,
            status: row.status,
            attempts: row.attempts,
            max_attempts: row.max_attempts,
            run_at: row.run_at,
            done_at: row.done_at,
            last_error: row.last_error,
        })
    }

    /// Print job status to console
    ///
    /// Displays formatted job information for debugging.
    pub async fn print_job_status(&self, job_id: &str) -> Result<()> {
        let job = self.get_job_info(job_id).await?;

        info!("╔═══════════════════════════════════════════════════════════════");
        info!("║ Job Status: {}", job.id);
        info!("╠═══════════════════════════════════════════════════════════════");
        info!("║ Type:         {}", job.job_type);
        info!("║ Status:       {}", job.status);
        info!("║ Attempts:     {}/{}", job.attempts, job.max_attempts);
        info!("║ Run At:       {}", job.run_at);
        if let Some(done_at) = job.done_at {
            info!("║ Done At:      {}", done_at);
        }
        if let Some(error) = &job.last_error {
            info!("║ Last Error:   {}", error);
        }
        info!("╚═══════════════════════════════════════════════════════════════");

        Ok(())
    }

    /// List S3 objects with optional prefix filter
    ///
    /// Returns a list of S3 object keys matching the prefix.
    pub async fn list_s3_objects(&self, prefix: Option<&str>) -> Result<Vec<S3Object>> {
        let mut request = self.s3.list_objects_v2().bucket(&self.bucket);

        if let Some(p) = prefix {
            request = request.prefix(p);
        }

        let response = request.send().await.context("Failed to list S3 objects")?;

        let objects = response
            .contents()
            .unwrap_or_default()
            .iter()
            .map(|obj| S3Object {
                key: obj.key().unwrap_or("").to_string(),
                size: obj.size().unwrap_or(0),
                last_modified: obj
                    .last_modified()
                    .map(|dt| dt.to_string())
                    .unwrap_or_default(),
                etag: obj.e_tag().unwrap_or("").to_string(),
            })
            .collect();

        Ok(objects)
    }
}

/// Job information (simplified)
#[derive(Debug, Clone)]
pub struct JobInfo {
    pub id: String,
    pub job_type: String,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub run_at: chrono::DateTime<chrono::Utc>,
    pub done_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error: Option<String>,
}

/// Apalis job information
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApalisJob {
    pub id: String,
    pub job_type: String,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub run_at: chrono::DateTime<chrono::Utc>,
    pub done_at: Option<chrono::DateTime<chrono::Utc>>,
    pub lock_at: Option<chrono::DateTime<chrono::Utc>>,
    pub lock_by: Option<String>,
    pub last_error: Option<String>,
}

/// Row type for querying apalis jobs
struct ApalisJobRow {
    id: String,
    job_type: String,
    status: String,
    attempts: i32,
    max_attempts: i32,
    run_at: chrono::DateTime<chrono::Utc>,
    done_at: Option<chrono::DateTime<chrono::Utc>>,
    lock_at: Option<chrono::DateTime<chrono::Utc>>,
    lock_by: Option<String>,
    last_error: Option<String>,
}

/// S3 object information
#[derive(Debug, Clone)]
pub struct S3Object {
    pub key: String,
    pub size: i64,
    pub last_modified: String,
    pub etag: String,
}

/// Database statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct DbStats {
    pub proteins: i64,
    pub organisms: i64,
    pub version_files: i64,
    pub data_sources: i64,
    pub organizations: i64,
    pub registry_entries: i64,
    pub versions: i64,
    pub jobs: i64,
}

/// Sync status information
#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncStatusInfo {
    pub id: uuid::Uuid,
    pub organization_id: uuid::Uuid,
    pub organization_name: String,
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_version: Option<String>,
    pub last_external_version: Option<String>,
    pub status: String,
    pub total_entries: i64,
    pub last_job_id: Option<uuid::Uuid>,
    pub last_error: Option<String>,
}

/// Row type for querying sync status
struct SyncStatusRow {
    id: uuid::Uuid,
    organization_id: uuid::Uuid,
    organization_name: String,
    last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    last_version: Option<String>,
    last_external_version: Option<String>,
    status: String,
    total_entries: Option<i64>,
    last_job_id: Option<uuid::Uuid>,
    last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_stats_serialization() {
        let stats = DbStats {
            proteins: 100,
            organisms: 50,
            version_files: 200,
            data_sources: 150,
            organizations: 5,
            registry_entries: 10,
            versions: 300,
            jobs: 25,
        };

        let json = serde_json::to_string(&stats);
        assert!(json.is_ok());
    }

    #[test]
    fn test_apalis_job_serialization() {
        let job = ApalisJob {
            id: "test-job-1".to_string(),
            job_type: "UniProtIngestJob".to_string(),
            status: "Done".to_string(),
            attempts: 1,
            max_attempts: 3,
            run_at: chrono::Utc::now(),
            done_at: Some(chrono::Utc::now()),
            lock_at: None,
            lock_by: None,
            last_error: None,
        };

        let json = serde_json::to_string(&job);
        assert!(json.is_ok());
    }
}
