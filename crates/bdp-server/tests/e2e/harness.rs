//! E2E test environment orchestration
//!
//! This module provides the main test harness for orchestrating end-to-end tests
//! of the BDP ingestion pipeline. It manages Docker containers, triggers jobs,
//! and provides helper methods for common test operations.
//!
//! # Architecture
//!
//! The E2E environment consists of:
//! - PostgreSQL database (via testcontainers)
//! - MinIO S3-compatible storage (via testcontainers)
//! - BDP server (via testcontainers Docker image)
//!
//! # Example
//!
//! ```no_run
//! use bdp_server::e2e::{E2EEnvironment, TestDataManager, TestDataMode};
//! use std::time::Duration;
//!
//! #[tokio::test]
//! async fn test_ingestion_pipeline() {
//!     let env = E2EEnvironment::new().await.unwrap();
//!     let data_mgr = TestDataManager::new(TestDataMode::CI);
//!
//!     // Upload test data
//!     let dat_path = data_mgr.get_uniprot_dat_path().unwrap();
//!     env.upload_test_data(&dat_path).await.unwrap();
//!
//!     // Trigger job
//!     let job_id = env.trigger_ingestion_job(org_id, "2024_01").await.unwrap();
//!
//!     // Wait for completion
//!     env.wait_for_job_completion(job_id, Duration::from_secs(60)).await.unwrap();
//!
//!     // Assertions
//!     let assertions = env.assertions();
//!     assert_eq!(assertions.count_proteins().await.unwrap(), 3);
//!
//!     env.cleanup().await;
//! }
//! ```

use anyhow::{anyhow, bail, Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use reqwest::Client as HttpClient;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::path::Path;
use std::time::Duration;
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, Image,
};
use testcontainers_modules::postgres::Postgres;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::assertions::E2EAssertions;
use super::observability::E2EObservability;

/// E2E test environment
///
/// Manages all containers and provides helper methods for testing.
pub struct E2EEnvironment {
    /// PostgreSQL container
    postgres_container: ContainerAsync<Postgres>,
    /// MinIO container
    minio_container: ContainerAsync<GenericImage>,
    /// BDP server container (optional, may not be started)
    bdp_server_container: Option<ContainerAsync<GenericImage>>,
    /// Database connection pool
    db_pool: PgPool,
    /// S3 client
    s3_client: S3Client,
    /// HTTP client for API requests
    http_client: HttpClient,
    /// BDP server URL
    server_url: String,
    /// PostgreSQL connection string
    postgres_conn_string: String,
    /// MinIO endpoint
    minio_endpoint: String,
    /// MinIO access key
    minio_access_key: String,
    /// MinIO secret key
    minio_secret_key: String,
    /// S3 bucket name
    s3_bucket: String,
}

impl E2EEnvironment {
    /// Create a new E2E test environment
    ///
    /// This will:
    /// 1. Start PostgreSQL container
    /// 2. Start MinIO container
    /// 3. Run database migrations
    /// 4. Create S3 bucket
    /// 5. Optionally start BDP server container
    ///
    /// # Errors
    ///
    /// Returns an error if any container fails to start or if initialization fails.
    pub async fn new() -> Result<Self> {
        info!("Starting E2E test environment");

        // Start PostgreSQL container
        info!("Starting PostgreSQL container");
        let postgres_container = Postgres::default()
            .with_tag("16-alpine")
            .start()
            .await
            .context("Failed to start PostgreSQL container")?;

        let postgres_host = postgres_container
            .get_host()
            .await
            .context("Failed to get PostgreSQL host")?;
        let postgres_port = postgres_container
            .get_host_port_ipv4(5432.tcp())
            .await
            .context("Failed to get PostgreSQL port")?;

        let postgres_conn_string =
            format!("postgresql://postgres:postgres@{}:{}/postgres", postgres_host, postgres_port);

        info!("PostgreSQL connection: {}", postgres_conn_string);

        // Create database pool
        let db_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&postgres_conn_string)
            .await
            .context("Failed to connect to PostgreSQL")?;

        // Run migrations
        info!("Running database migrations");
        Self::run_migrations(&db_pool)
            .await
            .context("Failed to run migrations")?;

