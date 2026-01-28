//! Common test utilities for BDP server integration tests using testcontainers
//!
//! This module provides easy-to-use utilities for spinning up PostgreSQL and MinIO
//! containers for integration testing without manual database/service setup.
//!
//! # Features
//!
//! - PostgreSQL container with automatic migration
//! - MinIO (S3-compatible) container with bucket creation
//! - Connection pool management
//! - Test isolation (each test gets its own containers)
//!
//! # Example
//!
//! ```no_run
//! use testcontainers::runners::AsyncRunner;
//!
//! mod common;
//! use common::{TestPostgres, TestMinio, TestEnvironment};
//!
//! #[tokio::test]
//! async fn test_with_postgres() {
//!     let pg = TestPostgres::start().await.expect("Failed to start PostgreSQL");
//!     let pool = pg.pool();
//!
//!     // Your test code here
//!     sqlx::query("SELECT 1").execute(pool).await.expect("Query failed");
//! }
//!
//! #[tokio::test]
//! async fn test_with_full_environment() {
//!     let env = TestEnvironment::start().await.expect("Failed to start environment");
//!
//!     // Access PostgreSQL
//!     let pool = env.db_pool();
//!
//!     // Access MinIO/S3
//!     let s3 = env.s3_client();
//!
//!     // Your test code here
//! }
//! ```

use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage,
};
use testcontainers_modules::postgres::Postgres;
use tracing::{debug, info};

/// Default S3 bucket name for tests
pub const DEFAULT_TEST_BUCKET: &str = "bdp-test-data";

/// Default MinIO credentials
pub const MINIO_ACCESS_KEY: &str = "minioadmin";
pub const MINIO_SECRET_KEY: &str = "minioadmin";

// ============================================================================
// PostgreSQL Test Container
// ============================================================================

/// PostgreSQL test container wrapper
///
/// Provides a PostgreSQL container with migrations pre-applied, ready for testing.
///
/// # Example
///
/// ```no_run
/// use common::TestPostgres;
///
/// #[tokio::test]
/// async fn test_database_operations() {
///     let pg = TestPostgres::start().await.unwrap();
///     let pool = pg.pool();
///
///     // Run your database tests
///     let result = sqlx::query!("SELECT 1 as value")
///         .fetch_one(pool)
///         .await
///         .unwrap();
///     assert_eq!(result.value, Some(1));
/// }
/// ```
pub struct TestPostgres {
    container: ContainerAsync<Postgres>,
    pool: PgPool,
    connection_string: String,
}

impl TestPostgres {
    /// Start a new PostgreSQL container with migrations applied
    ///
    /// # Returns
    ///
    /// Returns a `TestPostgres` instance with a connected pool and migrations run.
    ///
    /// # Errors
    ///
    /// Returns an error if the container fails to start or migrations fail.
    pub async fn start() -> Result<Self> {
        Self::start_with_options(PostgresOptions::default()).await
    }

    /// Start a new PostgreSQL container with custom options
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration options for the PostgreSQL container
    ///
    /// # Returns
    ///
    /// Returns a `TestPostgres` instance with the specified configuration.
    pub async fn start_with_options(options: PostgresOptions) -> Result<Self> {
        info!("Starting PostgreSQL test container...");

        let container = Postgres::default()
            .with_tag(&options.version)
            .start()
            .await
            .context("Failed to start PostgreSQL container")?;

        let host = container
            .get_host()
            .await
            .context("Failed to get container host")?;
        let port = container
            .get_host_port_ipv4(5432.tcp())
            .await
            .context("Failed to get container port")?;

        let connection_string =
            format!("postgresql://postgres:postgres@{}:{}/postgres", host, port);

        debug!("PostgreSQL connection: {}", connection_string);

        let pool = PgPoolOptions::new()
            .max_connections(options.max_connections)
            .acquire_timeout(Duration::from_secs(options.acquire_timeout_secs))
            .connect(&connection_string)
            .await
            .context("Failed to connect to PostgreSQL")?;

        // Run migrations if enabled
        if options.run_migrations {
            info!("Running database migrations...");
            Self::run_migrations(&pool).await?;
            info!("Migrations completed successfully");
        }

        Ok(Self {
            container,
            pool,
            connection_string,
        })
    }

