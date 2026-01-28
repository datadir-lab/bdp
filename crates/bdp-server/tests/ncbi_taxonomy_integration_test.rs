//! NCBI Taxonomy integration tests
//!
//! These tests require a PostgreSQL database to be running.
//! Run with: cargo test --test ncbi_taxonomy_integration_test -- --nocapture

use bdp_server::ingest::ncbi_taxonomy::{
    DeletedTaxon, MergedTaxon, NcbiTaxonomyFtpConfig, NcbiTaxonomyPipeline, NcbiTaxonomyStorage,
    StorageStats, TaxdumpData, TaxonomyEntry,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create a test database pool
async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/bdp_test".to_string());

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Helper to create a test organization
async fn create_test_org(pool: &PgPool) -> Uuid {
    let org_id = Uuid::new_v4();
    let slug = format!("test-org-{}", org_id);

    sqlx::query(
        r#"
        INSERT INTO organizations (id, slug, name, created_at, updated_at)
        VALUES ($1, $2, 'Test Organization', NOW(), NOW())
        ON CONFLICT (slug) DO NOTHING
        "#,
    )
    .bind(org_id)
    .bind(&slug)
    .execute(pool)
    .await
    .expect("Failed to create test organization");

    org_id
}

/// Helper to cleanup test data
async fn cleanup_test_data(pool: &PgPool, org_id: Uuid) {
    // Delete registry entries for this org (cascades to data_sources, versions, etc.)
    let _ = sqlx::query("DELETE FROM registry_entries WHERE organization_id = $1")
        .bind(org_id)
        .execute(pool)
        .await;

    // Delete organization
    let _ = sqlx::query("DELETE FROM organizations WHERE id = $1")
        .bind(org_id)
        .execute(pool)
        .await;
}

/// Create sample taxonomy data for testing
fn create_sample_taxdump() -> TaxdumpData {
    let entries = vec![
        TaxonomyEntry::new(
            9606,
            "Homo sapiens".to_string(),
            Some("human".to_string()),
            "species".to_string(),
            "Eukaryota;Metazoa;Chordata;Mammalia;Primates;Hominidae;Homo;Homo sapiens"
                .to_string(),
        ),
        TaxonomyEntry::new(
            10090,
            "Mus musculus".to_string(),
            Some("mouse".to_string()),
            "species".to_string(),
            "Eukaryota;Metazoa;Chordata;Mammalia;Rodentia;Muridae;Mus;Mus musculus".to_string(),
        ),
        TaxonomyEntry::new(
            7227,
            "Drosophila melanogaster".to_string(),
            Some("fruit fly".to_string()),
            "species".to_string(),
            "Eukaryota;Metazoa;Arthropoda;Insecta;Diptera;Drosophilidae;Drosophila;Drosophila melanogaster".to_string(),
        ),
    ];

    let merged = vec![MergedTaxon::new(12345, 9606)];
    let deleted = vec![DeletedTaxon::new(99999)];

    TaxdumpData::new(entries, merged, deleted, "2026-01-19".to_string())
}

#[tokio::test]
#[ignore] // Requires database
async fn test_storage_basic() {
    let pool = create_test_pool().await;
    let org_id = create_test_org(&pool).await;

    // Create sample data
    let taxdump = create_sample_taxdump();

    // Create storage handler
    let storage =
        NcbiTaxonomyStorage::new(pool.clone(), org_id, "1.0".to_string(), "2026-01-19".to_string());

    // Store data
    let stats = storage.store(&taxdump).await.expect("Storage failed");

    // Verify stats
    assert_eq!(stats.total, 3);
    assert_eq!(stats.stored, 3);
    assert_eq!(stats.updated, 0);
    assert_eq!(stats.failed, 0);

    // Verify data was stored
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM taxonomy_metadata tm
         JOIN registry_entries re ON tm.data_source_id = re.id
         WHERE re.organization_id = $1",
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("Failed to count entries");

    assert_eq!(count, 3);

    // Cleanup
    cleanup_test_data(&pool, org_id).await;
}

#[tokio::test]
#[ignore] // Requires database
async fn test_storage_idempotency() {
    let pool = create_test_pool().await;
    let org_id = create_test_org(&pool).await;

    let taxdump = create_sample_taxdump();
    let storage =
        NcbiTaxonomyStorage::new(pool.clone(), org_id, "1.0".to_string(), "2026-01-19".to_string());

    // First storage
    let stats1 = storage.store(&taxdump).await.expect("First storage failed");
    assert_eq!(stats1.stored, 3);
    assert_eq!(stats1.updated, 0);

    // Second storage (should update, not create new)
    let stats2 = storage
        .store(&taxdump)
        .await
        .expect("Second storage failed");
    assert_eq!(stats2.stored, 0);
    assert_eq!(stats2.updated, 3);

    // Verify only 3 entries exist (not 6)
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM taxonomy_metadata tm
         JOIN registry_entries re ON tm.data_source_id = re.id
         WHERE re.organization_id = $1",
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("Failed to count entries");

    assert_eq!(count, 3);

    cleanup_test_data(&pool, org_id).await;
}

#[tokio::test]
#[ignore] // Requires database
async fn test_storage_multiple_versions() {
    let pool = create_test_pool().await;
    let org_id = create_test_org(&pool).await;

    let taxdump = create_sample_taxdump();

    // Store version 1.0
    let storage_v1 =
        NcbiTaxonomyStorage::new(pool.clone(), org_id, "1.0".to_string(), "2026-01-19".to_string());
    storage_v1.store(&taxdump).await.expect("V1 storage failed");

    // Store version 1.1 (same data, different version)
    let storage_v2 =
        NcbiTaxonomyStorage::new(pool.clone(), org_id, "1.1".to_string(), "2026-01-20".to_string());
    storage_v2.store(&taxdump).await.expect("V2 storage failed");

    // Verify both versions exist
    let version_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT v.version_string)
         FROM versions v
         JOIN registry_entries re ON v.registry_entry_id = re.id
         WHERE re.organization_id = $1",
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("Failed to count versions");

    assert_eq!(version_count, 2);

    cleanup_test_data(&pool, org_id).await;
}

