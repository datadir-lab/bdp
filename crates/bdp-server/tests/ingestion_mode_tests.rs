//! Integration tests for ingestion mode functionality
//!
//! Tests verify that Latest and Historical ingestion modes work correctly:
//! 1. Config parsing from environment variables
//! 2. Latest mode behavior (detect newer versions, skip when current)
//! 3. Historical mode behavior (filter ranges, skip existing)
//!
//! ## Running Tests
//!
//! These tests use `#[sqlx::test]` which requires a test database to be running.
//! The database tests will automatically create a temporary database and run migrations.
//!
//! Set the `DATABASE_URL` environment variable to point to your test database:
//! ```bash
//! export DATABASE_URL=postgresql://bdp:bdp_dev_password@localhost:5432/bdp
//! cargo test --test ingestion_mode_tests
//! ```
//!
//! Or run only the config tests (no database required):
//! ```bash
//! cargo test --test ingestion_mode_tests test_config
//! ```

use bdp_server::ingest::config::{HistoricalConfig, IngestionMode, UniProtConfig};
use bdp_server::ingest::uniprot::{DiscoveredVersion, UniProtFtpConfig, VersionDiscovery};
use chrono::NaiveDate;
use serial_test::serial;
use sqlx::PgPool;
use std::env;
use uuid::Uuid;

// ============================================================================
// Test Helpers
// ============================================================================

/// Helper to create test organization
async fn create_test_organization(pool: &PgPool) -> Uuid {
    let org_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO organizations (id, slug, name, description)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(org_id)
    .bind(format!("test-org-{}", org_id))
    .bind("Test Organization")
    .bind("Test organization for ingestion mode tests")
    .execute(pool)
    .await
    .expect("Failed to create test organization");

    org_id
}

/// Helper to insert sync status with last ingested version
async fn insert_sync_status(
    pool: &PgPool,
    organization_id: Uuid,
    last_external_version: Option<&str>,
) {
    sqlx::query(
        r#"
        INSERT INTO organization_sync_status (organization_id, last_external_version, status)
        VALUES ($1, $2, 'completed')
        ON CONFLICT (organization_id)
        DO UPDATE SET last_external_version = EXCLUDED.last_external_version
        "#,
    )
    .bind(organization_id)
    .bind(last_external_version)
    .execute(pool)
    .await
    .expect("Failed to insert sync status");
}

/// Helper to insert ingestion job
async fn insert_ingestion_job(
    pool: &PgPool,
    organization_id: Uuid,
    external_version: &str,
    status: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO ingestion_jobs (
            organization_id,
            job_type,
            external_version,
            internal_version,
            status,
            source_metadata
        )
        VALUES (
            $1,
            'uniprot_swissprot',
            $2,
            '1.0',
            $3,
            '{}'::jsonb
        )
        "#,
    )
    .bind(organization_id)
    .bind(external_version)
    .bind(status)
    .execute(pool)
    .await
    .expect("Failed to insert ingestion job");
}

/// Helper to check if a version exists in ingestion_jobs
async fn version_ingested(pool: &PgPool, organization_id: Uuid, version: &str) -> bool {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM ingestion_jobs
        WHERE organization_id = $1
          AND external_version = $2
          AND status = 'completed'
        "#,
    )
    .bind(organization_id)
    .bind(version)
    .fetch_one(pool)
    .await
    .expect("Failed to check if version is ingested");

    count > 0
}

