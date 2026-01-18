//! Pipeline version deduplication tests
//!
//! Tests that the pipeline correctly:
//! 1. Extracts actual version from release notes
//! 2. Checks if version exists before downloading
//! 3. Skips re-downloading existing versions

use anyhow::Result;
use bdp_server::ingest::uniprot::{ReleaseType, UniProtFtpConfig, UniProtPipeline};
use serial_test::serial;
use sqlx::postgres::PgPoolOptions;
use testcontainers::{runners::AsyncRunner, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tracing::info;
use uuid::Uuid;

/// Initialize tracing for tests
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let _ = fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,bdp_server=debug")),
        )
        .with_test_writer()
        .try_init();
}

#[tokio::test]
#[serial]
async fn test_pipeline_extracts_version_from_current() -> Result<()> {
    init_tracing();
    info!("🧪 Testing pipeline extracts version from current release");

    // Setup test database
    let postgres_container = Postgres::default()
        .with_tag("16-alpine")
        .start()
        .await?;

    let host = postgres_container.get_host().await?;
    let port = postgres_container.get_host_port_ipv4(5432).await?;
    let conn_string = format!("postgresql://postgres:postgres@{}:{}/postgres", host, port);

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&conn_string)
        .await?;

    // Run migrations
    sqlx::migrate!("../../migrations").run(&db_pool).await?;

    // Create test organization
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, 'uniprot', 'UniProt', true)")
        .bind(org_id)
        .execute(&db_pool)
        .await?;

    // Configure for current release with small limit
    let config = UniProtFtpConfig::default()
        .with_release_type(ReleaseType::Current)
        .with_parse_limit(3);

    let pipeline = UniProtPipeline::new(db_pool.clone(), org_id, config);

    // Get release info without downloading full dataset
    let release_info = pipeline.get_release_info(None).await?;

    info!("✅ Extracted version: {}", release_info.external_version);
    assert!(!release_info.external_version.is_empty());
    assert!(release_info.external_version.contains("_"), "Version should be in YYYY_MM format");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_pipeline_version_deduplication() -> Result<()> {
    init_tracing();
    info!("🧪 Testing pipeline version deduplication");

    // Setup test database
    let postgres_container = Postgres::default()
        .with_tag("16-alpine")
        .start()
        .await?;

    let host = postgres_container.get_host().await?;
    let port = postgres_container.get_host_port_ipv4(5432).await?;
    let conn_string = format!("postgresql://postgres:postgres@{}:{}/postgres", host, port);

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&conn_string)
        .await?;

    // Run migrations
    sqlx::migrate!("../../migrations").run(&db_pool).await?;

    // Create test organization
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, 'uniprot', 'UniProt', true)")
        .bind(org_id)
        .execute(&db_pool)
        .await?;

    // Manually insert a version record to simulate existing data
    let test_version = "2024_01";
    let entry_id = Uuid::new_v4();

    // Create minimal registry entry
    sqlx::query(
        "INSERT INTO registry_entries (id, organization_id, slug, name, entry_type)
         VALUES ($1, $2, 'test-protein', 'Test Protein', 'data_source')"
    )
    .bind(entry_id)
    .bind(org_id)
    .execute(&db_pool)
    .await?;

    // Create version with the test version
    sqlx::query(
        "INSERT INTO versions (id, entry_id, version, external_version)
         VALUES ($1, $2, '1.0', $3)"
    )
    .bind(Uuid::new_v4())
    .bind(entry_id)
    .bind(test_version)
    .execute(&db_pool)
    .await?;

    info!("✅ Created test version: {}", test_version);

    // Configure pipeline for previous release (pointing to test_version)
    let config = UniProtFtpConfig::default()
        .with_release_type(ReleaseType::Previous)
        .with_parse_limit(5);

    let pipeline = UniProtPipeline::new(db_pool.clone(), org_id, config);

    // Run pipeline - should detect version exists and skip
    let stats = pipeline.run(Some(test_version)).await?;

    info!("✅ Pipeline stats:");
    info!("   Total entries: {}", stats.total_entries);
    info!("   Entries inserted: {}", stats.entries_inserted);

    // Should have skipped because version exists
    assert_eq!(stats.total_entries, 0, "Should not process any records for existing version");
    assert_eq!(stats.entries_inserted, 0, "Should not insert any records for existing version");

    info!("✅ Deduplication worked - existing version was skipped!");

    Ok(())
}

#[ctor::ctor]
fn init() {
    init_tracing();
}
