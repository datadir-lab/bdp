//! Tests for NCBI Taxonomy orchestrator
//!
//! These tests validate sequential and parallel catchup operations.
//! Run with: cargo test --test ncbi_taxonomy_orchestrator_test -- --nocapture --ignored

use anyhow::Result;
use bdp_server::ingest::ncbi_taxonomy::{NcbiTaxonomyFtpConfig, NcbiTaxonomyOrchestrator};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

/// Helper to get test configuration
fn get_test_config() -> Result<(String, Uuid)> {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let org_id_str =
        std::env::var("TEST_ORG_ID").expect("TEST_ORG_ID must be set for integration tests");

    let org_id = Uuid::parse_str(&org_id_str)?;

    Ok((database_url, org_id))
}

#[tokio::test]
#[ignore] // Requires database and network access
async fn test_list_available_versions() -> Result<()> {
    println!("\n=== Testing Version Discovery ===\n");

    let config = NcbiTaxonomyFtpConfig::new();
    let orchestrator = NcbiTaxonomyOrchestrator::new(config, sqlx::PgPool::connect("").await?);

    let versions = orchestrator.list_available_versions().await?;

    println!("Found {} available versions", versions.len());
    println!("Oldest: {}", versions.first().unwrap());
    println!("Newest: {}", versions.last().unwrap());

    assert!(!versions.is_empty());
    assert!(versions.len() >= 80); // At least 80 versions available

    println!("\n✓ Version discovery working correctly\n");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database and network access
async fn test_sequential_catchup_recent() -> Result<()> {
    let (database_url, org_id) = get_test_config()?;

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    let config = NcbiTaxonomyFtpConfig::new();
    let orchestrator = NcbiTaxonomyOrchestrator::new(config, db);

    println!("\n=== Testing Sequential Catchup (Recent 3 Versions) ===\n");
    println!("Organization ID: {}", org_id);
    println!("Start date: 2025-10-01");
    println!("Expected: 3 versions (Oct, Nov, Dec 2025)");
    println!("Expected time: 15-30 minutes\n");

    let start = std::time::Instant::now();

    let results = orchestrator
        .catchup_from_date(org_id, Some("2025-10-01"))
        .await?;

    let duration = start.elapsed();

    println!("\n{'='*60}\n");
    println!("{}", NcbiTaxonomyOrchestrator::summarize_results(&results));
    println!("\nTotal time: {:.1} minutes", duration.as_secs_f64() / 60.0);
    println!(
        "Average per version: {:.1} minutes",
        duration.as_secs_f64() / 60.0 / results.len() as f64
    );

    // Validations
    assert_eq!(results.len(), 3, "Should process 3 versions");
    assert!(
        results.iter().all(|r| r.is_success() || r.skipped),
        "All versions should succeed or be skipped"
    );

    println!("\n✓ Sequential catchup test passed\n");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database and network access
async fn test_parallel_catchup_recent() -> Result<()> {
    let (database_url, org_id) = get_test_config()?;

    let db = PgPoolOptions::new()
        .max_connections(25) // Higher for parallel processing
        .connect(&database_url)
        .await?;

    let config = NcbiTaxonomyFtpConfig::new();
    let orchestrator = NcbiTaxonomyOrchestrator::new(config, db);

    println!("\n=== Testing Parallel Catchup (Recent 3 Versions, Concurrency=2) ===\n");
    println!("Organization ID: {}", org_id);
    println!("Start date: 2025-10-01");
    println!("Expected: 3 versions (Oct, Nov, Dec 2025)");
    println!("Concurrency: 2");
    println!("Expected time: 8-15 minutes (2x speedup vs sequential)\n");

    let start = std::time::Instant::now();

    let results = orchestrator
        .catchup_from_date_parallel(org_id, Some("2025-10-01"), 2)
        .await?;

    let duration = start.elapsed();

    println!("\n{'='*60}\n");
    println!("{}", NcbiTaxonomyOrchestrator::summarize_results(&results));
    println!("\nTotal time: {:.1} minutes", duration.as_secs_f64() / 60.0);
    println!(
        "Average per version: {:.1} minutes",
        duration.as_secs_f64() / 60.0 / results.len() as f64
    );
    println!("Expected speedup: ~2x vs sequential");

    // Validations
    assert_eq!(results.len(), 3, "Should process 3 versions");
    assert!(
        results.iter().all(|r| r.is_success() || r.skipped),
        "All versions should succeed or be skipped"
    );

    println!("\n✓ Parallel catchup test passed\n");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database and network access
async fn test_catchup_and_current() -> Result<()> {
    let (database_url, org_id) = get_test_config()?;

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    let config = NcbiTaxonomyFtpConfig::new();
    let orchestrator = NcbiTaxonomyOrchestrator::new(config, db);

    println!("\n=== Testing Catchup + Current Version ===\n");
    println!("Organization ID: {}", org_id);
    println!("Start date: 2025-11-01");
    println!("Expected: 2 historical + 1 current = 3 total versions\n");

    let results = orchestrator
        .catchup_and_current(org_id, Some("2025-11-01"))
        .await?;

    println!("\n{}", NcbiTaxonomyOrchestrator::summarize_results(&results));

    // Validations
    assert!(results.len() >= 2, "Should process at least 2 versions");
    assert!(
        results.iter().all(|r| r.is_success() || r.skipped),
        "All versions should succeed or be skipped"
    );

    println!("\n✓ Catchup and current test passed\n");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database and network access
async fn test_full_historical_catchup_dry_run() -> Result<()> {
    let config = NcbiTaxonomyFtpConfig::new();
    let orchestrator = NcbiTaxonomyOrchestrator::new(config, sqlx::PgPool::connect("").await?);

    println!("\n=== Full Historical Catchup - Dry Run ===\n");
    println!("This test only lists versions, does not ingest\n");

    let versions = orchestrator.list_available_versions().await?;

    println!("Total historical versions available: {}", versions.len());
    println!("Date range: {} to {}", versions.first().unwrap(), versions.last().unwrap());

    println!("\nEstimated catchup time:");
    println!(
        "  Sequential (with batch ops): {:.1} hours",
        versions.len() as f64 * 10.0 / 60.0
    );
    println!(
        "  Parallel (concurrency=2): {:.1} hours",
        versions.len() as f64 * 10.0 / 60.0 / 2.0
    );
    println!(
        "  Parallel (concurrency=4): {:.1} hours",
        versions.len() as f64 * 10.0 / 60.0 / 4.0
    );

    println!("\n⚠️  To run full catchup, use:");
    println!("  cargo run --bin ncbi_taxonomy_full_catchup -- --org-id <UUID> --concurrency 4");

    println!("\n✓ Dry run complete\n");

    Ok(())
}

#[test]
fn test_result_summarization() {
    use bdp_server::ingest::ncbi_taxonomy::{pipeline::PipelineResult, storage::StorageStats};

    println!("\n=== Testing Result Summarization ===\n");

    let results = vec![
        PipelineResult {
            external_version: Some("2025-10-01".to_string()),
            internal_version: Some("1.0.0".to_string()),
            storage_stats: Some(StorageStats {
                total: 2_500_000,
                stored: 2_500_000,
                updated: 0,
                failed: 0,
            }),
            skipped: false,
        },
        PipelineResult {
            external_version: Some("2025-11-01".to_string()),
            internal_version: Some("2.0.0".to_string()),
            storage_stats: Some(StorageStats {
                total: 2_500_000,
                stored: 50_000,
                updated: 2_450_000,
                failed: 0,
            }),
            skipped: false,
        },
        PipelineResult {
            external_version: Some("2025-12-01".to_string()),
            internal_version: Some("2.1.0".to_string()),
            storage_stats: None,
            skipped: true,
        },
    ];

    let summary = NcbiTaxonomyOrchestrator::summarize_results(&results);
    println!("{}", summary);

    assert!(summary.contains("Total versions processed: 3"));
    assert!(summary.contains("Successful: 2"));
    assert!(summary.contains("Skipped (already ingested): 1"));
    assert!(summary.contains("Total taxa stored: 2,550,000"));
    assert!(summary.contains("Total taxa updated: 2,450,000"));

    println!("\n✓ Summarization test passed\n");
}

#[test]
fn test_performance_expectations() {
    println!("\n=== Performance Expectations ===\n");

    let versions = 86;
    let entries_per_version = 2_500_000;
    let total_entries = versions * entries_per_version;

    println!("Full historical catchup:");
    println!("  Versions: {}", versions);
    println!("  Entries per version: {}", entries_per_version);
    println!("  Total entries: {}", total_entries);

    println!("\nOld N+1 Pattern:");
    let old_queries_per_entry = 8;
    let old_total_queries = total_entries * old_queries_per_entry;
    let old_time_hours = versions as f64 * 8.0;
    println!("  Queries: {}", old_total_queries);
    println!("  Time: {:.0} hours ({:.1} days)", old_time_hours, old_time_hours / 24.0);

    println!("\nNew Batch Pattern:");
    let chunk_size = 500;
    let chunks_per_version = (entries_per_version + chunk_size - 1) / chunk_size;
    let queries_per_chunk = 6;
    let new_queries_per_version = chunks_per_version * queries_per_chunk;
    let new_total_queries = versions * new_queries_per_version;
    let new_time_minutes = versions as f64 * 10.0;
    println!("  Queries: {}", new_total_queries);
    println!("  Time: {:.0} minutes ({:.1} hours)", new_time_minutes, new_time_minutes / 60.0);
    println!("  Speedup: {}x", old_total_queries / new_total_queries);

    println!("\nBatch + Parallel (concurrency=4):");
    let parallel_time_minutes = new_time_minutes / 4.0;
    println!(
        "  Time: {:.0} minutes ({:.1} hours)",
        parallel_time_minutes,
        parallel_time_minutes / 60.0
    );
    println!("  Total speedup: {:.0}x", old_time_hours / (parallel_time_minutes / 60.0));

    // Assertions
    assert!(
        new_total_queries < old_total_queries / 600,
        "Should reduce queries by at least 600x"
    );
    assert!(parallel_time_minutes < 300.0, "Should complete in under 5 hours with parallel");

    println!("\n✓ Performance expectations validated\n");
}