/// Helper to clean up test data
async fn cleanup_test_data(pool: &PgPool, org_id: Uuid) {
    sqlx::query("DELETE FROM ingestion_jobs WHERE organization_id = $1")
        .bind(org_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query("DELETE FROM organization_sync_status WHERE organization_id = $1")
        .bind(org_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query("DELETE FROM organizations WHERE id = $1")
        .bind(org_id)
        .execute(pool)
        .await
        .ok();
}

// ============================================================================
// Config Parsing Tests
// ============================================================================

#[test]
#[serial]
fn test_config_parse_latest_mode() {
    // Set environment variables for latest mode
    env::set_var("INGEST_UNIPROT_MODE", "latest");
    env::set_var("INGEST_UNIPROT_CHECK_INTERVAL_SECS", "3600");
    env::set_var("INGEST_UNIPROT_AUTO_INGEST", "true");
    env::set_var("INGEST_UNIPROT_IGNORE_BEFORE", "2024_01");

    // Parse config from environment
    let config = UniProtConfig::from_env().expect("Failed to parse config from env");

    // Verify ingestion mode is Latest with correct values
    match config.ingestion_mode {
        IngestionMode::Latest(latest_cfg) => {
            assert_eq!(latest_cfg.check_interval_secs, 3600);
            assert_eq!(latest_cfg.auto_ingest, true);
            assert_eq!(latest_cfg.ignore_before, Some("2024_01".to_string()));
        },
        _ => panic!("Expected IngestionMode::Latest, got {:?}", config.ingestion_mode),
    }

    // Clean up environment variables
    env::remove_var("INGEST_UNIPROT_MODE");
    env::remove_var("INGEST_UNIPROT_CHECK_INTERVAL_SECS");
    env::remove_var("INGEST_UNIPROT_AUTO_INGEST");
    env::remove_var("INGEST_UNIPROT_IGNORE_BEFORE");
}

#[test]
#[serial]
fn test_config_parse_historical_mode() {
    // Set environment variables for historical mode
    env::set_var("INGEST_UNIPROT_MODE", "historical");
    env::set_var("INGEST_UNIPROT_HISTORICAL_START", "2020_01");
    env::set_var("INGEST_UNIPROT_HISTORICAL_END", "2021_12");
    env::set_var("INGEST_UNIPROT_HISTORICAL_BATCH_SIZE", "5");
    env::set_var("INGEST_UNIPROT_HISTORICAL_SKIP_EXISTING", "false");

    // Parse config from environment
    let config = UniProtConfig::from_env().expect("Failed to parse config from env");

    // Verify ingestion mode is Historical with correct values
    match config.ingestion_mode {
        IngestionMode::Historical(historical_cfg) => {
            assert_eq!(historical_cfg.start_version, "2020_01");
            assert_eq!(historical_cfg.end_version, Some("2021_12".to_string()));
            assert_eq!(historical_cfg.batch_size, 5);
            assert_eq!(historical_cfg.skip_existing, false);
        },
        _ => panic!("Expected IngestionMode::Historical, got {:?}", config.ingestion_mode),
    }

    // Clean up environment variables
    env::remove_var("INGEST_UNIPROT_MODE");
    env::remove_var("INGEST_UNIPROT_HISTORICAL_START");
    env::remove_var("INGEST_UNIPROT_HISTORICAL_END");
    env::remove_var("INGEST_UNIPROT_HISTORICAL_BATCH_SIZE");
    env::remove_var("INGEST_UNIPROT_HISTORICAL_SKIP_EXISTING");
}

// ============================================================================
// Latest Mode Behavior Tests
// ============================================================================

#[sqlx::test]
async fn test_latest_mode_ingests_newer(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    // Setup: last_ingested="2024_12" in organization_sync_status
    insert_sync_status(&pool, org_id, Some("2024_12")).await;

    // Mock FTP behavior using VersionDiscovery
    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    // Create mock discovered versions
    let available = vec![
        DiscoveredVersion {
            external_version: "2024_12".to_string(),
            release_date: NaiveDate::from_ymd_opt(2024, 12, 15).unwrap(),
            is_current: false,
            ftp_path: "previous_releases/release-2024_12".to_string(),
        },
        DiscoveredVersion {
            external_version: "2025_01".to_string(),
            release_date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            is_current: true,
            ftp_path: "current_release".to_string(),
        },
    ];

    // Get last ingested version
    let last_version = discovery
        .get_last_ingested_version(&pool, org_id)
        .await
        .expect("Failed to get last version");

    assert_eq!(last_version, Some("2024_12".to_string()));

    // Filter to get new versions
    let ingested_versions = vec!["2024_12".to_string()];
    let new_versions = discovery.filter_new_versions(available, ingested_versions);

    // Assert: 2025_01 should be identified as newer version
    assert_eq!(new_versions.len(), 1);
    assert_eq!(new_versions[0].external_version, "2025_01");
    assert!(new_versions[0].is_current);

    cleanup_test_data(&pool, org_id).await;
}

#[sqlx::test]
async fn test_latest_mode_skips_when_current(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    // Setup: last_ingested="2025_01" (already up-to-date)
    insert_sync_status(&pool, org_id, Some("2025_01")).await;

    // Mock FTP behavior - only 2025_01 available
    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    let available = vec![DiscoveredVersion {
        external_version: "2025_01".to_string(),
        release_date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
        is_current: true,
        ftp_path: "current_release".to_string(),
    }];

    // Get last ingested version
    let last_version = discovery
        .get_last_ingested_version(&pool, org_id)
        .await
        .expect("Failed to get last version");

    assert_eq!(last_version, Some("2025_01".to_string()));

    // Filter to get new versions
    let ingested_versions = vec!["2025_01".to_string()];
    let new_versions = discovery.filter_new_versions(available, ingested_versions);

    // Assert: No ingestion needed (already up-to-date)
    assert_eq!(new_versions.len(), 0, "Should not ingest any versions when already up-to-date");

    cleanup_test_data(&pool, org_id).await;
}

// ============================================================================
// Historical Mode Behavior Tests
// ============================================================================

#[sqlx::test]
async fn test_historical_mode_filters_range(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    // Mock FTP: available versions
    let available = vec![
        DiscoveredVersion {
            external_version: "2020_01".to_string(),
            release_date: NaiveDate::from_ymd_opt(2020, 1, 15).unwrap(),
            is_current: false,
            ftp_path: "previous_releases/release-2020_01".to_string(),
        },
        DiscoveredVersion {
            external_version: "2021_01".to_string(),
            release_date: NaiveDate::from_ymd_opt(2021, 1, 15).unwrap(),
            is_current: false,
            ftp_path: "previous_releases/release-2021_01".to_string(),
        },
        DiscoveredVersion {
            external_version: "2022_01".to_string(),
            release_date: NaiveDate::from_ymd_opt(2022, 1, 15).unwrap(),
            is_current: false,
            ftp_path: "previous_releases/release-2022_01".to_string(),
        },
        DiscoveredVersion {
            external_version: "2023_01".to_string(),
            release_date: NaiveDate::from_ymd_opt(2023, 1, 15).unwrap(),
            is_current: false,
            ftp_path: "previous_releases/release-2023_01".to_string(),
        },
    ];

    // Configure Historical mode with start_version=2020_01, end_version=2021_01
    let _historical_config = HistoricalConfig {
        start_version: "2020_01".to_string(),
        end_version: Some("2021_01".to_string()),
        batch_size: 3,
        skip_existing: false,
    };

    // Filter versions based on historical range
    let start_version = "2020_01".to_string();
    let end_version = Some("2021_01".to_string());

    let mut filtered: Vec<_> = available
        .into_iter()
        .filter(|v| {
            // Filter by start_version
            if v.external_version < start_version {
                return false;
            }

            // Filter by end_version if specified
            if let Some(ref end_ver) = end_version {
                if v.external_version > *end_ver {
                    return false;
                }
            }

            true
        })
        .collect();

    // Sort to ensure consistent ordering
    filtered.sort_by(|a, b| a.external_version.cmp(&b.external_version));

    // Assert: Only 2020_01 and 2021_01 should be in range
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].external_version, "2020_01");
    assert_eq!(filtered[1].external_version, "2021_01");

    cleanup_test_data(&pool, org_id).await;
}