    /// Run database migrations
    async fn run_migrations(pool: &PgPool) -> Result<()> {
        // The migrations are located relative to the crate root
        sqlx::migrate!("../../migrations")
            .run(pool)
            .await
            .context("Failed to run migrations")?;
        Ok(())
    }

    /// Get a reference to the database pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get a clone of the database pool
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// Get the connection string
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    /// Get host and port as a tuple
    pub async fn host_port(&self) -> Result<(String, u16)> {
        let host = self
            .container
            .get_host()
            .await
            .context("Failed to get host")?;
        let port = self
            .container
            .get_host_port_ipv4(5432.tcp())
            .await
            .context("Failed to get port")?;
        Ok((host.to_string(), port))
    }
}

/// Configuration options for PostgreSQL test container
pub struct PostgresOptions {
    /// PostgreSQL version/tag (default: "16-alpine")
    pub version: String,
    /// Maximum number of connections in the pool (default: 5)
    pub max_connections: u32,
    /// Connection acquire timeout in seconds (default: 30)
    pub acquire_timeout_secs: u64,
    /// Whether to run migrations on startup (default: true)
    pub run_migrations: bool,
}

impl Default for PostgresOptions {
    fn default() -> Self {
        Self {
            version: "16-alpine".to_string(),
            max_connections: 5,
            acquire_timeout_secs: 30,
            run_migrations: true,
        }
    }
}

impl PostgresOptions {
    /// Create new options without migrations
    pub fn without_migrations() -> Self {
        Self {
            run_migrations: false,
            ..Default::default()
        }
    }

    /// Set the PostgreSQL version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the max connections
    pub fn with_max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }
}

// ============================================================================
// MinIO Test Container
// ============================================================================

/// MinIO (S3-compatible) test container wrapper
///
/// Provides a MinIO container configured for testing with a default bucket created.
///
/// # Example
///
/// ```no_run
/// use common::TestMinio;
///
/// #[tokio::test]
/// async fn test_s3_operations() {
///     let minio = TestMinio::start().await.unwrap();
///     let client = minio.client();
///
///     // Upload a test object
///     client
///         .put_object()
///         .bucket(minio.bucket())
///         .key("test-file.txt")
///         .body("Hello, world!".as_bytes().to_vec().into())
///         .send()
///         .await
///         .unwrap();
/// }
/// ```
pub struct TestMinio {
    container: ContainerAsync<GenericImage>,
    client: S3Client,
    endpoint: String,
    bucket: String,
}

impl TestMinio {
    /// Start a new MinIO container with default bucket created
    ///
    /// # Returns
    ///
    /// Returns a `TestMinio` instance with a configured S3 client and bucket.
    pub async fn start() -> Result<Self> {
        Self::start_with_options(MinioOptions::default()).await
    }

    /// Start a new MinIO container with custom options
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration options for the MinIO container
    pub async fn start_with_options(options: MinioOptions) -> Result<Self> {
        info!("Starting MinIO test container...");

        let container = GenericImage::new("minio/minio", &options.version)
            .with_exposed_port(9000.tcp())
            .with_wait_for(WaitFor::message_on_stdout("MinIO Object Storage Server"))
            .with_env_var("MINIO_ROOT_USER", &options.access_key)
            .with_env_var("MINIO_ROOT_PASSWORD", &options.secret_key)
            .with_cmd(vec!["server", "/data"])
            .start()
            .await
            .context("Failed to start MinIO container")?;

        let host = container
            .get_host()
            .await
            .context("Failed to get MinIO host")?;
        let port = container
            .get_host_port_ipv4(9000.tcp())
            .await
            .context("Failed to get MinIO port")?;

        let endpoint = format!("http://{}:{}", host, port);
        debug!("MinIO endpoint: {}", endpoint);

        // Create S3 client
        let s3_config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url(&endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                &options.access_key,
                &options.secret_key,
                None,
                None,
                "static",
            ))
            .load()
            .await;

        let client = S3Client::new(&s3_config);

