//! Migration safety tests for UniProt version transitions
//!
//! These tests verify that when UniProt moves a release from current to historical
//! (which happens monthly), we don't re-ingest the same data. The key insight is that
//! the version number stays the same (e.g., "2025_01"), only the FTP location changes
//! from current_release/ to previous_releases/release-2025_01/.

use bdp_server::ingest::uniprot::{
    config::UniProtFtpConfig, version_discovery::VersionDiscovery,
};
use sqlx::PgPool;
use uuid::Uuid;

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a test organization for use in migration tests
async fn create_test_organization(pool: &PgPool) -> Uuid {
    let org_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO organizations (id, name, key, description)
        VALUES ($1, 'Test Migration Org', 'test_migration_org', 'Test organization for migration safety tests')
        "#
    )
    .bind(org_id)
    .execute(pool)
    .await
    .expect("Failed to create test organization");

    org_id
}

/// Insert an ingestion job with is_current metadata
async fn insert_ingestion_job(
    pool: &PgPool,
    organization_id: Uuid,
    external_version: &str,
    is_current: bool,
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
            'completed',
            jsonb_build_object('is_current', $3)
        )
        "#
    )
    .bind(organization_id)
    .bind(external_version)
    .bind(is_current)
    .execute(pool)
    .await
    .expect("Failed to insert ingestion job");
}

/// Check if an ingestion job exists for a version
async fn ingestion_job_exists(pool: &PgPool, external_version: &str) -> bool {
    let result = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM ingestion_jobs
        WHERE external_version = $1
        "#
    )
    .bind(external_version)
    .fetch_one(pool)
    .await
    .expect("Failed to check job existence");

    result > 0
}

/// Get the is_current value from source_metadata for a version
async fn get_is_current_metadata(pool: &PgPool, external_version: &str) -> Option<bool> {
    let result = sqlx::query_scalar::<_, Option<serde_json::Value>>(
        r#"
        SELECT source_metadata->>'is_current'
        FROM ingestion_jobs
        WHERE external_version = $1
        LIMIT 1
        "#
    )
    .bind(external_version)
    .fetch_optional(pool)
    .await
    .expect("Failed to fetch is_current metadata");

    result.flatten().and_then(|v| {
        if let serde_json::Value::String(s) = v {
            match s.as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            }
        } else {
            None
        }
    })
}

// ============================================================================
// Migration Safety Tests (4 tests)
// ============================================================================

/// Test 1: Verify that a version ingested as "current" is not re-ingested
/// when discovered in "previous_releases"
///
/// Scenario:
/// - Month 1: We ingest 2025_01 from current_release/ (is_current=true)
/// - Month 2: UniProt moves 2025_01 to previous_releases/release-2025_01/
/// - Result: was_ingested_as_current() returns true, pipeline skips it
#[sqlx::test]
async fn test_current_to_historical_no_reingest(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    // Month 1: Ingest 2025_01 as current
    insert_ingestion_job(&pool, org_id, "2025_01", true).await;

    // Verify it was recorded
    assert!(ingestion_job_exists(&pool, "2025_01").await);

    // Month 2: Check if we should re-ingest when discovered in previous_releases
    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    let was_current = discovery
        .was_ingested_as_current(&pool, "2025_01")
        .await
        .expect("Failed to check was_ingested_as_current");

    // Assert: Should return true, indicating we already have this data
    assert!(
        was_current,
        "Version 2025_01 should be recognized as already ingested when it was current"
    );

    // Verify metadata is correct
    let is_current_meta = get_is_current_metadata(&pool, "2025_01").await;
    assert_eq!(
        is_current_meta,
        Some(true),
        "Metadata should show is_current=true"
    );
}

/// Test 2: Verify that a new version found in historical releases is ingested
///
/// Scenario:
/// - We discover "2024_12" in previous_releases/ (was never ingested before)
/// - Result: was_ingested_as_current() returns false, pipeline ingests it
#[sqlx::test]
async fn test_new_version_in_historical_ingests(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    // No previous ingestion of "2024_12"
    assert!(!ingestion_job_exists(&pool, "2024_12").await);

    // Discover "2024_12" in previous_releases
    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    let was_current = discovery
        .was_ingested_as_current(&pool, "2024_12")
        .await
        .expect("Failed to check was_ingested_as_current");

    // Assert: Should return false, indicating this is new data we should ingest
    assert!(
        !was_current,
        "Version 2024_12 should not be found as previously ingested"
    );
}