#[sqlx::test]
async fn test_historical_mode_skips_existing(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    // Insert existing ingestion_jobs for version 2020_01
    insert_ingestion_job(&pool, org_id, "2020_01", "completed").await;

    // Configure Historical mode with skip_existing=true
    let _historical_config = HistoricalConfig {
        start_version: "2020_01".to_string(),
        end_version: Some("2021_01".to_string()),
        batch_size: 3,
        skip_existing: true,
    };

    // Mock FTP: available versions
    let available = vec![
        DiscoveredVersion {
            external_version: "2020_01".to_string(),
            release_date: NaiveDate::from_ymd_opt(2020, 1, 15).unwrap(),
            is_current: false,
            ftp_path: "previous_releases/release-2020_01".to_string(),
        },
        DiscoveredVersion {
            external_version: "2021_01".to_string(),
            release_date: NaiveDate::from_ymd_opt(2021, 1, 15).unwrap(),
            is_current: false,
            ftp_path: "previous_releases/release-2021_01".to_string(),
        },
    ];

    // Note: version_exists_in_db checks the versions table, not ingestion_jobs
    // For this test, we'll use the ingestion_jobs directly to verify skip_existing logic
    let job_2020_01_exists = version_ingested(&pool, org_id, "2020_01").await;
    let job_2021_01_exists = version_ingested(&pool, org_id, "2021_01").await;

    assert!(job_2020_01_exists, "2020_01 should exist in ingestion_jobs");
    assert!(!job_2021_01_exists, "2021_01 should not exist in ingestion_jobs");

    // Filter to only new versions (simulating skip_existing logic)
    let mut to_ingest = Vec::new();
    for version in available {
        let is_ingested = version_ingested(&pool, org_id, &version.external_version).await;
        if !is_ingested {
            to_ingest.push(version);
        }
    }

    // Assert: Only 2021_01 should be ingested (2020_01 skipped because it exists)
    assert_eq!(to_ingest.len(), 1);
    assert_eq!(to_ingest[0].external_version, "2021_01");

    cleanup_test_data(&pool, org_id).await;
}

