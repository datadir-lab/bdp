//! NCBI Taxonomy test ingestion binary - Small dataset (1000 entries)
//!
//! This binary tests the NCBI Taxonomy ingestion with a small dataset
//! to verify the implementation works correctly before running full ingestion.
//!
//! Usage:
//!   cargo run --bin ncbi_taxonomy_test_small

use anyhow::Result;
use bdp_server::ingest::ncbi_taxonomy::{
    NcbiTaxonomyFtpConfig, NcbiTaxonomyPipeline,
};
use sqlx::postgres::PgPoolOptions;
use tracing::{info, Level};
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

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    println!("\n🧪 NCBI Taxonomy Small Dataset Test\n");
    println!("This will download and parse 1,000 taxonomy entries");
    println!("to verify the implementation works correctly.\n");

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_dev_password@localhost:5432/bdp".to_string());

    info!("Connecting to database...");
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    info!("✓ Database connected");

    // NCBI organization ID (created earlier)
    let org_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")?;

    // Configure with parse limit for small test
    info!("Configuring pipeline with 1,000 entry limit...");
    let config = NcbiTaxonomyFtpConfig::new()
        .with_parse_limit(1000);

    let pipeline = NcbiTaxonomyPipeline::new(config, db);

    // Run ingestion for the latest version with limited parsing
    println!("\n📥 Starting ingestion...");
    println!("   - Version: Latest (2026-01-01)");
    println!("   - Parse limit: 1,000 entries");
    println!("   - Expected time: ~1-2 minutes\n");

    let start = std::time::Instant::now();

    match pipeline.run_version(org_id, Some("2026-01-01")).await {
        Ok(result) => {
            let duration = start.elapsed();

            println!("\n✅ Ingestion completed successfully!\n");
            println!("Results:");
            println!("  - Version: {}", result.external_version.unwrap_or_default());
            println!("  - Duration: {:.1} seconds", duration.as_secs_f64());

            if let Some(stats) = result.storage_stats {
                println!("  - Taxa stored: {}", stats.stored);
                println!("  - Taxa updated: {}", stats.updated);
                println!("  - Failed: {}", stats.failed);
            }

            if result.skipped {
                println!("\n⚠️  Version was already ingested (skipped)");
            }

            println!("\n🎉 Test completed successfully!");
            println!("   You can now run full ingestion with more data.");
        }
        Err(e) => {
            println!("\n❌ Ingestion failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
