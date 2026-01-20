// GenBank ingestion test binary - Phage division (smallest for testing)
//
// This binary tests the GenBank ingestion pipeline with the phage division,
// which is the smallest division (~20MB compressed, thousands of records).
//
// Usage:
//   cargo run --bin genbank_test_phage
//
// Expected runtime: 2-5 minutes (depending on network and CPU)

use anyhow::{Context, Result};
use bdp_server::ingest::{GenbankFtpConfig, GenbankOrchestrator};
use bdp_server::storage::Storage;
use sqlx::PgPool;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    info!("=== GenBank Phage Division Test ===");

    // Load environment
    dotenvy::dotenv().ok();

    // Connect to database
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;
    let db = PgPool::connect(&database_url)
        .await
        .context("Failed to connect to database")?;

    info!("Connected to database");

    // Initialize S3 storage
    let storage_config = bdp_server::storage::config::StorageConfig::from_env()?;
    let storage = Storage::new(storage_config).await?;

    info!("Initialized S3 storage");

    // Get or create test organization
    let org_id = get_or_create_test_org(&db).await?;
    info!("Using organization: {}", org_id);

    // Configure GenBank ingestion
    let config = GenbankFtpConfig::new()
        .with_genbank()
        .with_parse_limit(1000) // Limit to 1000 records for quick testing
        .with_batch_size(500)
        .with_concurrency(1) // Single division, no need for parallelism
        .with_timeout(600); // 10 minute timeout

    info!("GenBank configuration:");
    info!("  Source: GenBank");
    info!("  Division: Phage (test)");
    info!("  Parse limit: 1000 records");
    info!("  Batch size: 500");

    // Create orchestrator
    let orchestrator = GenbankOrchestrator::new(config, db.clone(), storage);

    // Run test ingestion
    info!("Starting phage division ingestion...");
    let result = orchestrator
        .run_test(org_id)
        .await
        .context("GenBank ingestion failed")?;

    // Print results
    info!("=== GenBank Ingestion Complete ===");
    info!("Release: {}", result.release);
    info!("Division: {}", result.division);
    info!("Records processed: {}", result.records_processed);
    info!("Sequences inserted: {}", result.sequences_inserted);
    info!("Protein mappings: {}", result.mappings_created);
    info!("Bytes uploaded: {} MB", result.bytes_uploaded / 1_000_000);
    info!("Duration: {:.2} seconds", result.duration_seconds);
    info!("Throughput: {:.0} records/second",
        result.records_processed as f64 / result.duration_seconds);

    // Verify data
    verify_data(&db, result.sequences_inserted).await?;

    info!("=== Test Successful ===");
    Ok(())
}

/// Get or create test organization
async fn get_or_create_test_org(db: &PgPool) -> Result<Uuid> {
    // Try to find existing test org
    let existing: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM organizations WHERE slug = 'genbank-test' LIMIT 1"
    )
    .fetch_optional(db)
    .await?;

    if let Some((org_id,)) = existing {
        Ok(org_id)
    } else {
        // Create new test org
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO organizations (id, name, slug, is_public) VALUES ($1, $2, $3, $4)"
        )
        .bind(id)
        .bind("GenBank Test")
        .bind("genbank-test")
        .bind(true)
        .execute(db)
        .await?;

        info!("Created test organization: {}", id);
        Ok(id)
    }
}

/// Verify that data was stored correctly
async fn verify_data(db: &PgPool, expected_count: usize) -> Result<()> {
    info!("Verifying stored data...");

    // Count sequence metadata entries
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sequence_metadata")
        .fetch_one(db)
        .await?;

    let actual_count = count as usize;

    info!("Database verification:");
    info!("  Expected sequences: {}", expected_count);
    info!("  Actual sequences: {}", actual_count);

    if actual_count >= expected_count {
        info!("  ✓ Data verified");
    } else {
        anyhow::bail!("Data verification failed: expected {}, found {}", expected_count, actual_count);
    }

    // Sample some records
    let samples: Vec<(String, String, i32, Option<f64>, Option<String>)> = sqlx::query_as(
        "SELECT accession_version, definition, sequence_length, gc_content, division FROM sequence_metadata LIMIT 5"
    )
    .fetch_all(db)
    .await?;

    info!("Sample records:");
    for (accession, definition, length, gc, division) in samples {
        info!(
            "  {} - {} ({}bp, {:.1}% GC, div: {})",
            accession,
            definition.chars().take(50).collect::<String>(),
            length,
            gc.unwrap_or(0.0),
            division.unwrap_or_else(|| "unknown".to_string())
        );
    }

    Ok(())
}
