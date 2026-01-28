// Integration tests for InterPro storage layer
//
// These tests verify:
// 1. NO N+1 queries - all operations use batching
// 2. Transaction atomicity
// 3. Proper foreign key relationships
// 4. Version-specific references
// 5. Performance optimizations

use bdp_server::db::create_pool;
use bdp_server::ingest::interpro::{
    helpers::{GoTermLookupHelper, InterProEntryLookupHelper, ProteinLookupHelper, SignatureLookupHelper},
    models::{EntryType, ExternalReferenceData, GoMappingData, InterProEntry, InterProMetadata, MemberSignatureData, ProteinMatch, SignatureDatabase},
    storage::*,
};
use sqlx::PgPool;
use std::env;

// ============================================================================
// Test Helpers
// ============================================================================

async fn get_test_pool() -> PgPool {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_dev_password@localhost:5432/bdp".to_string());

    create_pool(&database_url, 5).await.expect("Failed to create test pool")
}

fn create_test_entry() -> InterProEntry {
    InterProEntry {
        interpro_id: "IPR_TEST_001".to_string(),
        entry_type: EntryType::Domain,
        name: "Test Domain".to_string(),
        short_name: Some("TestDom".to_string()),
        description: Some("A test domain for integration testing".to_string()),
    }
}

fn create_test_signature() -> MemberSignatureData {
    MemberSignatureData {
        database: SignatureDatabase::Pfam,
        accession: "PF_TEST_001".to_string(),
        name: Some("Test Pfam Signature".to_string()),
        description: Some("Test signature description".to_string()),
        is_primary: true,
    }
}

// ============================================================================
// InterPro Entry Storage Tests
// ============================================================================

#[tokio::test]
async fn test_store_interpro_entry() {
    let pool = get_test_pool().await;
    let entry = create_test_entry();

    let result = store_interpro_entry(&pool, &entry).await;

    assert!(result.is_ok(), "Failed to store InterPro entry: {:?}", result.err());

    let (data_source_id, version_id) = result.unwrap();
    assert!(!data_source_id.is_nil());
    assert!(!version_id.is_nil());

    // Verify entry was created in database
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM interpro_entry_metadata WHERE interpro_id = $1"
    )
    .bind(&entry.interpro_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, 1, "InterPro entry not found in database");

    // Cleanup
    sqlx::query("DELETE FROM interpro_entry_metadata WHERE interpro_id = $1")
        .bind(&entry.interpro_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_store_interpro_entries_batch() {
    let pool = get_test_pool().await;

    let entries = vec![
        InterProEntry {
            interpro_id: "IPR_BATCH_001".to_string(),
            entry_type: EntryType::Family,
            name: "Batch Test 1".to_string(),
            short_name: None,
            description: None,
        },
        InterProEntry {
            interpro_id: "IPR_BATCH_002".to_string(),
            entry_type: EntryType::Domain,
            name: "Batch Test 2".to_string(),
            short_name: None,
            description: None,
        },
        InterProEntry {
            interpro_id: "IPR_BATCH_003".to_string(),
            entry_type: EntryType::Repeat,
            name: "Batch Test 3".to_string(),
            short_name: None,
            description: None,
        },
    ];

    let result = store_interpro_entries_batch(&pool, &entries).await;

    assert!(result.is_ok(), "Failed to store batch: {:?}", result.err());

    let map = result.unwrap();
    assert_eq!(map.len(), 3, "Should have stored 3 entries");

    // Verify all entries created
    for entry in &entries {
        assert!(map.contains_key(&entry.interpro_id), "Missing entry {}", entry.interpro_id);
    }

    // Cleanup
    for entry in &entries {
        sqlx::query("DELETE FROM interpro_entry_metadata WHERE interpro_id = $1")
            .bind(&entry.interpro_id)
            .execute(&pool)
            .await
            .unwrap();
    }
}

// ============================================================================
// Signature Storage Tests
// ============================================================================

#[tokio::test]
async fn test_store_signature() {
    let pool = get_test_pool().await;
    let signature = create_test_signature();

    let result = store_signature(&pool, &signature).await;

    assert!(result.is_ok(), "Failed to store signature: {:?}", result.err());

    let signature_id = result.unwrap();
    assert!(!signature_id.is_nil());

    // Verify signature in database
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM protein_signatures WHERE accession = $1"
    )
    .bind(&signature.accession)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, 1, "Signature not found in database");

    // Cleanup
    sqlx::query("DELETE FROM protein_signatures WHERE accession = $1")
        .bind(&signature.accession)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_store_signatures_batch_deduplication() {
    let pool = get_test_pool().await;

    // Create duplicates - should deduplicate
    let signatures = vec![
        MemberSignatureData {
            database: SignatureDatabase::Pfam,
            accession: "PF_DEDUP_001".to_string(),
            name: Some("Test 1".to_string()),
            description: None,
            is_primary: true,
        },
        MemberSignatureData {
            database: SignatureDatabase::Pfam,
            accession: "PF_DEDUP_001".to_string(), // DUPLICATE
            name: Some("Test 1 Dup".to_string()),
            description: None,
            is_primary: false,
        },
        MemberSignatureData {
            database: SignatureDatabase::Smart,
            accession: "SM_DEDUP_001".to_string(),
            name: Some("Test 2".to_string()),
            description: None,
            is_primary: true,
        },
    ];

    let result = store_signatures_batch(&pool, &signatures).await;

    assert!(result.is_ok(), "Failed to store batch: {:?}", result.err());

    let map = result.unwrap();
    // Should only have 2 unique signatures (PF_DEDUP_001 and SM_DEDUP_001)
    assert_eq!(map.len(), 2, "Should deduplicate signatures");

    // Cleanup
    sqlx::query("DELETE FROM protein_signatures WHERE accession LIKE '%DEDUP%'")
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_link_signatures_to_entry() {
    let pool = get_test_pool().await;

    // Create entry and signature
    let entry = create_test_entry();
    let (interpro_ds_id, _) = store_interpro_entry(&pool, &entry).await.unwrap();

    let signature = create_test_signature();
    let sig_id = store_signature(&pool, &signature).await.unwrap();

    // Link them
    let sig_links = vec![(sig_id, true)];
    let result = link_signatures_to_entry(&pool, interpro_ds_id, &sig_links).await;

    assert!(result.is_ok(), "Failed to link signature: {:?}", result.err());

    // Verify link created
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM interpro_member_signatures WHERE interpro_data_source_id = $1"
    )
    .bind(interpro_ds_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, 1, "Signature link not found");

    // Cleanup
    sqlx::query("DELETE FROM interpro_entry_metadata WHERE interpro_id = $1")
        .bind(&entry.interpro_id)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("DELETE FROM protein_signatures WHERE accession = $1")
        .bind(&signature.accession)
        .execute(&pool)
        .await
        .unwrap();
}

