//! NCBI Taxonomy Full Historical Catchup
//!
//! Ingests all historical versions from NCBI Taxonomy archives.
//! Uses batch operations and parallel processing for optimal performance.
//!
//! Usage:
//!   ORG_ID=<uuid> CONCURRENCY=4 START_DATE=2024-01-01 cargo run --bin ncbi_taxonomy_full_catchup
//!
//! Environment variables:
//!   ORG_ID (required) - Organization UUID
//!   DATABASE_URL (required) - PostgreSQL connection string
//!   CONCURRENCY (optional, default: 4) - Parallel versions to process
//!   START_DATE (optional) - Start date in YYYY-MM-DD format
//!   DB_MAX_CONNECTIONS (optional, default: 30) - Database max connections
//!   SEQUENTIAL (optional, default: false) - Use sequential processing
//!   DRY_RUN (optional, default: false) - List versions without ingesting
//!   VERBOSE (optional, default: false) - Enable verbose logging
//!   AWS_* - AWS credentials for S3 (optional)

use anyhow::{Context, Result};
use bdp_server::ingest::ncbi_taxonomy::{NcbiTaxonomyFtpConfig, NcbiTaxonomyOrchestrator};
use sqlx::postgres::PgPoolOptions;
use std::env;
use tracing::{info, warn, Level};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse environment variables
    let org_id_str = env::var("ORG_ID").context("ORG_ID environment variable must be set")?;
    let org_id = Uuid::parse_str(&org_id_str).context("ORG_ID must be a valid UUID")?;

    let concurrency: usize = env::var("CONCURRENCY")
        .unwrap_or_else(|_| "4".to_string())
        .parse()
        .context("CONCURRENCY must be a number")?;

    let start_date = env::var("START_DATE").ok();

    let db_max_connections: u32 = env::var("DB_MAX_CONNECTIONS")
        .unwrap_or_else(|_| "30".to_string())
        .parse()
        .context("DB_MAX_CONNECTIONS must be a number")?;

    let sequential = env::var("SEQUENTIAL")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);

    let dry_run = env::var("DRY_RUN")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);

    let verbose = env::var("VERBOSE")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);

    // Set up logging
    let log_level = if verbose { Level::TRACE } else { Level::INFO };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    // Validate concurrency
    if concurrency > 10 {
        warn!(
            concurrency = concurrency,
            "High concurrency may cause FTP rate limiting. Recommended: 2-4"
        );
    }

    // Get database URL
    let database_url =
        env::var("DATABASE_URL").context("DATABASE_URL environment variable must be set")?;

    // Connect to database
    info!("Connecting to database...");
    let db = PgPoolOptions::new()
        .max_connections(db_max_connections)
        .connect(&database_url)
        .await
        .context("Failed to connect to database")?;
    info!(max_connections = db_max_connections, "Database connected");

    // Create orchestrator
    let config = NcbiTaxonomyFtpConfig::new();
    let orchestrator = NcbiTaxonomyOrchestrator::new(config, db);

    // Log header
    info!("======================================================================");
    info!("NCBI Taxonomy Full Historical Catchup");
    info!("======================================================================");

    info!(
        org_id = %org_id,
        concurrency = if sequential { 1 } else { concurrency },
        start_date = start_date.as_deref().unwrap_or("Beginning of time (2018-12-01)"),
        db_max_connections = db_max_connections,
        "Configuration"
    );

    // Dry run - just list versions
    if dry_run {
        info!("Dry run mode - listing versions only");

        let versions = orchestrator.list_available_versions().await?;

        let filtered_versions: Vec<_> = if let Some(date) = &start_date {
            versions
                .into_iter()
                .filter(|v| v.as_str() >= date.as_str())
                .collect()
        } else {
            versions
        };

        info!(
            count = filtered_versions.len(),
            oldest = filtered_versions
                .first()
                .map(|s| s.as_str())
                .unwrap_or("N/A"),
            newest = filtered_versions
                .last()
                .map(|s| s.as_str())
                .unwrap_or("N/A"),
            "Found versions to process"
        );

        let avg_minutes = 10.0;
        let sequential_time = filtered_versions.len() as f64 * avg_minutes;
        let parallel_time = sequential_time / concurrency as f64;

        info!(
            sequential_hours = format!("{:.1}", sequential_time / 60.0),
            sequential_minutes = format!("{:.0}", sequential_time),
            "Estimated sequential time"
        );
        if !sequential {
            info!(
                concurrency = concurrency,
                parallel_hours = format!("{:.1}", parallel_time / 60.0),
                parallel_minutes = format!("{:.0}", parallel_time),
                "Estimated parallel time"
            );
        }

        info!("To run actual catchup, set DRY_RUN=false");
        return Ok(());
    }

    // Estimate and confirm
    let versions = orchestrator.list_available_versions().await?;
    let filtered_count = if let Some(date) = &start_date {
        versions
            .iter()
            .filter(|v| v.as_str() >= date.as_str())
            .count()
    } else {
        versions.len()
    };

    let avg_minutes = 10.0;
    let estimated_time = if sequential {
        filtered_count as f64 * avg_minutes
    } else {
        filtered_count as f64 * avg_minutes / concurrency as f64
    };

    info!(
        versions_to_process = filtered_count,
        estimated_hours = format!("{:.1}", estimated_time / 60.0),
        estimated_minutes = format!("{:.0}", estimated_time),
        estimated_download_gb = format!("~{:.1}", filtered_count as f64 * 0.1),
        estimated_db_entries_million = format!("~{:.1}", filtered_count as f64 * 2.5),
        "Estimated workload"
    );

    warn!(
        estimated_hours = format!("{:.1}", estimated_time / 60.0),
        "This will take a while. Press Ctrl+C to cancel within next 5 seconds"
    );

    // Give user time to cancel
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Start catchup
    info!("Starting historical catchup...");
    let start = std::time::Instant::now();

    let results = if sequential {
        info!("Using sequential processing");
        orchestrator
            .catchup_from_date(org_id, start_date.as_deref())
            .await?
    } else {
        info!("Using parallel processing (concurrency: {})", concurrency);
        orchestrator
            .catchup_from_date_parallel(org_id, start_date.as_deref(), concurrency)
            .await?
    };

    let duration = start.elapsed();

    // Log summary
    info!("======================================================================");
    info!("Full catchup completed!");
    info!("======================================================================");

    info!("{}", NcbiTaxonomyOrchestrator::summarize_results(&results));

    info!(
        total_hours = format!("{:.1}", duration.as_secs_f64() / 3600.0),
        total_minutes = format!("{:.0}", duration.as_secs_f64() / 60.0),
        avg_minutes_per_version =
            format!("{:.1}", duration.as_secs_f64() / 60.0 / results.len() as f64),
        "Timing"
    );

    if !sequential {
        let sequential_estimate = duration.as_secs_f64() * concurrency as f64;
        info!(
            estimated_sequential_hours = format!("{:.1}", sequential_estimate / 3600.0),
            speedup = format!("~{:.1}x", concurrency as f64),
            "Parallelism benefits"
        );
    }

    // Performance comparison
    let old_time_hours = results.len() as f64 * 8.0;
    let new_time_hours = duration.as_secs_f64() / 3600.0;
    info!(
        old_pattern_hours = format!("{:.0}", old_time_hours),
        old_pattern_days = format!("{:.1}", old_time_hours / 24.0),
        new_pattern_hours = format!("{:.1}", new_time_hours),
        total_speedup = format!("{:.0}x", old_time_hours / new_time_hours),
        "Performance comparison"
    );

    info!("Success! NCBI Taxonomy historical data is now up to date.");

    Ok(())
}
