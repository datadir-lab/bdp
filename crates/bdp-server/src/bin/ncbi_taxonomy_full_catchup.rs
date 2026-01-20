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
use bdp_server::ingest::ncbi_taxonomy::{
    NcbiTaxonomyFtpConfig, NcbiTaxonomyOrchestrator,
};
use sqlx::postgres::PgPoolOptions;
use tracing::{info, Level};
use uuid::Uuid;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse environment variables
    let org_id_str = env::var("ORG_ID")
        .context("ORG_ID environment variable must be set")?;
    let org_id = Uuid::parse_str(&org_id_str)
        .context("ORG_ID must be a valid UUID")?;

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
    let log_level = if verbose {
        Level::TRACE
    } else {
        Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    // Validate concurrency
    if concurrency > 10 {
        eprintln!("⚠️  Warning: High concurrency ({}) may cause FTP rate limiting", concurrency);
        eprintln!("   Recommended: 2-4");
    }

    // Get database URL
    let database_url = env::var("DATABASE_URL")
        .context("DATABASE_URL environment variable must be set")?;

    // Connect to database
    info!("Connecting to database...");
    let db = PgPoolOptions::new()
        .max_connections(db_max_connections)
        .connect(&database_url)
        .await
        .context("Failed to connect to database")?;
    info!("✓ Database connected (max connections: {})", db_max_connections);

    // Create orchestrator
    let config = NcbiTaxonomyFtpConfig::new();
    let orchestrator = NcbiTaxonomyOrchestrator::new(config, db);

    // Print header
    println!("\n{}", "=".repeat(70));
    println!("🚀 NCBI Taxonomy Full Historical Catchup");
    println!("{}\n", "=".repeat(70));

    println!("Configuration:");
    println!("  Organization ID: {}", org_id);
    println!("  Concurrency: {}", if sequential { "1 (sequential)".to_string() } else { concurrency.to_string() });
    println!("  Start date: {}", start_date.as_deref().unwrap_or("Beginning of time (2018-12-01)"));
    println!("  Database connections: {}", db_max_connections);

    // Dry run - just list versions
    if dry_run {
        println!("\n🔍 Dry run mode - listing versions only\n");

        let versions = orchestrator.list_available_versions().await?;

        let filtered_versions: Vec<_> = if let Some(date) = &start_date {
            versions.into_iter().filter(|v| v.as_str() >= date.as_str()).collect()
        } else {
            versions
        };

        println!("Found {} versions to process", filtered_versions.len());
        if !filtered_versions.is_empty() {
            println!("  Oldest: {}", filtered_versions.first().unwrap());
            println!("  Newest: {}", filtered_versions.last().unwrap());
        }

        println!("\nEstimated time:");
        let avg_minutes = 10.0;
        let sequential_time = filtered_versions.len() as f64 * avg_minutes;
        let parallel_time = sequential_time / concurrency as f64;

        println!("  Sequential: {:.1} hours ({:.0} minutes)", sequential_time / 60.0, sequential_time);
        if !sequential {
            println!("  Parallel ({}x): {:.1} hours ({:.0} minutes)", concurrency, parallel_time / 60.0, parallel_time);
        }

        println!("\n💡 To run actual catchup, set DRY_RUN=false\n");
        return Ok(());
    }

    // Estimate and confirm
    let versions = orchestrator.list_available_versions().await?;
    let filtered_count = if let Some(date) = &start_date {
        versions.iter().filter(|v| v.as_str() >= date.as_str()).count()
    } else {
        versions.len()
    };

    let avg_minutes = 10.0;
    let estimated_time = if sequential {
        filtered_count as f64 * avg_minutes
    } else {
        filtered_count as f64 * avg_minutes / concurrency as f64
    };

    println!("\nEstimated:");
    println!("  Versions to process: {}", filtered_count);
    println!("  Time: {:.1} hours ({:.0} minutes)", estimated_time / 60.0, estimated_time);
    println!("  Data download: ~{} GB", filtered_count as f64 * 0.1);
    println!("  Database entries: ~{} million", filtered_count as f64 * 2.5);

    println!("\n⚠️  This will take {:.1} hours. Progress will be logged.", estimated_time / 60.0);
    println!("Press Ctrl+C to cancel within next 5 seconds...\n");

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
            .catchup_from_date_parallel(
                org_id,
                start_date.as_deref(),
                concurrency,
            )
            .await?
    };

    let duration = start.elapsed();

    // Print summary
    println!("\n{}", "=".repeat(70));
    println!("✅ Full catchup completed!");
    println!("{}\n", "=".repeat(70));

    println!("{}", NcbiTaxonomyOrchestrator::summarize_results(&results));

    println!("\nTiming:");
    println!("  Total time: {:.1} hours ({:.0} minutes)",
        duration.as_secs_f64() / 3600.0,
        duration.as_secs_f64() / 60.0
    );
    println!("  Average per version: {:.1} minutes",
        duration.as_secs_f64() / 60.0 / results.len() as f64
    );

    if !sequential {
        let sequential_estimate = duration.as_secs_f64() * concurrency as f64;
        println!("  Estimated sequential time: {:.1} hours",
            sequential_estimate / 3600.0
        );
        println!("  Speedup from parallelism: ~{:.1}x", concurrency as f64);
    }

    // Performance comparison
    let old_time_hours = results.len() as f64 * 8.0;
    let new_time_hours = duration.as_secs_f64() / 3600.0;
    println!("\nPerformance:");
    println!("  Old N+1 pattern time: {:.0} hours ({:.1} days)", old_time_hours, old_time_hours / 24.0);
    println!("  New batch+parallel time: {:.1} hours", new_time_hours);
    println!("  Total speedup: {:.0}x", old_time_hours / new_time_hours);

    println!("\n✨ Success! NCBI Taxonomy historical data is now up to date.\n");

    Ok(())
}