        // Start MinIO container
        info!("Starting MinIO container");
        let minio_access_key = "minioadmin";
        let minio_secret_key = "minioadmin";
        let minio_container = GenericImage::new("minio/minio", "latest")
            .with_exposed_port(9000.tcp())
            .with_wait_for(WaitFor::message_on_stdout("MinIO Object Storage Server"))
            .with_env_var("MINIO_ROOT_USER", minio_access_key)
            .with_env_var("MINIO_ROOT_PASSWORD", minio_secret_key)
            .with_cmd(vec!["server", "/data"])
            .start()
            .await
            .context("Failed to start MinIO container")?;

        let minio_host = minio_container
            .get_host()
            .await
            .context("Failed to get MinIO host")?;
        let minio_port = minio_container
            .get_host_port_ipv4(9000.tcp())
            .await
            .context("Failed to get MinIO port")?;

        let minio_endpoint = format!("http://{}:{}", minio_host, minio_port);
        info!("MinIO endpoint: {}", minio_endpoint);

        // Create S3 client
        let s3_config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url(&minio_endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                minio_access_key,
                minio_secret_key,
                None,
                None,
                "static",
            ))
            .load()
            .await;

        let s3_client = S3Client::new(&s3_config);

        // Create S3 bucket
        let s3_bucket = "bdp-test-data".to_string();
        info!("Creating S3 bucket: {}", s3_bucket);
        s3_client
            .create_bucket()
            .bucket(&s3_bucket)
            .send()
            .await
            .context("Failed to create S3 bucket")?;

        // Create HTTP client
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        // For now, we'll assume the BDP server is running externally
        // In a full implementation, we would build and start a Docker container
        let server_url =
            std::env::var("BDP_SERVER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        info!("BDP server URL: {}", server_url);
        warn!("Note: BDP server container auto-start not yet implemented");
        warn!("Please ensure BDP server is running at: {}", server_url);

        Ok(Self {
            postgres_container,
            minio_container,
            bdp_server_container: None,
            db_pool,
            s3_client,
            http_client,
            server_url,
            postgres_conn_string,
            minio_endpoint,
            minio_access_key: minio_access_key.to_string(),
            minio_secret_key: minio_secret_key.to_string(),
            s3_bucket,
        })
    }

    /// Run database migrations
    async fn run_migrations(pool: &PgPool) -> Result<()> {
        let migrations_dir = std::env::current_dir()
            .context("Failed to get current directory")?
            .join("migrations");

        if !migrations_dir.exists() {
            bail!("Migrations directory not found: {:?}", migrations_dir);
        }

        info!("Running migrations from: {:?}", migrations_dir);

        sqlx::migrate::Migrator::new(migrations_dir)
            .await
            .context("Failed to load migrations")?
            .run(pool)
            .await
            .context("Failed to run migrations")?;

        // Note: Apalis will create its own schema and tables on first use.
        // The apalis.jobs and apalis.workers tables will be automatically
        // created by the apalis-postgres library when the job scheduler
        // is initialized.
        info!("Database migrations complete. Apalis will auto-create its schema on first use.");

        Ok(())
    }

    /// Trigger an ingestion job
    ///
    /// This sends a POST request to `/api/v1/jobs/ingest` to trigger a new
    /// ingestion job for the specified organization and version.
    ///
    /// # Arguments
    ///
    /// * `org_id` - Organization UUID to ingest data for
    /// * `version` - Version string (e.g., "2024_01")
    ///
    /// # Returns
    ///
    /// Returns the job ID as a string.
    ///
    /// # Note
    ///
    /// This endpoint may not exist yet in the current BDP implementation.
    /// The actual implementation uses apalis job queue directly. This is
    /// a placeholder for the E2E test API.
    pub async fn trigger_ingestion_job(&self, org_id: Uuid, version: &str) -> Result<String> {
        info!("Triggering ingestion job for org={}, version={}", org_id, version);

        let url = format!("{}/api/v1/jobs/ingest", self.server_url);
        let payload = serde_json::json!({
            "organization_id": org_id.to_string(),
            "target_version": version,
            "full_sync": true,
        });

        let response = self
            .http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send ingestion job request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Failed to trigger ingestion job: status={}, body={}", status, body);
        }

        let job_response: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse job response")?;

        let job_id = job_response
            .get("job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Job ID not found in response"))?
            .to_string();

        info!("Ingestion job triggered: job_id={}", job_id);
        Ok(job_id)
    }

    /// Wait for a job to complete
    ///
    /// Polls the job status until it reaches a terminal state (completed, failed)
    /// or the timeout is reached.
    ///
    /// # Arguments
    ///
    /// * `job_id` - Job ID to wait for
    /// * `timeout` - Maximum time to wait
    ///
    /// # Returns
    ///
    /// Returns the final job status.
    ///
    /// # Errors
    ///
    /// Returns an error if the job fails or the timeout is reached.
    pub async fn wait_for_job_completion(
        &self,
        job_id: String,
        timeout: Duration,
    ) -> Result<JobStatus> {
        info!("Waiting for job completion: job_id={}", job_id);

        let start = std::time::Instant::now();
        let poll_interval = Duration::from_secs(1);

        loop {
            if start.elapsed() > timeout {
                bail!("Timeout waiting for job completion: job_id={}", job_id);
            }

            let status = self.get_job_status(&job_id).await?;

            match status.status.as_str() {
                "Done" | "Completed" => {
                    info!("Job completed successfully: job_id={}", job_id);
                    return Ok(status);
                },
                "Failed" => {
                    error!("Job failed: job_id={}, error={:?}", job_id, status.last_error);
                    bail!(
                        "Job failed: {}",
                        status
                            .last_error
                            .unwrap_or_else(|| "Unknown error".to_string())
                    );
                },
                "Cancelled" => {
                    warn!("Job was cancelled: job_id={}", job_id);
                    bail!("Job was cancelled");
                },
                _ => {
                    // Job still running
                    tokio::time::sleep(poll_interval).await;
                },
            }
        }
    }

    /// Get job status from the database
    ///
    /// Queries the `apalis.jobs` table directly to get the current job status.
    ///
    /// # Arguments
    ///
    /// * `job_id` - Job ID to query
    ///
    /// # Returns
    ///
    /// Returns the job status information.
    pub async fn get_job_status(&self, job_id: &str) -> Result<JobStatus> {
        let row = sqlx::query!(
            r#"
            SELECT id, job_type, status, attempts, max_attempts, run_at, done_at, last_error
            FROM apalis.jobs
            WHERE id = $1
            "#,
            job_id
        )
        .fetch_one(&self.db_pool)
        .await
        .context("Failed to query job status")?;

        Ok(JobStatus {
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

    /// Create an organization in the database
    ///
    /// Directly inserts an organization into the database for testing purposes.
    ///
    /// # Arguments
    ///
    /// * `slug` - Organization slug (unique identifier)
    /// * `name` - Organization display name
    ///
    /// # Returns
    ///
    /// Returns the UUID of the created organization.
    pub async fn create_organization(&self, slug: &str, name: &str) -> Result<Uuid> {
        info!("Creating test organization: slug={}, name={}", slug, name);

        let org_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, description, is_system)
            VALUES ($1, $2, $3, $4, false)
            "#,
            org_id,
            slug,
            name,
            format!("Test organization: {}", name)
        )
        .execute(&self.db_pool)
        .await
        .context("Failed to create organization")?;

        info!("Organization created: id={}", org_id);
        Ok(org_id)
    }

    /// Upload test data file to S3
    ///
    /// Uploads a test data file (e.g., UniProt DAT file) to the test S3 bucket
    /// at a known location that the ingestion pipeline will read from.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Local path to the file to upload
    ///
    /// # Returns
    ///
    /// Returns the S3 key where the file was uploaded.
    pub async fn upload_test_data(&self, file_path: &Path, s3_key: &str) -> Result<String> {
        info!("Uploading test data: {} -> {}", file_path.display(), s3_key);

        let file_contents = tokio::fs::read(file_path)
            .await
            .context("Failed to read test data file")?;

        self.s3_client
            .put_object()
            .bucket(&self.s3_bucket)
            .key(s3_key)
            .body(file_contents.into())
            .send()
            .await
            .context("Failed to upload test data to S3")?;

        info!("Test data uploaded successfully: {}", s3_key);
        Ok(s3_key.to_string())
    }

    /// Upload test data from bytes to S3
    ///
    /// Uploads raw bytes to the test S3 bucket with the specified key.
    /// Useful for testing error conditions with invalid data.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw bytes to upload
    /// * `s3_key` - S3 key where to upload the data
    ///
    /// # Returns
    ///
    /// Returns the S3 key where the data was uploaded.
    pub async fn upload_test_data_bytes(&self, data: &[u8], s3_key: &str) -> Result<String> {
        info!("Uploading test data bytes: {} ({} bytes)", s3_key, data.len());

        self.s3_client
            .put_object()
            .bucket(&self.s3_bucket)
            .key(s3_key)
            .body(data.to_vec().into())
            .send()
            .await
            .context("Failed to upload test data bytes to S3")?;

        info!("Test data bytes uploaded successfully: {}", s3_key);
        Ok(s3_key.to_string())
    }

    /// Get assertions helper
    ///
    /// Returns an assertions helper for verifying test results.
    pub fn assertions(&self) -> E2EAssertions {
        E2EAssertions::new(self.db_pool.clone(), self.s3_client.clone(), self.s3_bucket.clone())
    }

    /// Get observability helper
    ///
    /// Returns an observability helper for debugging and monitoring tests.
    pub fn observability(&self) -> E2EObservability {
        E2EObservability::new(self.db_pool.clone(), self.s3_client.clone(), self.s3_bucket.clone())
    }

    /// Get database pool
    ///
    /// Returns a clone of the database connection pool for direct queries.
    pub fn db_pool(&self) -> PgPool {
        self.db_pool.clone()
    }

    /// Get S3 client
    ///
    /// Returns a clone of the S3 client for direct S3 operations.
    pub fn s3_client(&self) -> S3Client {
        self.s3_client.clone()
    }

    /// Get S3 bucket name
    pub fn s3_bucket(&self) -> &str {
        &self.s3_bucket
    }

    /// Get server URL
    pub fn server_url(&self) -> &str {
        &self.server_url
    }

    /// Get PostgreSQL connection string
    pub fn postgres_conn_string(&self) -> &str {
        &self.postgres_conn_string
    }

    /// Get MinIO endpoint
    pub fn minio_endpoint(&self) -> &str {
        &self.minio_endpoint
    }

    /// Cleanup test environment
    ///
    /// Stops all containers and cleans up resources.
    /// This should be called at the end of each test.
    pub async fn cleanup(self) {
        info!("Cleaning up E2E test environment");

        // Containers will be automatically stopped and removed when dropped
        drop(self.postgres_container);
        drop(self.minio_container);
        if let Some(container) = self.bdp_server_container {
            drop(container);
        }

        info!("E2E test environment cleaned up");
    }
}

