//! NCBI Taxonomy test ingestion binary - Small dataset (1000 entries)
//!
//! This binary tests the NCBI Taxonomy ingestion with a small dataset
//! to verify the implementation works correctly before running full ingestion.
//!
//! Usage:
//!   cargo run --bin ncbi_taxonomy_test_small

use anyhow::Result;
use bdp_server::ingest::ncbi_taxonomy::{NcbiTaxonomyFtpConfig, NcbiTaxonomyPipeline};
use sqlx::postgres::PgPoolOptions;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    info!("NCBI Taxonomy Small Dataset Test");
    info!("This will download and parse 1,000 taxonomy entries to verify the implementation works correctly.");

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_dev_password@localhost:5432/bdp".to_string());

    info!("Connecting to database...");
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    info!("Database connected");

    // NCBI organization ID (created earlier)
    let org_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")?;

    // Configure with parse limit for small test
    info!(parse_limit = 1000, "Configuring pipeline with entry limit");
    let config = NcbiTaxonomyFtpConfig::new().with_parse_limit(1000);

    let pipeline = NcbiTaxonomyPipeline::new(config, db);

    // Run ingestion for the latest version with limited parsing
    info!(
        version = "Latest (2026-01-01)",
        parse_limit = 1000,
        expected_time = "~1-2 minutes",
        "Starting ingestion"
    );

    let start = std::time::Instant::now();

    match pipeline.run_version(org_id, Some("2026-01-01")).await {
        Ok(result) => {
            let duration = start.elapsed();

            info!("Ingestion completed successfully!");
            info!(
                version = result.external_version.as_deref().unwrap_or("unknown"),
                duration_secs = format!("{:.1}", duration.as_secs_f64()),
                "Results"
            );

            if let Some(stats) = result.storage_stats {
                info!(
                    taxa_stored = stats.stored,
                    taxa_updated = stats.updated,
                    failed = stats.failed,
                    "Storage statistics"
                );
            }

            if result.skipped {
                warn!("Version was already ingested (skipped)");
            }

            info!("Test completed successfully! You can now run full ingestion with more data.");
        },
        Err(e) => {
            error!(error = %e, "Ingestion failed");
            return Err(e);
        },
    }

    Ok(())
}
