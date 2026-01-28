//! NCBI Taxonomy historical version tests
//!
//! These tests verify FTP archive discovery and historical version downloads.
//! Run with: cargo test --test ncbi_taxonomy_historical_test -- --nocapture --ignored

use bdp_server::ingest::ncbi_taxonomy::{NcbiTaxonomyFtp, NcbiTaxonomyFtpConfig, TaxdumpParser};

#[tokio::test]
#[ignore] // Requires FTP access
async fn test_list_available_versions() {
    println!("\n=== Testing FTP Archive Discovery ===\n");

    let config = NcbiTaxonomyFtpConfig::new();
    let ftp = NcbiTaxonomyFtp::new(config);

    // List all available versions
    let versions = ftp
        .list_available_versions()
        .await
        .expect("Failed to list available versions");

    println!("Found {} archive versions", versions.len());

    assert!(versions.len() > 0, "Should find at least one archive version");

    // Show first and last versions
    if let Some(first) = versions.first() {
        println!("Oldest version: {}", first);
        assert!(first.starts_with("20"), "Version should be a date YYYY-MM-DD");
    }

    if let Some(last) = versions.last() {
        println!("Newest version: {}", last);
    }

    // Show a few random versions
    println!("\nSample versions:");
    for version in versions.iter().take(5) {
        println!("  - {}", version);
    }

    println!("\n✓ Archive discovery successful");
}

#[tokio::test]
#[ignore] // Requires FTP access
async fn test_download_current_version() {
    println!("\n=== Testing Current Version Download ===\n");

    let config = NcbiTaxonomyFtpConfig::new().with_parse_limit(10); // Only parse 10 entries for speed

    let ftp = NcbiTaxonomyFtp::new(config.clone());

    // Download current version (None)
    println!("Downloading current version...");
    let taxdump_files = ftp
        .download_taxdump_version(None)
        .await
        .expect("Failed to download current version");

    println!("Current version: {}", taxdump_files.external_version);
    println!("Downloaded files:");
    println!("  - rankedlineage.dmp: {} bytes", taxdump_files.rankedlineage.len());
    println!("  - merged.dmp: {} bytes", taxdump_files.merged.len());
    println!("  - delnodes.dmp: {} bytes", taxdump_files.delnodes.len());

    assert!(taxdump_files.rankedlineage.len() > 0, "Rankedlineage should not be empty");
    assert!(taxdump_files.external_version.len() > 0, "External version should be set");

    // Parse a few entries
    println!("\nParsing first 10 entries...");
    let parser = TaxdumpParser::with_limit(10);
    let taxdump = parser
        .parse(
            &taxdump_files.rankedlineage,
            &taxdump_files.merged,
            &taxdump_files.delnodes,
            taxdump_files.external_version.clone(),
        )
        .expect("Failed to parse taxdump");

    println!("Parsed:");
    println!("  - {} taxonomy entries", taxdump.entries.len());
    println!("  - {} merged taxa", taxdump.merged.len());
    println!("  - {} deleted taxa", taxdump.deleted.len());

    assert!(taxdump.entries.len() > 0, "Should have parsed some entries");

    // Show first entry
    if let Some(entry) = taxdump.entries.first() {
        println!("\nFirst entry:");
        println!("  - ID: {}", entry.taxonomy_id);
        println!("  - Name: {}", entry.scientific_name);
        println!("  - Rank: {}", entry.rank);
    }

    println!("\n✓ Current version download successful");
}