#[tokio::test]
#[ignore] // Requires database
async fn test_merged_taxa_handling() {
    let pool = create_test_pool().await;
    let org_id = create_test_org(&pool).await;

    // Create data with merged taxon
    let entries = vec![TaxonomyEntry::new(
        12345,
        "Old species name".to_string(),
        None,
        "species".to_string(),
        "Eukaryota;Old lineage".to_string(),
    )];
    let merged = vec![MergedTaxon::new(12345, 9606)];
    let taxdump = TaxdumpData::new(entries, merged, vec![], "2026-01-19".to_string());

    let storage =
        NcbiTaxonomyStorage::new(pool.clone(), org_id, "1.0".to_string(), "2026-01-19".to_string());

    storage.store(&taxdump).await.expect("Storage failed");

    // Verify merged taxon has special lineage note
    let lineage: String =
        sqlx::query_scalar("SELECT lineage FROM taxonomy_metadata WHERE taxonomy_id = $1")
            .bind(12345)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch lineage");

    assert!(lineage.contains("[MERGED INTO"));
    assert!(lineage.contains("9606"));

    cleanup_test_data(&pool, org_id).await;
}

#[tokio::test]
#[ignore] // Requires database
async fn test_deleted_taxa_handling() {
    let pool = create_test_pool().await;
    let org_id = create_test_org(&pool).await;

    // Create data with deleted taxon
    let entries = vec![TaxonomyEntry::new(
        99999,
        "Deleted species".to_string(),
        None,
        "species".to_string(),
        "Eukaryota;Deleted lineage".to_string(),
    )];
    let deleted = vec![DeletedTaxon::new(99999)];
    let taxdump = TaxdumpData::new(entries, vec![], deleted, "2026-01-19".to_string());

    let storage =
        NcbiTaxonomyStorage::new(pool.clone(), org_id, "1.0".to_string(), "2026-01-19".to_string());

    storage.store(&taxdump).await.expect("Storage failed");

    // Verify deleted taxon has special lineage note
    let lineage: String =
        sqlx::query_scalar("SELECT lineage FROM taxonomy_metadata WHERE taxonomy_id = $1")
            .bind(99999)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch lineage");

    assert!(lineage.contains("[DELETED FROM NCBI]"));

    cleanup_test_data(&pool, org_id).await;
}

#[tokio::test]
#[ignore] // Requires database
async fn test_version_files_creation() {
    let pool = create_test_pool().await;
    let org_id = create_test_org(&pool).await;

    let taxdump = create_sample_taxdump();
    let storage =
        NcbiTaxonomyStorage::new(pool.clone(), org_id, "1.0".to_string(), "2026-01-19".to_string());

    storage.store(&taxdump).await.expect("Storage failed");

    // Verify version_files were created for each taxonomy (JSON + TSV)
    let file_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM version_files vf
         JOIN versions v ON vf.version_id = v.id
         JOIN registry_entries re ON v.registry_entry_id = re.id
         WHERE re.organization_id = $1",
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("Failed to count version files");

    // 3 taxonomies * 2 formats (JSON + TSV) = 6 files
    assert_eq!(file_count, 6);

    // Verify both JSON and TSV formats exist
    let json_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM version_files vf
         JOIN versions v ON vf.version_id = v.id
         JOIN registry_entries re ON v.registry_entry_id = re.id
         WHERE re.organization_id = $1 AND vf.format = 'json'",
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("Failed to count JSON files");

    assert_eq!(json_count, 3);

    cleanup_test_data(&pool, org_id).await;
}

#[test]
fn test_storage_stats() {
    let stats = StorageStats {
        total: 100,
        stored: 95,
        updated: 3,
        failed: 2,
    };

    assert_eq!(stats.total, 100);
    assert_eq!(stats.stored, 95);
    assert_eq!(stats.updated, 3);
    assert_eq!(stats.failed, 2);
}
