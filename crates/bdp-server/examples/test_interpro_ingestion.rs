// Test InterPro Ingestion Pipeline
//
// This example tests the InterPro ingestion pipeline end-to-end
// against a running database.
//
// Usage: cargo run --example test_interpro_ingestion

use bdp_server::db::{create_pool, DbConfig};
use bdp_server::ingest::interpro::{
    config::InterProConfig,
    models::{EntryType, InterProEntry, MemberSignatureData, SignatureDatabase},
    pipeline::InterProPipeline,
    storage::*,
};
use sqlx::PgPool;
use std::env;
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("=== InterPro Ingestion Test ===");

    // Get database URL
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_dev_password@localhost:5432/bdp".to_string());

    info!("Connecting to database: {}", database_url);
    let db_config = DbConfig {
        url: database_url,
        max_connections: 10,
        ..Default::default()
    };
    let pool = create_pool(&db_config).await?;

    info!("✓ Database connected");

    // Test 1: Store a single InterPro entry
    info!("\n=== Test 1: Store Single Entry ===");
    test_store_single_entry(&pool).await?;

    // Test 2: Store multiple entries in batch
    info!("\n=== Test 2: Store Batch Entries ===");
    test_store_batch_entries(&pool).await?;

    // Test 3: Store signatures
    info!("\n=== Test 3: Store Signatures ===");
    test_store_signatures(&pool).await?;

    // Test 4: Run pipeline test
    info!("\n=== Test 4: Pipeline Test ===");
    test_pipeline(&pool).await?;

    // Cleanup
    info!("\n=== Cleanup ===");
    cleanup_test_data(&pool).await?;

    info!("\n✓ All tests passed!");

    Ok(())
}

async fn test_store_single_entry(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let entry = InterProEntry {
        interpro_id: "IPR_TEST_SINGLE".to_string(),
        entry_type: EntryType::Domain,
        name: "Test Single Entry".to_string(),
        short_name: Some("TestSingle".to_string()),
        description: Some("A test entry for validation".to_string()),
    };

    info!("Storing entry: {}", entry.interpro_id);

    let (ds_id, ver_id) = store_interpro_entry(pool, &entry, "96.0").await?;

    info!("✓ Entry stored successfully");
    info!("  - Data Source ID: {}", ds_id);
    info!("  - Version ID: {}", ver_id);

    // Verify it exists in database
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM interpro_entry_metadata WHERE interpro_id = $1",
    )
    .bind(&entry.interpro_id)
    .fetch_one(pool)
    .await?;

    assert_eq!(count, 1, "Entry not found in database");
    info!("✓ Entry verified in database");

    Ok(())
}

async fn test_store_batch_entries(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let entries = vec![
        InterProEntry {
            interpro_id: "IPR_TEST_BATCH_001".to_string(),
            entry_type: EntryType::Family,
            name: "Test Batch 1".to_string(),
            short_name: None,
            description: None,
        },
        InterProEntry {
            interpro_id: "IPR_TEST_BATCH_002".to_string(),
            entry_type: EntryType::Domain,
            name: "Test Batch 2".to_string(),
            short_name: None,
            description: None,
        },
        InterProEntry {
            interpro_id: "IPR_TEST_BATCH_003".to_string(),
            entry_type: EntryType::Repeat,
            name: "Test Batch 3".to_string(),
            short_name: None,
            description: None,
        },
    ];

    info!("Storing {} entries in batch", entries.len());

    let result = store_interpro_entries_batch(pool, &entries, "96.0").await?;

    info!("✓ Batch stored successfully");
    info!("  - Entries stored: {}", result.len());

    assert_eq!(result.len(), 3, "Not all entries were stored");

    // Verify in database
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM interpro_entry_metadata WHERE interpro_id LIKE 'IPR_TEST_BATCH_%'",
    )
    .fetch_one(pool)
    .await?;

    assert_eq!(count, 3, "Not all batch entries found in database");
    info!("✓ All batch entries verified in database");

    Ok(())
}

async fn test_store_signatures(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let signatures = vec![
        MemberSignatureData {
            database: SignatureDatabase::Pfam,
            accession: "PF_TEST_001".to_string(),
            name: Some("Test Pfam Signature".to_string()),
            description: Some("A test Pfam signature".to_string()),
            is_primary: true,
        },
        MemberSignatureData {
            database: SignatureDatabase::Smart,
            accession: "SM_TEST_001".to_string(),
            name: Some("Test SMART Signature".to_string()),
            description: None,
            is_primary: false,
        },
        MemberSignatureData {
            database: SignatureDatabase::Prosite,
            accession: "PS_TEST_001".to_string(),
            name: Some("Test PROSITE Signature".to_string()),
            description: None,
            is_primary: false,
        },
    ];

    info!("Storing {} signatures", signatures.len());

    let result = store_signatures_batch(pool, &signatures).await?;

    info!("✓ Signatures stored successfully");
    info!("  - Signatures stored: {}", result.len());

    assert_eq!(result.len(), 3, "Not all signatures were stored");

    // Verify in database
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM protein_signatures WHERE accession LIKE '%_TEST_%'",
    )
    .fetch_one(pool)
    .await?;

    assert!(count >= 3, "Not all signatures found in database");
    info!("✓ Signatures verified in database");

    Ok(())
}

async fn test_pipeline(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let config = InterProConfig::default();
    let download_dir = PathBuf::from("./test_downloads");

    // Create download directory
    std::fs::create_dir_all(&download_dir)?;

    let pipeline = InterProPipeline::new(pool.clone(), config, download_dir.clone());

    info!("Running pipeline test");

    let stats = pipeline.run_test().await?;

    info!("✓ Pipeline test completed");
    info!("  - Entries stored: {}", stats.entries_stored);

    // Cleanup download directory
    let _ = std::fs::remove_dir_all(&download_dir);

    Ok(())
}

async fn cleanup_test_data(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    info!("Cleaning up test data");

    // Delete test entries
    let deleted_entries = sqlx::query!(
        "DELETE FROM interpro_entry_metadata WHERE interpro_id LIKE 'IPR_TEST%' OR interpro_id = 'IPR000001'"
    )
    .execute(pool)
    .await?;

    info!("  - Deleted {} test entries", deleted_entries.rows_affected());

    // Delete test signatures
    let deleted_sigs = sqlx::query!(
        "DELETE FROM protein_signatures WHERE accession LIKE '%_TEST_%'"
    )
    .execute(pool)
    .await?;

    info!("  - Deleted {} test signatures", deleted_sigs.rows_affected());

    info!("✓ Cleanup complete");

    Ok(())
}
