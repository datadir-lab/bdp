//! Integration tests for UniProt FTP downloads
//!
//! These tests connect to the real UniProt FTP server and are marked with #[ignore]
//! to avoid running in CI. Run them explicitly with:
//!
//! ```bash
//! cargo test --test ftp_integration_tests -- --ignored --nocapture
//! ```

use anyhow::Result;
use bdp_server::ingest::uniprot::{UniProtFtp, UniProtFtpConfig, DatParser};
use tracing::info;

/// Initialize tracing for tests
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let _ = fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,bdp_server=debug")),
        )
        .with_test_writer()
        .try_init();
}

/// Test: Connect to UniProt FTP and download release notes
#[tokio::test]
#[ignore] // Only run when explicitly requested: cargo test -- --ignored
async fn test_download_release_notes_real() -> Result<()> {
    use bdp_server::ingest::uniprot::ReleaseType;

    init_tracing();
    info!("🧪 Testing real FTP download of release notes");

    let config = UniProtFtpConfig::default()
        .with_release_type(ReleaseType::Previous);
    let ftp = UniProtFtp::new(config);

    // Try to download release notes for version 2024_01
    info!("Downloading release notes for 2024_01...");
    let notes = ftp.download_release_notes(Some("2024_01")).await?;

    info!("✅ Downloaded release notes ({} bytes)", notes.len());
    assert!(!notes.is_empty(), "Release notes should not be empty");
    assert!(notes.contains("Swiss-Prot Release"), "Should contain release info");

    // Parse the release notes
    let release_info = ftp.parse_release_notes(&notes)?;
    info!("📋 Parsed release info:");
    info!("  Version: {}", release_info.external_version);
    info!("  Date: {}", release_info.release_date);
    info!("  Count: {} proteins", release_info.swissprot_count);

    assert_eq!(release_info.external_version, "2024_01");
    assert!(release_info.swissprot_count > 500_000, "Should have >500k proteins");

    info!("✅ Release notes test passed!");
    Ok(())
}

/// Test: Download a small portion of DAT file and verify it parses
#[tokio::test]
#[ignore] // Only run when explicitly requested
async fn test_download_dat_file_sample() -> Result<()> {
    use bdp_server::ingest::uniprot::ReleaseType;

    init_tracing();
    info!("🧪 Testing real FTP download of DAT file (limited parse)");

    let config = UniProtFtpConfig::default()
        .with_parse_limit(10)
        .with_release_type(ReleaseType::Previous);
    let ftp = UniProtFtp::new(config.clone());

    // Download DAT file (this will be large, ~30MB compressed)
    info!("Downloading DAT file for 2024_01 (this may take a minute)...");
    let dat_data = ftp.download_dat_file(Some("2024_01"), None).await?;

    info!("✅ Downloaded DAT file ({} bytes decompressed)", dat_data.len());
    assert!(!dat_data.is_empty(), "DAT file should not be empty");

    // Parse first 10 entries
    info!("Parsing first 10 entries...");
    let parser = DatParser::new().with_limit(config.parse_limit);
    let entries = parser.parse_bytes(&dat_data)?;

    info!("✅ Parsed {} entries", entries.len());
    assert!(entries.len() > 0, "Should parse at least 1 entry");
    assert!(entries.len() <= 10, "Should respect parse limit");

    // Verify first entry has required fields
    let first = &entries[0];
    info!("📋 First entry:");
    info!("  Accession: {}", first.accession);
    info!("  Entry Name: {}", first.entry_name);
    info!("  Protein Name: {}", first.protein_name);
    info!("  Organism: {}", first.organism_name);
    info!("  Sequence Length: {}", first.sequence_length);

    assert!(!first.accession.is_empty(), "Accession should not be empty");
    assert!(!first.sequence.is_empty(), "Sequence should not be empty");
    assert!(first.sequence_length > 0, "Sequence length should be positive");

    info!("✅ DAT file download and parse test passed!");
    Ok(())
}

/// Test: Check if a version exists on FTP server
#[tokio::test]
#[ignore]
async fn test_check_version_exists() -> Result<()> {
    use bdp_server::ingest::uniprot::ReleaseType;

    init_tracing();
    info!("🧪 Testing version existence check");

    let config = UniProtFtpConfig::default()
        .with_release_type(ReleaseType::Previous);
    let ftp = UniProtFtp::new(config);

    // Check if 2024_01 exists (should exist)
    info!("Checking if version 2024_01 exists...");
    let exists = ftp.check_version_exists(Some("2024_01")).await?;
    assert!(exists, "Version 2024_01 should exist");
    info!("✅ Version 2024_01 exists");

    // Check if a fake version exists (should not exist)
    info!("Checking if version 9999_99 exists...");
    let exists = ftp.check_version_exists(Some("9999_99")).await?;
    assert!(!exists, "Version 9999_99 should not exist");
    info!("✅ Version 9999_99 does not exist (as expected)");

    info!("✅ Version existence check test passed!");
    Ok(())
}