        // Create the default bucket
        if options.create_bucket {
            info!("Creating test bucket: {}", options.bucket_name);
            client
                .create_bucket()
                .bucket(&options.bucket_name)
                .send()
                .await
                .context("Failed to create S3 bucket")?;
        }

        Ok(Self {
            container,
            client,
            endpoint,
            bucket: options.bucket_name,
        })
    }

    /// Get the S3 client
    pub fn client(&self) -> &S3Client {
        &self.client
    }

    /// Get a clone of the S3 client
    pub fn client_clone(&self) -> S3Client {
        self.client.clone()
    }

    /// Get the MinIO endpoint URL
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Get the bucket name
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    /// Get host and port as a tuple
    pub async fn host_port(&self) -> Result<(String, u16)> {
        let host = self
            .container
            .get_host()
            .await
            .context("Failed to get host")?;
        let port = self
            .container
            .get_host_port_ipv4(9000.tcp())
            .await
            .context("Failed to get port")?;
        Ok((host.to_string(), port))
    }

    /// Upload bytes to S3
    ///
    /// Convenience method for uploading test data.
    pub async fn upload(&self, key: &str, data: Vec<u8>) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data.into())
            .send()
            .await
            .context("Failed to upload to S3")?;
        Ok(())
    }

    /// Download bytes from S3
    ///
    /// Convenience method for downloading test data.
    pub async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context("Failed to download from S3")?;

        let bytes = response
            .body
            .collect()
            .await
            .context("Failed to read S3 response body")?
            .into_bytes();

        Ok(bytes.to_vec())
    }

    /// List objects with optional prefix
    pub async fn list_objects(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let mut request = self.client.list_objects_v2().bucket(&self.bucket);

        if let Some(p) = prefix {
            request = request.prefix(p);
        }

        let response = request.send().await.context("Failed to list S3 objects")?;

        let keys = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(String::from))
            .collect();

        Ok(keys)
    }
}

/// Configuration options for MinIO test container
pub struct MinioOptions {
    /// MinIO version/tag (default: "latest")
    pub version: String,
    /// Access key (default: "minioadmin")
    pub access_key: String,
    /// Secret key (default: "minioadmin")
    pub secret_key: String,
    /// Bucket name to create (default: "bdp-test-data")
    pub bucket_name: String,
    /// Whether to create the bucket on startup (default: true)
    pub create_bucket: bool,
}

impl Default for MinioOptions {
    fn default() -> Self {
        Self {
            version: "latest".to_string(),
            access_key: MINIO_ACCESS_KEY.to_string(),
            secret_key: MINIO_SECRET_KEY.to_string(),
            bucket_name: DEFAULT_TEST_BUCKET.to_string(),
            create_bucket: true,
        }
    }
}

impl MinioOptions {
    /// Set the bucket name
    pub fn with_bucket(mut self, bucket: impl Into<String>) -> Self {
        self.bucket_name = bucket.into();
        self
    }

    /// Set the MinIO version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Disable automatic bucket creation
    pub fn without_bucket_creation(mut self) -> Self {
        self.create_bucket = false;
        self
    }
}

// ============================================================================
// Complete Test Environment
// ============================================================================

/// Complete test environment with PostgreSQL and MinIO
///
/// Provides both database and S3 storage for integration tests.
///
/// # Example
///
/// ```no_run
/// use common::TestEnvironment;
///
/// #[tokio::test]
/// async fn test_full_integration() {
///     let env = TestEnvironment::start().await.unwrap();
///
///     // Use PostgreSQL
///     let pool = env.db_pool();
///     sqlx::query("SELECT 1").execute(pool).await.unwrap();
///
///     // Use MinIO/S3
///     env.minio().upload("test.txt", b"Hello".to_vec()).await.unwrap();
/// }
/// ```
pub struct TestEnvironment {
    postgres: TestPostgres,
    minio: TestMinio,
}

impl TestEnvironment {
    /// Start a complete test environment
    ///
    /// # Returns
    ///
    /// Returns a `TestEnvironment` with both PostgreSQL (with migrations) and MinIO ready.
    pub async fn start() -> Result<Self> {
        Self::start_with_options(PostgresOptions::default(), MinioOptions::default()).await
    }