#[tokio::test]
#[ignore] // Requires FTP access
async fn test_download_historical_version() {
    println!("\n=== Testing Historical Version Download ===\n");

    let config = NcbiTaxonomyFtpConfig::new().with_parse_limit(10); // Only parse 10 entries for speed

    let ftp = NcbiTaxonomyFtp::new(config.clone());

    // First, list available versions to pick one
    let versions = ftp
        .list_available_versions()
        .await
        .expect("Failed to list versions");

    assert!(versions.len() > 0, "Should have at least one version");

    // Pick a recent version (but not the newest, to ensure it's historical)
    // PERFORMANCE: Use reference instead of cloning the entire string
    let test_version = if versions.len() > 3 {
        // Pick 3rd from last (definitely historical)
        &versions[versions.len() - 3]
    } else {
        versions.first().expect("At least one version should exist")
    };

    println!("Testing with historical version: {}", test_version);

    // Download historical version
    println!("Downloading historical version...");
    let taxdump_files = ftp
        .download_taxdump_version(Some(test_version))
        .await
        .expect("Failed to download historical version");

    println!("Downloaded version: {}", taxdump_files.external_version);
    println!("Downloaded files:");
    println!("  - rankedlineage.dmp: {} bytes", taxdump_files.rankedlineage.len());
    println!("  - merged.dmp: {} bytes", taxdump_files.merged.len());
    println!("  - delnodes.dmp: {} bytes", taxdump_files.delnodes.len());

    // Verify external_version matches what we requested
    assert_eq!(
        &taxdump_files.external_version, test_version,
        "External version should match requested version"
    );

    assert!(taxdump_files.rankedlineage.len() > 0, "Rankedlineage should not be empty");

    // Parse a few entries
    println!("\nParsing first 10 entries...");
    let parser = TaxdumpParser::with_limit(10);
    let taxdump = parser
        .parse(
            &taxdump_files.rankedlineage,
            &taxdump_files.merged,
            &taxdump_files.delnodes,
            taxdump_files.external_version.clone(),
        )
        .expect("Failed to parse taxdump");

    println!("Parsed:");
    println!("  - {} taxonomy entries", taxdump.entries.len());
    println!("  - {} merged taxa", taxdump.merged.len());
    println!("  - {} deleted taxa", taxdump.deleted.len());

    assert!(taxdump.entries.len() > 0, "Should have parsed some entries");

    println!("\n✓ Historical version download successful");
}

#[tokio::test]
#[ignore] // Requires FTP access
async fn test_compare_versions() {
    println!("\n=== Testing Version Comparison ===\n");

    let config = NcbiTaxonomyFtpConfig::new().with_parse_limit(5); // Only parse 5 entries for speed

    let ftp = NcbiTaxonomyFtp::new(config.clone());

    // Get available versions
    let versions = ftp
        .list_available_versions()
        .await
        .expect("Failed to list versions");

    if versions.len() < 2 {
        println!("Not enough versions to compare, skipping test");
        return;
    }

    // Download two different versions
    let version1 = &versions[versions.len() - 2]; // Second to last
    let version2 = &versions[versions.len() - 1]; // Last

    println!("Comparing versions:");
    println!("  - Version 1: {}", version1);
    println!("  - Version 2: {}", version2);

    // Download both
    let taxdump1 = ftp
        .download_taxdump_version(Some(version1))
        .await
        .expect("Failed to download version 1");
    let taxdump2 = ftp
        .download_taxdump_version(Some(version2))
        .await
        .expect("Failed to download version 2");

    println!("\nVersion 1 ({}):", taxdump1.external_version);
    println!("  - rankedlineage: {} bytes", taxdump1.rankedlineage.len());
    println!("  - merged: {} bytes", taxdump1.merged.len());
    println!("  - delnodes: {} bytes", taxdump1.delnodes.len());

    println!("\nVersion 2 ({}):", taxdump2.external_version);
    println!("  - rankedlineage: {} bytes", taxdump2.rankedlineage.len());
    println!("  - merged: {} bytes", taxdump2.merged.len());
    println!("  - delnodes: {} bytes", taxdump2.delnodes.len());

    // Verify they're different
    assert_ne!(
        taxdump1.external_version, taxdump2.external_version,
        "External versions should be different"
    );

    println!("\n✓ Version comparison successful");
}

#[test]
fn test_archive_path_construction() {
    println!("\n=== Testing Archive Path Construction ===\n");

    let config = NcbiTaxonomyFtpConfig::new();

    // Test current version path
    let current_path = config.taxdump_path();
    println!("Current path: {}", current_path);
    assert_eq!(current_path, "/pub/taxonomy/new_taxdump/new_taxdump.tar.gz");

    // Test archive path construction
    let archive_path = config.archive_path("2024-01-01");
    println!("Archive path: {}", archive_path);
    assert_eq!(archive_path, "/pub/taxonomy/taxdump_archive/new_taxdump_2024-01-01.zip");

    // Test various dates
    let dates = vec![
        ("2014-01-01", "/pub/taxonomy/taxdump_archive/new_taxdump_2014-01-01.zip"),
        ("2025-12-01", "/pub/taxonomy/taxdump_archive/new_taxdump_2025-12-01.zip"),
        ("2026-01-01", "/pub/taxonomy/taxdump_archive/new_taxdump_2026-01-01.zip"),
    ];

    for (date, expected) in dates {
        let path = config.archive_path(date);
        assert_eq!(path, expected, "Path mismatch for {}", date);
        println!("  ✓ {}: {}", date, path);
    }

    println!("\n✓ Archive path construction successful");
}