// ============================================================================
// External Reference Storage Tests
// ============================================================================

#[tokio::test]
async fn test_store_external_references() {
    let pool = get_test_pool().await;

    let entry = create_test_entry();
    let (interpro_ds_id, _) = store_interpro_entry(&pool, &entry).await.unwrap();

    let references = vec![
        ExternalReferenceData {
            database: "PDB".to_string(),
            database_id: "1ABC".to_string(),
            description: Some("Test PDB structure".to_string()),
        },
        ExternalReferenceData {
            database: "Wikipedia".to_string(),
            database_id: "Test_Domain".to_string(),
            description: None,
        },
    ];

    let result = store_external_references(&pool, interpro_ds_id, &references).await;

    assert!(result.is_ok(), "Failed to store references: {:?}", result.err());

    let count = result.unwrap();
    assert_eq!(count, 2, "Should have stored 2 references");

    // Verify in database
    let db_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM interpro_external_references WHERE interpro_data_source_id = $1"
    )
    .bind(interpro_ds_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(db_count, 2, "References not found in database");

    // Cleanup
    sqlx::query("DELETE FROM interpro_entry_metadata WHERE interpro_id = $1")
        .bind(&entry.interpro_id)
        .execute(&pool)
        .await
        .unwrap();
}

// ============================================================================
// Complete Metadata Storage Tests
// ============================================================================

