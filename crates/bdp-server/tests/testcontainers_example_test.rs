//! Example integration tests using testcontainers
//!
//! This file demonstrates how to use the testcontainers infrastructure for
//! integration testing without manual database/service setup.
//!
//! # Running These Tests
//!
//! These tests require Docker to be running. Run with:
//!
//! ```bash
//! # Run all testcontainers tests
//! cargo test --test testcontainers_example_test -- --ignored --nocapture
//!
//! # Run a specific test
//! cargo test --test testcontainers_example_test test_database_operations -- --ignored --nocapture
//! ```
//!
//! # Prerequisites
//!
//! - Docker daemon running
//! - Sufficient disk space for container images
//! - Network access to pull container images (first run only)

mod common;

use common::{init_test_tracing, TestEnvironment, TestMinio, TestPostgres, TestDataHelper};
use anyhow::Result;

// ============================================================================
// PostgreSQL-only Tests
// ============================================================================

/// Example: Basic database connectivity test
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_database_connectivity() {
    init_test_tracing();

    // Start PostgreSQL container
    let pg = TestPostgres::start()
        .await
        .expect("Failed to start PostgreSQL container");

    // Verify basic connectivity
    let result: (i32,) = sqlx::query_as("SELECT 42 as answer")
        .fetch_one(pg.pool())
        .await
        .expect("Query failed");

    assert_eq!(result.0, 42);
}

/// Example: Test that migrations are applied correctly
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_migrations_applied() {
    init_test_tracing();

    let pg = TestPostgres::start()
        .await
        .expect("Failed to start PostgreSQL container");

    // Check that key tables exist after migrations
    let tables = vec![
        "organizations",
        "registry_entries",
        "versions",
        "version_files",
        "dependencies",
    ];

    for table in tables {
        let exists = sqlx::query_scalar(&format!(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = '{}')",
            table
        ))
        .fetch_one(pg.pool())
        .await
        .expect("Failed to check table existence");

        assert!(
            exists.unwrap_or(false),
            "Table '{}' should exist after migrations",
            table
        );
    }
}

/// Example: Test CRUD operations on organizations
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_organization_crud() {
    init_test_tracing();

    let pg = TestPostgres::start()
        .await
        .expect("Failed to start PostgreSQL container");
    let pool = pg.pool();

    // Create
    let org_id = sqlx::query_scalar!(
        r#"
        INSERT INTO organizations (slug, name, description, is_system)
        VALUES ('test-org', 'Test Organization', 'A test organization', false)
        RETURNING id
        "#
    )
    .fetch_one(pool)
    .await
    .expect("Failed to create organization");

    // Read
    let org = sqlx::query!(
        "SELECT slug, name, description FROM organizations WHERE id = $1",
        org_id
    )
    .fetch_one(pool)
    .await
    .expect("Failed to read organization");

    assert_eq!(org.slug, "test-org");
    assert_eq!(org.name, "Test Organization");

    // Update
    sqlx::query!(
        "UPDATE organizations SET name = 'Updated Organization' WHERE id = $1",
        org_id
    )
    .execute(pool)
    .await
    .expect("Failed to update organization");

    let updated = sqlx::query!("SELECT name FROM organizations WHERE id = $1", org_id)
        .fetch_one(pool)
        .await
        .expect("Failed to read updated organization");

    assert_eq!(updated.name, "Updated Organization");

    // Delete
    sqlx::query!("DELETE FROM organizations WHERE id = $1", org_id)
        .execute(pool)
        .await
        .expect("Failed to delete organization");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organizations")
        .fetch_one(pool)
        .await
        .expect("Failed to count organizations");

    assert_eq!(count, 0);
}

/// Example: Test using the TestDataHelper for quick test data setup
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_with_data_helper() {
    init_test_tracing();

    let pg = TestPostgres::start()
        .await
        .expect("Failed to start PostgreSQL container");
    let helper = TestDataHelper::new(pg.pool());

    // Create a complete test dataset in one call
    let (org_id, entry_id, version_id) = helper
        .create_test_dataset("uniprot", "swissprot-human", "2024.01")
        .await
        .expect("Failed to create test dataset");

    // Verify the relationships
    let entry = sqlx::query!(
        "SELECT organization_id FROM registry_entries WHERE id = $1",
        entry_id
    )
    .fetch_one(pg.pool())
    .await
    .expect("Failed to fetch entry");

    assert_eq!(entry.organization_id, org_id);

    let version = sqlx::query!("SELECT entry_id FROM versions WHERE id = $1", version_id)
        .fetch_one(pg.pool())
        .await
        .expect("Failed to fetch version");

    assert_eq!(version.entry_id, entry_id);
}

// ============================================================================
// MinIO/S3-only Tests
// ============================================================================

/// Example: Basic S3 operations
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_s3_basic_operations() {
    init_test_tracing();

    let minio = TestMinio::start()
        .await
        .expect("Failed to start MinIO container");

    // Upload
    minio
        .upload("data/protein.fasta", b">P01308\nMALWMRLLPL...".to_vec())
        .await
        .expect("Failed to upload file");

    // Download
    let data = minio
        .download("data/protein.fasta")
        .await
        .expect("Failed to download file");

    assert!(String::from_utf8_lossy(&data).contains("P01308"));

    // List
    let objects = minio
        .list_objects(Some("data/"))
        .await
        .expect("Failed to list objects");

    assert_eq!(objects.len(), 1);
    assert!(objects.contains(&"data/protein.fasta".to_string()));
}