/// Job status information
#[derive(Debug, Clone)]
pub struct JobStatus {
    /// Job ID
    pub id: String,
    /// Job type (e.g., "UniProtIngestJob")
    pub job_type: String,
    /// Current status (e.g., "Pending", "Running", "Done", "Failed")
    pub status: String,
    /// Number of attempts
    pub attempts: i32,
    /// Maximum attempts allowed
    pub max_attempts: i32,
    /// When the job was scheduled to run
    pub run_at: chrono::DateTime<chrono::Utc>,
    /// When the job completed (if finished)
    pub done_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last error message (if failed)
    pub last_error: Option<String>,
}

impl JobStatus {
    /// Check if the job is completed
    pub fn is_completed(&self) -> bool {
        matches!(self.status.as_str(), "Done" | "Completed")
    }

    /// Check if the job failed
    pub fn is_failed(&self) -> bool {
        self.status == "Failed"
    }

    /// Check if the job is running
    pub fn is_running(&self) -> bool {
        matches!(self.status.as_str(), "Running" | "Pending")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn test_environment_startup() {
        let env = E2EEnvironment::new().await;
        assert!(env.is_ok(), "Environment should start successfully");

        if let Ok(env) = env {
            // Verify database connection
            let result = sqlx::query!("SELECT 1 as one")
                .fetch_one(&env.db_pool)
                .await;
            assert!(result.is_ok(), "Database should be accessible");

            // Verify S3 bucket exists
            let buckets = env.s3_client.list_buckets().send().await;
            assert!(buckets.is_ok(), "S3 should be accessible");

            env.cleanup().await;
        }
    }

    #[test]
    fn test_job_status_checks() {
        let completed_status = JobStatus {
            id: "test-1".to_string(),
            job_type: "TestJob".to_string(),
            status: "Done".to_string(),
            attempts: 1,
            max_attempts: 3,
            run_at: chrono::Utc::now(),
            done_at: Some(chrono::Utc::now()),
            last_error: None,
        };

        assert!(completed_status.is_completed());
        assert!(!completed_status.is_failed());
        assert!(!completed_status.is_running());

        let failed_status = JobStatus {
            status: "Failed".to_string(),
            last_error: Some("Test error".to_string()),
            ..completed_status.clone()
        };

        assert!(!failed_status.is_completed());
        assert!(failed_status.is_failed());
        assert!(!failed_status.is_running());
    }
}
