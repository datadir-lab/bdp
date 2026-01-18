//! Integration tests for UniProt idempotent ingestion pipeline
//!
//! Tests:
//! 1. Version discovery and filtering
//! 2. Idempotent behavior (don't re-download same version)
//! 3. "current" → versioned migration
//! 4. Real DAT parsing validation

use bdp_server::ingest::uniprot::{
    DiscoveredVersion, IdempotentUniProtPipeline, UniProtFtpConfig, VersionDiscovery,
};
use bdp_server::ingest::framework::BatchConfig;
use chrono::NaiveDate;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// Helper to create test database pool
async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_dev_password@localhost:5432/bdp".to_string());

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Helper to create test organization
async fn create_test_org(pool: &PgPool) -> Uuid {
    let org_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO organizations (id, slug, name)
        VALUES ($1, $2, $3)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(org_id)
    .bind(format!("test-org-{}", org_id))
    .bind("Test Organization")
    .execute(pool)
    .await
    .expect("Failed to create test organization");

    org_id
}

/// Helper to clean up test data
async fn cleanup_test_data(pool: &PgPool, org_id: Uuid) {
    sqlx::query("DELETE FROM ingestion_jobs WHERE organization_id = $1")
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

#[tokio::test]
async fn test_version_discovery_filters_ingested() {
    let pool = create_test_pool().await;
    let org_id = create_test_org(&pool).await;

    // Create mock discovered versions
    let discovered = vec![
        DiscoveredVersion {
            external_version: "2024_11".to_string(),
            release_date: NaiveDate::from_ymd_opt(2024, 11, 15).unwrap(),
            is_current: false,
            ftp_path: "previous_releases/release-2024_11".to_string(),
        },
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

    // Simulate that 2024_11 was already ingested
    let already_ingested = vec!["2024_11".to_string()];

    // Filter
    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);
    let new_versions = discovery.filter_new_versions(discovered, already_ingested);

    // Should only have 2024_12 and 2025_01
    assert_eq!(new_versions.len(), 2);
    assert_eq!(new_versions[0].external_version, "2024_12");
    assert_eq!(new_versions[1].external_version, "2025_01");

    cleanup_test_data(&pool, org_id).await;
}

#[tokio::test]
async fn test_idempotent_pipeline_skips_ingested() {
    let pool = Arc::new(create_test_pool().await);
    let org_id = create_test_org(&pool).await;

    // Create a completed ingestion job for version 2024_12
    sqlx::query(
        r#"
        INSERT INTO ingestion_jobs (
            id, organization_id, job_type, external_version,
            internal_version, status
        )
        VALUES ($1, $2, 'uniprot_swissprot', '2024_12', '1.0', 'completed')
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(org_id)
    .execute(&*pool)
    .await
    .expect("Failed to create test job");

    // Check if version is marked as ingested
    let config = UniProtFtpConfig::new();
    let batch_config = BatchConfig::default();
    let pipeline = IdempotentUniProtPipeline::new(pool.clone(), org_id, config, batch_config);

    let is_ingested = pipeline
        .is_version_ingested("2024_12")
        .await
        .expect("Failed to check ingestion status");

    assert!(is_ingested, "Version 2024_12 should be marked as ingested");

    let is_new_ingested = pipeline
        .is_version_ingested("2025_01")
        .await
        .expect("Failed to check ingestion status");

    assert!(!is_new_ingested, "Version 2025_01 should not be ingested yet");

    cleanup_test_data(&pool, org_id).await;
}

#[tokio::test]
async fn test_current_to_versioned_migration() {
    let pool = Arc::new(create_test_pool().await);
    let org_id = create_test_org(&pool).await;

    // Scenario: We ingested "2025_01" as current
    sqlx::query(
        r#"
        INSERT INTO ingestion_jobs (
            id, organization_id, job_type, external_version,
            internal_version, status, source_metadata
        )
        VALUES ($1, $2, 'uniprot_swissprot', '2025_01', '1.0', 'completed', $3)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(org_id)
    .bind(serde_json::json!({
        "is_current": true,
        "ftp_path": "current_release"
    }))
    .execute(&*pool)
    .await
    .expect("Failed to create test job");

    // Now "2025_01" has moved to previous_releases (new current is 2025_02)
    let discovered_old_as_previous = DiscoveredVersion {
        external_version: "2025_01".to_string(),
        release_date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
        is_current: false,
        ftp_path: "previous_releases/release-2025_01".to_string(),
    };

    // Check that we correctly identify this should NOT be re-ingested
    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    let should_reingest =
        discovery.should_reingest(&discovered_old_as_previous, "2025_01", true);

    assert!(
        !should_reingest,
        "Should NOT re-ingest version that just migrated from current to previous"
    );

    cleanup_test_data(&pool, org_id).await;
}

#[tokio::test]
async fn test_versions_processed_oldest_first() {
    let mut versions = vec![
        DiscoveredVersion {
            external_version: "2025_01".to_string(),
            release_date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            is_current: true,
            ftp_path: "current_release".to_string(),
        },
        DiscoveredVersion {
            external_version: "2024_11".to_string(),
            release_date: NaiveDate::from_ymd_opt(2024, 11, 15).unwrap(),
            is_current: false,
            ftp_path: "previous_releases/release-2024_11".to_string(),
        },
        DiscoveredVersion {
            external_version: "2024_12".to_string(),
            release_date: NaiveDate::from_ymd_opt(2024, 12, 15).unwrap(),
            is_current: false,
            ftp_path: "previous_releases/release-2024_12".to_string(),
        },
    ];

    // Sort (should be oldest first)
    versions.sort();

    assert_eq!(versions[0].external_version, "2024_11");
    assert_eq!(versions[1].external_version, "2024_12");
    assert_eq!(versions[2].external_version, "2025_01");
}

#[test]
fn test_idempotent_stats_calculation() {
    use bdp_server::ingest::uniprot::idempotent_pipeline::IdempotentStats;

    let stats = IdempotentStats {
        discovered_count: 3,
        already_ingested_count: 2,
        newly_ingested_count: 3,
        failed_count: 0,
    };

    assert_eq!(stats.total_versions(), 5);
    assert_eq!(stats.success_rate(), 100.0);
}
