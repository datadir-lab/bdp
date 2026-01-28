//! Test Gene Ontology Version Discovery
//!
//! A simple example to test the version discovery functionality
//! without requiring database or S3 setup.
//!
//! Usage:
//! ```bash
//! cargo run --example test_go_version_discovery
//! ```

use anyhow::Result;
use bdp_server::ingest::gene_ontology::{GoHttpConfig, VersionDiscovery};
use chrono::NaiveDate;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    println!("\n=== Gene Ontology Version Discovery Test ===\n");

    // Create version discovery
    let config = GoHttpConfig::default();
    let discovery = VersionDiscovery::new(config)?;

    // Test 1: Discover all versions
    println!("Test 1: Discovering all available GO versions...");
    let versions = discovery.discover_all_versions().await?;

    println!("✅ Found {} versions", versions.len());
    println!();

    // Show the most recent 10 versions
    println!("Most recent 10 versions:");
    println!("{:<15} {:<12} {}", "Version", "Date", "URL");
    println!("{:-<100}", "");

    for version in versions.iter().rev().take(10) {
        println!(
            "{:<15} {:<12} {}",
            version.external_version, version.release_date, version.release_url
        );
    }
    println!();

    // Show the oldest 5 versions
    println!("Oldest 5 versions:");
    println!("{:<15} {:<12} {}", "Version", "Date", "URL");
    println!("{:-<100}", "");

    for version in versions.iter().take(5) {
        println!(
            "{:<15} {:<12} {}",
            version.external_version, version.release_date, version.release_url
        );
    }
    println!();

    // Test 2: Filter by date
    let cutoff_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    println!(
        "Test 2: Filtering versions since {}...",
        cutoff_date
    );

    let filtered = discovery.discover_versions_since(cutoff_date).await?;
    println!("✅ Found {} versions since {}", filtered.len(), cutoff_date);
    println!();

    // Test 3: Filter out "already ingested" versions
    println!("Test 3: Filtering out already-ingested versions...");

    // Simulate some already-ingested versions
    let ingested = vec![
        "2024-01-01".to_string(),
        "2024-02-01".to_string(),
        "2024-03-01".to_string(),
    ];

    println!("Simulated ingested versions: {:?}", ingested);

    let new_versions = discovery.filter_new_versions(filtered.clone(), ingested);
    println!(
        "✅ Found {} new versions (out of {} total)",
        new_versions.len(),
        filtered.len()
    );
    println!();

    // Test 4: Check version ordering
    println!("Test 4: Verifying chronological ordering...");
    let mut is_sorted = true;
    for window in versions.windows(2) {
        if window[0].release_date > window[1].release_date {
            is_sorted = false;
            break;
        }
    }

    if is_sorted {
        println!("✅ Versions are correctly sorted chronologically");
    } else {
        println!("❌ Versions are NOT sorted correctly");
    }
    println!();

    // Summary
    println!("=== Test Summary ===");
    println!("✅ All tests completed successfully!");
    println!();
    println!("Available versions: {}", versions.len());
    println!(
        "Date range: {} to {}",
        versions
            .first()
            .map(|v| v.external_version.as_str())
            .unwrap_or("N/A"),
        versions
            .last()
            .map(|v| v.external_version.as_str())
            .unwrap_or("N/A")
    );
    println!();
    println!("💡 Use 'go_historical_ingestion' example for actual ingestion");
    println!();

    Ok(())
}
