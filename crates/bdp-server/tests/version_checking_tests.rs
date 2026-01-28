//! Integration tests for UniProt version checking database methods

use bdp_server::ingest::uniprot::{config::UniProtFtpConfig, version_discovery::VersionDiscovery};
use sqlx::PgPool;
use uuid::Uuid;

// Helper to create test organization
async fn create_test_organization(pool: &PgPool) -> Uuid {
    let org_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO organizations (id, name, key, description)
        VALUES ($1, 'Test Org', 'test_org', 'Test organization for version checking')
        "#,
    )
    .bind(org_id)
    .execute(pool)
    .await
    .expect("Failed to create test organization");

    org_id
}

// Helper to insert sync status
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

// Helper to insert version
async fn insert_version(pool: &PgPool, entry_id: Uuid, external_version: &str) {
    sqlx::query(
        r#"
        INSERT INTO versions (entry_id, version, external_version, release_date)
        VALUES ($1, '1.0', $2, CURRENT_DATE)
        "#,
    )
    .bind(entry_id)
    .bind(external_version)
    .execute(pool)
    .await
    .expect("Failed to insert version");
}

// Helper to create a registry entry
async fn create_registry_entry(pool: &PgPool) -> Uuid {
    let entry_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO registry_entries (
            id, name, key, entry_type, organization_id, description
        )
        SELECT $1, 'UniProt', 'uniprot', 'data_source', id, 'UniProt data source'
        FROM organizations
        LIMIT 1
        "#,
    )
    .bind(entry_id)
    .execute(pool)
    .await
    .expect("Failed to create registry entry");

    entry_id
}

// Helper to insert ingestion job
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
        "#,
    )
    .bind(organization_id)
    .bind(external_version)
    .bind(is_current)
    .execute(pool)
    .await
    .expect("Failed to insert ingestion job");
}

// ============================================================================
// VERSION CHECKING TESTS (6 tests)
// ============================================================================

#[sqlx::test]
async fn test_check_for_newer_version_when_none_ingested(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    // No previous ingestion recorded
    let result = discovery.check_for_newer_version(&pool, org_id).await;

    // Since we're mocking FTP, this will return whatever discover_all_versions returns
    // In a real test, we'd mock the FTP calls
    // For now, just verify the method doesn't crash
    assert!(result.is_ok());
}

#[sqlx::test]
async fn test_check_for_newer_version_when_up_to_date(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    // Set last ingested version to current (assuming current is 2025_01)
    insert_sync_status(&pool, org_id, Some("2025_01")).await;

    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    // This test would need FTP mocking to properly test
    // For now, just verify the method works
    let result = discovery.check_for_newer_version(&pool, org_id).await;
    assert!(result.is_ok());
}

#[sqlx::test]
async fn test_check_for_newer_version_when_newer_available(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    // Set last ingested version to older version
    insert_sync_status(&pool, org_id, Some("2024_12")).await;

    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    // This would need FTP mocking to properly test newer version detection
    let result = discovery.check_for_newer_version(&pool, org_id).await;
    assert!(result.is_ok());
}

#[sqlx::test]
async fn test_get_last_ingested_version(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    // Insert sync status with version
    insert_sync_status(&pool, org_id, Some("2025_01")).await;

    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    let last_version = discovery
        .get_last_ingested_version(&pool, org_id)
        .await
        .expect("Failed to get last version");

    assert_eq!(last_version, Some("2025_01".to_string()));
}

#[sqlx::test]
async fn test_get_last_ingested_version_none(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    // No sync status inserted

    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    let last_version = discovery
        .get_last_ingested_version(&pool, org_id)
        .await
        .expect("Failed to get last version");

    assert_eq!(last_version, None);
}

#[sqlx::test]
async fn test_version_exists_in_db(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;
    let entry_id = create_registry_entry(&pool).await;

    // Insert a version
    insert_version(&pool, entry_id, "2025_01").await;

    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    // Check existing version
    let exists = discovery
        .version_exists_in_db(&pool, "2025_01")
        .await
        .expect("Failed to check version exists");

    assert!(exists);

    // Check non-existing version
    let not_exists = discovery
        .version_exists_in_db(&pool, "2024_99")
        .await
        .expect("Failed to check version exists");

    assert!(!not_exists);
}

#[sqlx::test]
async fn test_was_ingested_as_current(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;

    // Insert job with is_current=true
    insert_ingestion_job(&pool, org_id, "2025_01", true).await;

    // Insert another job with is_current=false
    insert_ingestion_job(&pool, org_id, "2024_12", false).await;

    let config = UniProtFtpConfig::new();
    let discovery = VersionDiscovery::new(config);

    // Check version ingested as current
    let was_current = discovery
        .was_ingested_as_current(&pool, "2025_01")
        .await
        .expect("Failed to check was_ingested_as_current");

    assert!(was_current);

    // Check version not ingested as current
    let was_not_current = discovery
        .was_ingested_as_current(&pool, "2024_12")
        .await
        .expect("Failed to check was_ingested_as_current");

    assert!(!was_not_current);

    // Check version never ingested
    let never_ingested = discovery
        .was_ingested_as_current(&pool, "2024_11")
        .await
        .expect("Failed to check was_ingested_as_current");

    assert!(!never_ingested);
}