/// Test: Full pipeline from FTP download to database storage
#[tokio::test]
#[ignore]
async fn test_full_ftp_to_database_pipeline() -> Result<()> {
    use bdp_server::ingest::uniprot::UniProtStorage;
    use serial_test::serial;
    use sqlx::postgres::PgPoolOptions;
    use testcontainers::{runners::AsyncRunner, ImageExt};
    use testcontainers_modules::postgres::Postgres;

    init_tracing();
    info!("🧪 Testing full pipeline: FTP → Parse → Database");

    // Setup test database
    let postgres_container = Postgres::default()
        .with_tag("16-alpine")
        .start()
        .await?;

    let host = postgres_container.get_host().await?;
    let port = postgres_container.get_host_port_ipv4(5432).await?;
    let conn_string = format!("postgresql://postgres:postgres@{}:{}/postgres", host, port);

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&conn_string)
        .await?;

    // Run migrations
    info!("Running migrations...");
    sqlx::migrate!("../../migrations").run(&db_pool).await?;

    // Create test organization
    let org_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, 'uniprot', 'UniProt', true)")
        .bind(org_id)
        .execute(&db_pool)
        .await?;

    // Download from FTP (limit to 5 entries for speed)
    use bdp_server::ingest::uniprot::ReleaseType;

    let config = UniProtFtpConfig::default()
        .with_parse_limit(5)
        .with_release_type(ReleaseType::Previous);
    let ftp = UniProtFtp::new(config.clone());

    info!("Downloading DAT file from FTP...");
    let dat_data = ftp.download_dat_file(Some("2024_01"), None).await?;
    info!("✅ Downloaded {} bytes", dat_data.len());

    // Parse entries
    info!("Parsing entries...");
    let parser = DatParser::new().with_limit(config.parse_limit);
    let entries = parser.parse_bytes(&dat_data)?;
    info!("✅ Parsed {} entries", entries.len());

    // Store in database
    info!("Storing in database...");
    let storage = UniProtStorage::new(
        db_pool.clone(),
        org_id,
        "1.0".to_string(),
        "2024_01".to_string(),
    );
    let stored_count = storage.store_entries(&entries).await?;
    info!("✅ Stored {} proteins", stored_count);

    assert_eq!(stored_count, entries.len(), "All entries should be stored");

    // Verify in database
    let protein_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM protein_metadata"
    )
    .fetch_one(&db_pool)
    .await?;

    info!("✅ Verified {} proteins in database", protein_count);
    assert_eq!(protein_count, stored_count as i64);

    info!("✅ Full pipeline test passed!");
    Ok(())
}

/// Test: Download from current release (no version specified)
#[tokio::test]
#[ignore]
async fn test_download_current_release() -> Result<()> {
    use bdp_server::ingest::uniprot::ReleaseType;

    init_tracing();
    info!("🧪 Testing download from current release");

    let config = UniProtFtpConfig::default()
        .with_release_type(ReleaseType::Current);
    let ftp = UniProtFtp::new(config);

    // Download current release notes (no version needed)
    info!("Downloading current release notes...");
    let notes = ftp.download_release_notes(None).await?;

    info!("✅ Downloaded current release notes ({} bytes)", notes.len());
    assert!(!notes.is_empty(), "Release notes should not be empty");
    assert!(notes.contains("Swiss-Prot Release"), "Should contain release info");

    // Parse to see what version we got
    let release_info = ftp.parse_release_notes(&notes)?;
    info!("📋 Current release info:");
    info!("  Version: {}", release_info.external_version);
    info!("  Date: {}", release_info.release_date);
    info!("  Count: {} proteins", release_info.swissprot_count);

    assert!(release_info.swissprot_count > 500_000, "Should have >500k proteins");

    info!("✅ Current release test passed!");
    Ok(())
}

/// Test: Download TrEMBL dataset (unreviewed proteins)
#[tokio::test]
#[ignore]
async fn test_download_trembl_dataset() -> Result<()> {
    use bdp_server::ingest::uniprot::ReleaseType;

    init_tracing();
    info!("🧪 Testing TrEMBL dataset download");

    let config = UniProtFtpConfig::default()
        .with_parse_limit(5)
        .with_release_type(ReleaseType::Previous);
    let ftp = UniProtFtp::new(config.clone());

    // Download TrEMBL DAT file (much larger than Swiss-Prot)
    info!("Downloading TrEMBL DAT file for 2024_01 (limited to 5 entries)...");
    let dat_data = ftp.download_dat_file(Some("2024_01"), Some("trembl")).await?;

    info!("✅ Downloaded TrEMBL DAT file ({} bytes decompressed)", dat_data.len());
    assert!(!dat_data.is_empty(), "DAT file should not be empty");

    // Parse to verify it's valid
    info!("Parsing TrEMBL entries...");
    let parser = DatParser::new().with_limit(config.parse_limit);
    let entries = parser.parse_bytes(&dat_data)?;

    info!("✅ Parsed {} TrEMBL entries", entries.len());
    assert!(entries.len() > 0, "Should parse at least 1 entry");

    info!("✅ TrEMBL dataset test passed!");
    Ok(())
}

#[ctor::ctor]
fn init() {
    init_tracing();
}