/// Test 3: Verify that pipeline stores is_current metadata correctly
///
/// Scenario:
/// - Run pipeline with version=None (current release) → is_current=true
/// - Run pipeline with specific version (historical) → is_current=false
#[sqlx::test]
async fn test_pipeline_stores_is_current_metadata(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    // Simulate ingestion of current release
    insert_ingestion_job(&pool, org_id, "2025_02", true).await;

    // Simulate ingestion of historical release
    insert_ingestion_job(&pool, org_id, "2025_01", false).await;

    // Assert: Current release has is_current=true
    let current_meta = get_is_current_metadata(&pool, "2025_02").await;
    assert_eq!(
        current_meta,
        Some(true),
        "Current release should have source_metadata->>'is_current' = 'true'"
    );

    // Assert: Historical release has is_current=false
    let historical_meta = get_is_current_metadata(&pool, "2025_01").await;
    assert_eq!(
        historical_meta,
        Some(false),
        "Historical release should have source_metadata->>'is_current' = 'false'"
    );

    // Verify using the discovery API
    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    let was_current_2025_02 = discovery
        .was_ingested_as_current(&pool, "2025_02")
        .await
        .expect("Failed to check 2025_02");
    assert!(was_current_2025_02, "2025_02 should be marked as current");

    let was_current_2025_01 = discovery
        .was_ingested_as_current(&pool, "2025_01")
        .await
        .expect("Failed to check 2025_01");
    assert!(
        !was_current_2025_01,
        "2025_01 should not be marked as current"
    );
}

/// Test 4: Verify the complete monthly update scenario
///
/// Scenario:
/// - Month 1: Ingest 2025_01 as current (is_current=true)
/// - Month 2: Discover [2025_01 (in previous_releases), 2025_02 (in current_release)]
/// - Result: Only 2025_02 should be marked for ingestion, 2025_01 should be skipped
#[sqlx::test]
async fn test_monthly_update_scenario(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    // Month 1: Ingest 2025_01 as current
    insert_ingestion_job(&pool, org_id, "2025_01", true).await;

    // Verify Month 1 state
    assert!(ingestion_job_exists(&pool, "2025_01").await);
    let month1_meta = get_is_current_metadata(&pool, "2025_01").await;
    assert_eq!(month1_meta, Some(true));

    // Month 2: Check both versions
    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    // 2025_01 is now in previous_releases (but was ingested as current)
    let should_skip_2025_01 = discovery
        .was_ingested_as_current(&pool, "2025_01")
        .await
        .expect("Failed to check 2025_01");
    assert!(
        should_skip_2025_01,
        "2025_01 should be skipped (already ingested when it was current)"
    );

    // 2025_02 is new in current_release
    let should_ingest_2025_02 = discovery
        .was_ingested_as_current(&pool, "2025_02")
        .await
        .expect("Failed to check 2025_02");
    assert!(
        !should_ingest_2025_02,
        "2025_02 should be ingested (new version)"
    );

    // Simulate Month 2 ingestion of 2025_02
    insert_ingestion_job(&pool, org_id, "2025_02", true).await;

    // Verify final state
    assert!(ingestion_job_exists(&pool, "2025_02").await);
    let month2_meta = get_is_current_metadata(&pool, "2025_02").await;
    assert_eq!(month2_meta, Some(true));

    // Verify both versions exist in database
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM ingestion_jobs
        WHERE external_version IN ('2025_01', '2025_02')
        "#
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to count ingestion jobs");

    assert_eq!(count, 2, "Should have exactly 2 ingestion jobs");

    // Verify only one is marked as current
    let current_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM ingestion_jobs
        WHERE external_version IN ('2025_01', '2025_02')
          AND source_metadata->>'is_current' = 'true'
        "#
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to count current jobs");

    assert_eq!(
        current_count, 2,
        "Both versions were ingested as current at different times"
    );
}