    /// Start with custom options
    ///
    /// # Arguments
    ///
    /// * `pg_options` - PostgreSQL configuration
    /// * `minio_options` - MinIO configuration
    pub async fn start_with_options(
        pg_options: PostgresOptions,
        minio_options: MinioOptions,
    ) -> Result<Self> {
        info!("Starting complete test environment...");

        // Start both containers in parallel for faster startup
        let (postgres, minio) = tokio::try_join!(
            TestPostgres::start_with_options(pg_options),
            TestMinio::start_with_options(minio_options),
        )?;

        info!("Test environment ready");

        Ok(Self { postgres, minio })
    }

    /// Get the PostgreSQL test container
    pub fn postgres(&self) -> &TestPostgres {
        &self.postgres
    }

    /// Get the MinIO test container
    pub fn minio(&self) -> &TestMinio {
        &self.minio
    }

    /// Get the database pool
    pub fn db_pool(&self) -> &PgPool {
        self.postgres.pool()
    }

    /// Get a clone of the database pool
    pub fn db_pool_clone(&self) -> PgPool {
        self.postgres.pool_clone()
    }

    /// Get the S3 client
    pub fn s3_client(&self) -> &S3Client {
        self.minio.client()
    }

    /// Get the S3 bucket name
    pub fn s3_bucket(&self) -> &str {
        self.minio.bucket()
    }

    /// Get the PostgreSQL connection string
    pub fn postgres_connection_string(&self) -> &str {
        self.postgres.connection_string()
    }

    /// Get the MinIO endpoint
    pub fn minio_endpoint(&self) -> &str {
        self.minio.endpoint()
    }
}

// ============================================================================
// Test Data Helpers
// ============================================================================

/// Helper for creating test data in the database
pub struct TestDataHelper<'a> {
    pool: &'a PgPool,
}

impl<'a> TestDataHelper<'a> {
    /// Create a new test data helper
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Create a test organization
    pub async fn create_organization(&self, slug: &str, name: &str) -> Result<uuid::Uuid> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO organizations (slug, name, description, is_system)
            VALUES ($1, $2, $3, false)
            RETURNING id
            "#,
            slug,
            name,
            format!("Test organization: {}", name)
        )
        .fetch_one(self.pool)
        .await
        .context("Failed to create organization")?;

        Ok(id)
    }

    /// Create a test registry entry
    pub async fn create_registry_entry(
        &self,
        organization_id: uuid::Uuid,
        slug: &str,
        name: &str,
        entry_type: &str,
    ) -> Result<uuid::Uuid> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
            organization_id,
            slug,
            name,
            entry_type
        )
        .fetch_one(self.pool)
        .await
        .context("Failed to create registry entry")?;

        Ok(id)
    }

    /// Create a test version
    pub async fn create_version(&self, entry_id: uuid::Uuid, version: &str) -> Result<uuid::Uuid> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO versions (entry_id, version, status)
            VALUES ($1, $2, 'published')
            RETURNING id
            "#,
            entry_id,
            version
        )
        .fetch_one(self.pool)
        .await
        .context("Failed to create version")?;

        Ok(id)
    }

    /// Create a complete test dataset (organization + entry + version)
    pub async fn create_test_dataset(
        &self,
        org_slug: &str,
        entry_slug: &str,
        version: &str,
    ) -> Result<(uuid::Uuid, uuid::Uuid, uuid::Uuid)> {
        let org_id = self
            .create_organization(org_slug, &format!("{} Organization", org_slug))
            .await?;

        let entry_id = self
            .create_registry_entry(
                org_id,
                entry_slug,
                &format!("{} Dataset", entry_slug),
                "data_source",
            )
            .await?;

        let version_id = self.create_version(entry_id, version).await?;

        Ok((org_id, entry_id, version_id))
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Initialize tracing for tests
///
/// Call this at the start of your test to enable logging.
pub fn init_test_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let _ = fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("info,bdp_server=debug,sqlx=warn,testcontainers=info")
        }))
        .with_test_writer()
        .try_init();
}