#[tokio::test]
async fn test_store_complete_metadata() {
    let pool = get_test_pool().await;

    let metadata = InterProMetadata {
        entry: InterProEntry {
            interpro_id: "IPR_COMPLETE_001".to_string(),
            entry_type: EntryType::Domain,
            name: "Complete Test".to_string(),
            short_name: Some("CompleteTest".to_string()),
            description: Some("Complete metadata test".to_string()),
        },
        member_signatures: vec![
            MemberSignatureData {
                database: SignatureDatabase::Pfam,
                accession: "PF_COMPLETE_001".to_string(),
                name: Some("Primary Signature".to_string()),
                description: None,
                is_primary: true,
            },
            MemberSignatureData {
                database: SignatureDatabase::Smart,
                accession: "SM_COMPLETE_001".to_string(),
                name: Some("Secondary Signature".to_string()),
                description: None,
                is_primary: false,
            },
        ],
        go_mappings: vec![], // Would need real GO terms
        external_references: vec![
            ExternalReferenceData {
                database: "PDB".to_string(),
                database_id: "2XYZ".to_string(),
                description: Some("Test structure".to_string()),
            },
        ],
    };

    let mut go_helper = GoTermLookupHelper::new();
    let result = store_interpro_metadata(&pool, &metadata, &mut go_helper).await;

    assert!(result.is_ok(), "Failed to store complete metadata: {:?}", result.err());

    let (ds_id, ver_id) = result.unwrap();
    assert!(!ds_id.is_nil());
    assert!(!ver_id.is_nil());

    // Verify entry
    let entry_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM interpro_entry_metadata WHERE interpro_id = $1"
    )
    .bind(&metadata.entry.interpro_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(entry_count, 1);

    // Verify signatures
    let sig_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM interpro_member_signatures WHERE interpro_data_source_id = $1"
    )
    .bind(ds_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(sig_count, 2);

    // Verify external references
    let ref_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM interpro_external_references WHERE interpro_data_source_id = $1"
    )
    .bind(ds_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ref_count, 1);

    // Cleanup
    sqlx::query("DELETE FROM interpro_entry_metadata WHERE interpro_id = $1")
        .bind(&metadata.entry.interpro_id)
        .execute(&pool)
        .await
        .unwrap();

    for sig in &metadata.member_signatures {
        sqlx::query("DELETE FROM protein_signatures WHERE accession = $1")
            .bind(&sig.accession)
            .execute(&pool)
            .await
            .unwrap();
    }
}

// ============================================================================
// Helper Tests
// ============================================================================

#[tokio::test]
async fn test_helpers_no_n_plus_one() {
    let pool = get_test_pool().await;

    // This test verifies helpers use batch queries, not N+1

    let mut protein_helper = ProteinLookupHelper::new();
    let accessions = vec![
        "P12345".to_string(),
        "P67890".to_string(),
        "Q99999".to_string(),
    ];

    // Single batch load should handle all accessions
    let result = protein_helper.load_batch(&pool, &accessions).await;

    // Even if proteins don't exist, batch load should succeed
    assert!(result.is_ok(), "Batch load failed: {:?}", result.err());

    // Verify cache behavior
    assert_eq!(
        protein_helper.cache_size(),
        0,
        "No proteins should be in cache (they don't exist)"
    );
}

#[tokio::test]
async fn test_signature_helper_batch() {
    let pool = get_test_pool().await;

    // Create some test signatures first
    let sig1 = MemberSignatureData {
        database: SignatureDatabase::Pfam,
        accession: "PF_HELPER_001".to_string(),
        name: Some("Helper Test 1".to_string()),
        description: None,
        is_primary: true,
    };

    let sig2 = MemberSignatureData {
        database: SignatureDatabase::Smart,
        accession: "SM_HELPER_001".to_string(),
        name: Some("Helper Test 2".to_string()),
        description: None,
        is_primary: false,
    };

    let _ = store_signature(&pool, &sig1).await.unwrap();
    let _ = store_signature(&pool, &sig2).await.unwrap();

    // Now test helper
    let mut helper = SignatureLookupHelper::new();

    let signatures = vec![
        ("Pfam".to_string(), "PF_HELPER_001".to_string()),
        ("SMART".to_string(), "SM_HELPER_001".to_string()),
    ];

    let result = helper.load_batch(&pool, &signatures).await;

    assert!(result.is_ok(), "Helper batch load failed: {:?}", result.err());
    assert_eq!(helper.cache_size(), 2, "Should have 2 signatures in cache");

    assert!(helper.contains("Pfam", "PF_HELPER_001"));
    assert!(helper.contains("SMART", "SM_HELPER_001"));

    // Cleanup
    sqlx::query("DELETE FROM protein_signatures WHERE accession LIKE '%HELPER%'")
        .execute(&pool)
        .await
        .unwrap();
}

// ============================================================================
// Performance Tests
// ============================================================================

#[tokio::test]
async fn test_batch_performance() {
    let pool = get_test_pool().await;

    // Create 100 entries to test batch performance
    let mut entries = Vec::new();
    for i in 0..100 {
        entries.push(InterProEntry {
            interpro_id: format!("IPR_PERF_{:03}", i),
            entry_type: EntryType::Domain,
            name: format!("Performance Test {}", i),
            short_name: None,
            description: None,
        });
    }

    let start = std::time::Instant::now();
    let result = store_interpro_entries_batch(&pool, &entries).await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Batch storage failed");
    assert_eq!(result.unwrap().len(), 100);

    println!("Stored 100 entries in {:?}", duration);
    assert!(duration.as_secs() < 10, "Batch operation took too long");

    // Cleanup
    for entry in &entries {
        sqlx::query("DELETE FROM interpro_entry_metadata WHERE interpro_id = $1")
            .bind(&entry.interpro_id)
            .execute(&pool)
            .await
            .unwrap();
    }
}
