//! InterPro Historical Ingestion Example
//!
//! Demonstrates ingesting multiple InterPro versions starting from a specific version
//!
//! Usage:
//!   # Ingest from version 96.0 onwards (skip already-ingested)
//!   cargo run --example interpro_historical_ingestion -- 96.0
//!
//!   # Ingest specific version only
//!   cargo run --example interpro_historical_ingestion -- 98.0 --single

use anyhow::{Context, Result};
use bdp_server::{
    config::DatabaseConfig,
    ingest::interpro::{config::InterProConfig, pipeline::InterProPipeline},
};
use sqlx::PgPool;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Parse arguments
    let args: Vec<String> = env::args().collect();
    let start_version = args.get(1).map(|s| s.as_str()).unwrap_or("96.0");
    let single_version = args.contains(&"--single".to_string());

    tracing::info!("InterPro Historical Ingestion");
    tracing::info!("==============================");
    tracing::info!("Start version: {}", start_version);
    tracing::info!(
        "Mode: {}",
        if single_version {
            "single"
        } else {
            "historical"
        }
    );
    tracing::info!("");

    // Load configuration
    let db_config = DatabaseConfig::from_env();
    let interpro_config = InterProConfig::from_env();

    // Connect to database
    tracing::info!("Connecting to database...");
    let pool = PgPool::connect(&db_config.url)
        .await
        .context("Failed to connect to database")?;

    tracing::info!("Database connection established");

    // Create download directory
    let download_dir = std::path::PathBuf::from("./data/interpro_downloads");
    std::fs::create_dir_all(&download_dir).context("Failed to create download directory")?;

    // Create pipeline
    let pipeline = InterProPipeline::new(pool.clone(), interpro_config, download_dir);

    if single_version {
        // Ingest single version only
        tracing::info!("Ingesting single version: {}", start_version);

        match pipeline.run(start_version).await {
            Ok(stats) => {
                tracing::info!("Ingestion complete!");
                tracing::info!("Statistics:");
                tracing::info!("  Files downloaded: {}", stats.files_downloaded);
                tracing::info!("  Entries parsed: {}", stats.entries_parsed);
                tracing::info!("  Entries stored: {}", stats.entries_stored);
                tracing::info!("  Signatures stored: {}", stats.signatures_stored);
                tracing::info!("  Matches parsed: {}", stats.matches_parsed);
                tracing::info!("  Matches stored: {}", stats.matches_stored);
            },
            Err(e) => {
                tracing::error!("Ingestion failed: {}", e);
                return Err(e.into());
            },
        }
    } else {
        // Ingest all versions from start_version onwards
        tracing::info!("Starting historical ingestion from version {}", start_version);

        match pipeline.ingest_from_version(start_version, true).await {
            Ok(results) => {
                tracing::info!("");
                tracing::info!("Historical ingestion complete!");
                tracing::info!("Successfully ingested {} versions", results.len());
                tracing::info!("");
                tracing::info!("Summary by version:");

                for (version, stats) in results {
                    tracing::info!("");
                    tracing::info!("Version {}:", version);
                    tracing::info!("  Entries: {}", stats.entries_stored);
                    tracing::info!("  Signatures: {}", stats.signatures_stored);
                    tracing::info!("  Protein matches: {}", stats.matches_stored);
                }
            },
            Err(e) => {
                tracing::error!("Historical ingestion failed: {}", e);
                return Err(e.into());
            },
        }
    }

    tracing::info!("");
    tracing::info!("Done!");

    Ok(())
}