/// Check if Docker is available
///
/// Returns true if Docker daemon is running and accessible.
pub fn is_docker_available() -> bool {
    std::process::Command::new("docker")
        .arg("info")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Skip test if Docker is not available
///
/// Use this macro at the start of tests that require Docker:
///
/// ```no_run
/// #[tokio::test]
/// async fn test_with_docker() {
///     skip_if_no_docker!();
///     // ... rest of test
/// }
/// ```
#[macro_export]
macro_rules! skip_if_no_docker {
    () => {
        if !$crate::common::is_docker_available() {
            eprintln!("Skipping test: Docker is not available");
            return;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postgres_options_default() {
        let opts = PostgresOptions::default();
        assert_eq!(opts.version, "16-alpine");
        assert_eq!(opts.max_connections, 5);
        assert!(opts.run_migrations);
    }

    #[test]
    fn test_minio_options_default() {
        let opts = MinioOptions::default();
        assert_eq!(opts.bucket_name, DEFAULT_TEST_BUCKET);
        assert!(opts.create_bucket);
    }

    #[test]
    fn test_options_builder() {
        let pg_opts = PostgresOptions::default()
            .with_version("15-alpine")
            .with_max_connections(10);
        assert_eq!(pg_opts.version, "15-alpine");
        assert_eq!(pg_opts.max_connections, 10);

        let minio_opts = MinioOptions::default()
            .with_bucket("custom-bucket")
            .without_bucket_creation();
        assert_eq!(minio_opts.bucket_name, "custom-bucket");
        assert!(!minio_opts.create_bucket);
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn test_postgres_container() {
        init_test_tracing();

        let pg = TestPostgres::start()
            .await
            .expect("Failed to start PostgreSQL");
        let pool = pg.pool();

        // Verify connection works
        let result: (i32,) = sqlx::query_as("SELECT 1 as value")
            .fetch_one(pool)
            .await
            .expect("Query failed");
        assert_eq!(result.0, 1);

        // Verify migrations ran (check for a known table)
        let tables_exist = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 'organizations')"
        )
        .fetch_one(pool)
        .await
        .expect("Failed to check tables");
        assert!(
            tables_exist.unwrap_or(false),
            "Organizations table should exist after migrations"
        );
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn test_minio_container() {
        init_test_tracing();

        let minio = TestMinio::start().await.expect("Failed to start MinIO");

        // Upload test data
        minio
            .upload("test-file.txt", b"Hello, World!".to_vec())
            .await
            .expect("Failed to upload");

        // Download and verify
        let data = minio
            .download("test-file.txt")
            .await
            .expect("Failed to download");
        assert_eq!(data, b"Hello, World!");

        // List objects
        let objects = minio.list_objects(None).await.expect("Failed to list");
        assert!(objects.contains(&"test-file.txt".to_string()));
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn test_full_environment() {
        init_test_tracing();

        let env = TestEnvironment::start()
            .await
            .expect("Failed to start environment");

        // Test PostgreSQL
        let pool = env.db_pool();
        let result: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(pool)
            .await
            .expect("PostgreSQL query failed");
        assert_eq!(result.0, 1);

        // Test MinIO
        env.minio()
            .upload("test.txt", b"test data".to_vec())
            .await
            .expect("MinIO upload failed");
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn test_data_helper() {
        init_test_tracing();

        let pg = TestPostgres::start()
            .await
            .expect("Failed to start PostgreSQL");
        let helper = TestDataHelper::new(pg.pool());

        let (org_id, entry_id, version_id) = helper
            .create_test_dataset("test-org", "test-entry", "1.0.0")
            .await
            .expect("Failed to create test dataset");

        // Verify the data was created
        let org_exists =
            sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM organizations WHERE id = $1)", org_id)
                .fetch_one(pg.pool())
                .await
                .expect("Query failed");
        assert!(org_exists.unwrap_or(false));

        let entry_exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM registry_entries WHERE id = $1)",
            entry_id
        )
        .fetch_one(pg.pool())
        .await
        .expect("Query failed");
        assert!(entry_exists.unwrap_or(false));

        let version_exists =
            sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM versions WHERE id = $1)", version_id)
                .fetch_one(pg.pool())
                .await
                .expect("Query failed");
        assert!(version_exists.unwrap_or(false));
    }
}