/// Example: Test S3 with multiple files
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_s3_multiple_files() {
    init_test_tracing();

    let minio = TestMinio::start()
        .await
        .expect("Failed to start MinIO container");

    // Upload multiple files
    let files = vec![
        ("proteins/human.fasta", b"Human proteins".to_vec()),
        ("proteins/mouse.fasta", b"Mouse proteins".to_vec()),
        ("metadata/readme.txt", b"Dataset readme".to_vec()),
    ];

    for (key, data) in &files {
        minio.upload(key, data.clone()).await.expect("Upload failed");
    }

    // List only proteins
    let protein_files = minio
        .list_objects(Some("proteins/"))
        .await
        .expect("Failed to list proteins");

    assert_eq!(protein_files.len(), 2);

    // List all
    let all_files = minio.list_objects(None).await.expect("Failed to list all");
    assert_eq!(all_files.len(), 3);
}

// ============================================================================
// Full Environment Tests
// ============================================================================

/// Example: Complete integration test with database and S3
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_full_environment() {
    init_test_tracing();

    let env = TestEnvironment::start()
        .await
        .expect("Failed to start test environment");

    // Create data source in database
    let helper = TestDataHelper::new(env.db_pool());
    let (org_id, entry_id, version_id) = helper
        .create_test_dataset("test-org", "test-data", "1.0")
        .await
        .expect("Failed to create test dataset");

    // Store file metadata in database
    let file_key = format!("{}/{}/{}/data.fasta", org_id, entry_id, version_id);
    let file_content = b"FASTA content here";

    // Upload to S3
    env.minio()
        .upload(&file_key, file_content.to_vec())
        .await
        .expect("Failed to upload to S3");

    // Verify S3 content
    let downloaded = env
        .minio()
        .download(&file_key)
        .await
        .expect("Failed to download from S3");

    assert_eq!(downloaded, file_content);

    // Verify database state
    let version = sqlx::query!("SELECT version FROM versions WHERE id = $1", version_id)
        .fetch_one(env.db_pool())
        .await
        .expect("Failed to fetch version");

    assert_eq!(version.version, "1.0");
}

/// Example: Test parallel container startup performance
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_parallel_startup() {
    init_test_tracing();

    let start = std::time::Instant::now();

    // TestEnvironment starts both containers in parallel
    let env = TestEnvironment::start()
        .await
        .expect("Failed to start environment");

    let startup_time = start.elapsed();

    // Verify both services are working
    let _: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(env.db_pool())
        .await
        .expect("PostgreSQL should be accessible");

    env.minio()
        .upload("test.txt", b"test".to_vec())
        .await
        .expect("MinIO should be accessible");

    // Log startup time for performance tracking
    tracing::info!(
        startup_time_ms = startup_time.as_millis(),
        "Test environment started"
    );

    // Parallel startup should typically take less than sequential
    // (This is more of a documentation than a hard assertion)
    assert!(
        startup_time.as_secs() < 120,
        "Startup should complete within 2 minutes"
    );
}

// ============================================================================
// Example: Converting an Ignored Test to Use Testcontainers
// ============================================================================

/// This example shows how to convert an ignored test that requires a database
/// to use testcontainers instead.
///
/// Before (would be marked #[ignore] and require manual database setup):
/// ```ignore
/// #[tokio::test]
/// #[ignore] // Requires database
/// async fn test_search_basic_query() {
///     let pool = get_pool_from_env().await; // Manual setup
///     // test code...
/// }
/// ```
///
/// After (uses testcontainers, runs anywhere with Docker):
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_search_basic_query_with_testcontainers() {
    init_test_tracing();

    let pg = TestPostgres::start()
        .await
        .expect("Failed to start PostgreSQL");
    let helper = TestDataHelper::new(pg.pool());

    // Set up test data
    let (_org_id, entry_id, _version_id) = helper
        .create_test_dataset("uniprot", "swissprot", "1.0")
        .await
        .expect("Failed to create test data");

    // Now run the actual test logic
    let results = sqlx::query!(
        r#"
        SELECT id, slug FROM registry_entries WHERE id = $1
        "#,
        entry_id
    )
    .fetch_all(pg.pool())
    .await
    .expect("Query failed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].slug, "swissprot");
}

// ============================================================================
// Test Isolation Example
// ============================================================================

/// Example: Verify that each test gets isolated containers
///
/// These two tests can run in parallel without interfering with each other
/// because each gets its own container instances.
mod isolation_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn test_isolation_a() {
        init_test_tracing();

        let pg = TestPostgres::start().await.expect("Failed to start PostgreSQL");
        let helper = TestDataHelper::new(pg.pool());

        // Create data specific to this test
        helper
            .create_organization("org-a", "Organization A")
            .await
            .expect("Failed to create org");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organizations")
            .fetch_one(pg.pool())
            .await
            .expect("Query failed");

        assert_eq!(count, 1, "Should have exactly one organization");
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn test_isolation_b() {
        init_test_tracing();

        let pg = TestPostgres::start().await.expect("Failed to start PostgreSQL");
        let helper = TestDataHelper::new(pg.pool());

        // Create different data in this test
        helper
            .create_organization("org-b", "Organization B")
            .await
            .expect("Failed to create org");

        // This test's database is completely separate from test_isolation_a
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organizations")
            .fetch_one(pg.pool())
            .await
            .expect("Query failed");

        assert_eq!(count, 1, "Should have exactly one organization");
    }
}