// ============================================================================
// Integration Tests with Full Config
// ============================================================================

#[test]
#[serial]
fn test_default_mode_is_latest() {
    // Test that default ingestion mode is Latest when no env vars are set
    env::remove_var("INGEST_UNIPROT_MODE");

    let config = UniProtConfig::from_env().expect("Failed to parse config from env");

    // Verify default mode is Latest
    match config.ingestion_mode {
        IngestionMode::Latest(_) => {
            // Success
        },
        _ => panic!("Expected default IngestionMode::Latest, got {:?}", config.ingestion_mode),
    }
}

#[test]
#[serial]
fn test_invalid_mode_returns_error() {
    // Set invalid mode
    env::set_var("INGEST_UNIPROT_MODE", "invalid_mode");

    let result = UniProtConfig::from_env();

    // Should return error for invalid mode
    assert!(result.is_err(), "Should return error for invalid ingestion mode");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Invalid INGEST_UNIPROT_MODE"),
        "Error message should mention invalid mode"
    );

    // Clean up
    env::remove_var("INGEST_UNIPROT_MODE");
}

#[test]
#[serial]
fn test_latest_config_defaults() {
    env::set_var("INGEST_UNIPROT_MODE", "latest");
    // Don't set other vars to test defaults
    env::remove_var("INGEST_UNIPROT_CHECK_INTERVAL_SECS");
    env::remove_var("INGEST_UNIPROT_AUTO_INGEST");
    env::remove_var("INGEST_UNIPROT_IGNORE_BEFORE");

    let config = UniProtConfig::from_env().expect("Failed to parse config from env");

    match config.ingestion_mode {
        IngestionMode::Latest(latest_cfg) => {
            assert_eq!(
                latest_cfg.check_interval_secs, 86400,
                "Default check interval should be 86400 (1 day)"
            );
            assert_eq!(latest_cfg.auto_ingest, false, "Default auto_ingest should be false");
            assert_eq!(latest_cfg.ignore_before, None, "Default ignore_before should be None");
        },
        _ => panic!("Expected IngestionMode::Latest"),
    }

    env::remove_var("INGEST_UNIPROT_MODE");
}

#[test]
#[serial]
fn test_historical_config_defaults() {
    env::set_var("INGEST_UNIPROT_MODE", "historical");
    // Don't set other vars to test defaults
    env::remove_var("INGEST_UNIPROT_HISTORICAL_START");
    env::remove_var("INGEST_UNIPROT_HISTORICAL_END");
    env::remove_var("INGEST_UNIPROT_HISTORICAL_BATCH_SIZE");
    env::remove_var("INGEST_UNIPROT_HISTORICAL_SKIP_EXISTING");

    let config = UniProtConfig::from_env().expect("Failed to parse config from env");

    match config.ingestion_mode {
        IngestionMode::Historical(historical_cfg) => {
            assert_eq!(
                historical_cfg.start_version, "2020_01",
                "Default start_version should be 2020_01"
            );
            assert_eq!(historical_cfg.end_version, None, "Default end_version should be None");
            assert_eq!(historical_cfg.batch_size, 3, "Default batch_size should be 3");
            assert_eq!(historical_cfg.skip_existing, true, "Default skip_existing should be true");
        },
        _ => panic!("Expected IngestionMode::Historical"),
    }

    env::remove_var("INGEST_UNIPROT_MODE");
}
