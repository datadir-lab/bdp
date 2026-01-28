//! E2E tests for UniProt DAT parser and database storage
//!
//! These tests validate the parser and database integration without
//! requiring the full job queue infrastructure.

use anyhow::Result;
use bdp_server::ingest::uniprot::{parser::DatParser, storage::UniProtStorage};
use serial_test::serial;
use sqlx::postgres::PgPoolOptions;
use std::path::Path;
use testcontainers::{runners::AsyncRunner, ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tracing::info;

/// Test environment for parser E2E tests
struct ParserTestEnv {
    _postgres_container: ContainerAsync<Postgres>,
    db_pool: sqlx::PgPool,
}

impl ParserTestEnv {
    async fn new() -> Result<Self> {
        // Start PostgreSQL
        let postgres_container = Postgres::default().with_tag("16-alpine").start().await?;

        let host = postgres_container.get_host().await?;
        let port = postgres_container.get_host_port_ipv4(5432).await?;

        let conn_string = format!("postgresql://postgres:postgres@{}:{}/postgres", host, port);
        info!("PostgreSQL connection: {}", conn_string);

        // Create pool
        let db_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&conn_string)
            .await?;

        // Run migrations
        info!("Running migrations");
        sqlx::migrate!("../../migrations").run(&db_pool).await?;

        Ok(Self {
            _postgres_container: postgres_container,
            db_pool,
        })
    }

    fn pool(&self) -> &sqlx::PgPool {
        &self.db_pool
    }
}

/// Test: Parse CI sample DAT file
#[tokio::test]
#[serial]
async fn test_parse_ci_sample() -> Result<()> {
    init_tracing();
    info!("🧪 Testing DAT parser with CI sample");

    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/uniprot_ci_sample.dat");

    info!("Reading DAT file: {}", fixture_path.display());

    let parser = DatParser::new();
    let entries = parser.parse_file(&fixture_path)?;

    info!("Parsed {} entries", entries.len());
    assert_eq!(entries.len(), 3, "CI sample should have 3 entries");

    // Verify first entry
    let first = &entries[0];
    info!("First entry: {}", first.accession);
    assert!(!first.accession.is_empty());
    assert!(!first.entry_name.is_empty());

    info!("✅ DAT parser test passed!");
    Ok(())
}

/// Test: Parse and store to database
#[tokio::test]
#[serial]
async fn test_parse_and_store() -> Result<()> {
    init_tracing();
    info!("🧪 Testing DAT parser with database storage");

    let env = ParserTestEnv::new().await?;

    // Create test organization
    let org_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, 'uniprot', 'UniProt', false)"
    )
    .bind(org_id)
    .execute(env.pool())
    .await?;

    info!("Created organization: {}", org_id);

    // Parse DAT file
    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/uniprot_ci_sample.dat");

    let parser = DatParser::new();
    let entries = parser.parse_file(&fixture_path)?;

    info!("Parsed {} entries", entries.len());
    assert_eq!(entries.len(), 3);

    // Store parsed entries
    let storage =
        UniProtStorage::new(env.pool().clone(), org_id, "1.0".to_string(), "2024_01".to_string());
    let stored_count = storage.store_entries(&entries).await?;

    info!("Stored {} entries", stored_count);
    assert_eq!(stored_count, 3, "All entries should be stored");

    // Verify data was stored correctly
    let protein_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM protein_metadata pm
         JOIN data_sources ds ON ds.id = pm.data_source_id
         JOIN registry_entries re ON re.id = ds.id
         WHERE re.organization_id = $1",
    )
    .bind(org_id)
    .fetch_one(env.pool())
    .await?;

    assert_eq!(protein_count, 3, "Should have 3 proteins in database");

    // Verify specific protein exists
    let protein = sqlx::query_as::<_, (String, String, Option<i32>)>(
        "SELECT pm.accession, pm.entry_name, pm.sequence_length
         FROM protein_metadata pm
         WHERE pm.accession = 'Q6GZX4'",
    )
    .fetch_one(env.pool())
    .await?;

    info!("Found protein: {} ({})", protein.0, protein.1);
    assert_eq!(protein.0, "Q6GZX4");
    assert!(protein.2.is_some());

    // Verify version_files for multiple formats
    let file_formats = sqlx::query_as::<_, (String,)>(
        "SELECT vf.format FROM version_files vf
         JOIN versions v ON v.id = vf.version_id
         JOIN registry_entries re ON re.id = v.entry_id
         WHERE re.organization_id = $1
         ORDER BY vf.format",
    )
    .bind(org_id)
    .fetch_all(env.pool())
    .await?;

    let formats: Vec<String> = file_formats.into_iter().map(|f| f.0).collect();
    info!("Version file formats found: {:?}", formats);

    // Should have dat, fasta, json for each protein (3 proteins * 3 formats = 9 files)
    assert_eq!(formats.len(), 9, "Should have 9 version files (3 proteins × 3 formats)");
    assert_eq!(formats.iter().filter(|f| f == &"dat").count(), 3);
    assert_eq!(formats.iter().filter(|f| f == &"fasta").count(), 3);
    assert_eq!(formats.iter().filter(|f| f == &"json").count(), 3);

    // Create aggregate source
    info!("Creating aggregate source...");
    let aggregate_id = storage.create_aggregate_source(stored_count).await?;
    info!("Created aggregate source: {}", aggregate_id);

    // Verify aggregate exists
    let aggregate = sqlx::query_as::<_, (String, i32)>(
        "SELECT re.slug, v.dependency_count
         FROM registry_entries re
         JOIN versions v ON v.entry_id = re.id
         WHERE re.id = $1",
    )
    .bind(aggregate_id)
    .fetch_one(env.pool())
    .await?;

    assert_eq!(aggregate.0, "uniprot-all");
    assert_eq!(aggregate.1, 3, "Aggregate should have 3 dependencies");

    // Verify dependencies exist
    let dep_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dependencies WHERE version_id = (
            SELECT id FROM versions WHERE entry_id = $1
        )",
    )
    .bind(aggregate_id)
    .fetch_one(env.pool())
    .await?;

    assert_eq!(dep_count, 3, "Should have 3 dependency records");

    info!("✅ Parse and store test completed!");
    Ok(())
}

/// Test: Verify parser handles malformed data
#[tokio::test]
#[serial]
async fn test_parse_invalid_data() -> Result<()> {
    init_tracing();
    info!("🧪 Testing DAT parser error handling");

    let parser = DatParser::new();

    // Create temp file with invalid data
    let temp_dir = tempfile::tempdir()?;
    let invalid_path = temp_dir.path().join("invalid.dat");
    std::fs::write(&invalid_path, b"This is not valid DAT format\nJust random text\n")?;

    // Parser should handle gracefully
    let result = parser.parse_file(&invalid_path);

    match result {
        Ok(entries) => {
            info!("Parser returned {} entries from invalid data", entries.len());
            // Should return empty or minimal results, not crash
            assert!(entries.is_empty() || entries.len() < 3);
        },
        Err(e) => {
            info!("Parser returned error (expected): {}", e);
            // Error is also acceptable
        },
    }

    info!("✅ Error handling test passed!");
    Ok(())
}

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

#[ctor::ctor]
fn init() {
    init_tracing();
}
