//! Gene Ontology Historical Ingestion Example
//!
//! Demonstrates how to:
//! 1. Discover available GO versions from the release archive
//! 2. Filter out already-ingested versions
//! 3. Ingest historical versions in chronological order
//!
//! Usage:
//! ```bash
//! # Discover all available versions
//! cargo run --example go_historical_ingestion -- discover
//!
//! # Check for new versions to ingest
//! cargo run --example go_historical_ingestion -- check
//!
//! # Ingest all versions from 2024-01-01 onwards
//! cargo run --example go_historical_ingestion -- backfill 2024-01-01
//!
//! # Ingest a specific version
//! cargo run --example go_historical_ingestion -- ingest 2025-01-01
//! ```

use anyhow::{Context, Result};
use bdp_server::config::load_config;
use bdp_server::ingest::gene_ontology::{GoHttpConfig, GoPipeline, VersionDiscovery};
use bdp_server::storage::Storage;
use chrono::NaiveDate;
use sqlx::PgPool;
use std::env;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "discover" => discover_versions().await?,
        "check" => check_for_new_versions().await?,
        "backfill" => {
            if args.len() < 3 {
                eprintln!("Error: backfill requires a start date (YYYY-MM-DD)");
                print_usage();
                return Ok(());
            }
            let start_date = parse_date(&args[2])?;
            backfill_versions(start_date, None).await?;
        },
        "backfill-range" => {
            if args.len() < 4 {
                eprintln!("Error: backfill-range requires start and end dates (YYYY-MM-DD)");
                print_usage();
                return Ok(());
            }
            let start_date = parse_date(&args[2])?;
            let end_date = parse_date(&args[3])?;
            backfill_versions(start_date, Some(end_date)).await?;
        },
        "ingest" => {
            if args.len() < 3 {
                eprintln!("Error: ingest requires a version (YYYY-MM-DD)");
                print_usage();
                return Ok(());
            }
            ingest_version(&args[2]).await?;
        },
        _ => {
            eprintln!("Error: Unknown command '{}'", command);
            print_usage();
        },
    }

    Ok(())
}

fn print_usage() {
    eprintln!("Gene Ontology Historical Ingestion");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    cargo run --example go_historical_ingestion -- <COMMAND> [ARGS]");
    eprintln!();
    eprintln!("COMMANDS:");
    eprintln!("    discover                     List all available GO versions");
    eprintln!("    check                        Check for new versions to ingest");
    eprintln!("    backfill <START_DATE>        Ingest all versions from START_DATE onwards");
    eprintln!("    backfill-range <START> <END> Ingest versions within date range");
    eprintln!("    ingest <VERSION>             Ingest a specific version");
    eprintln!();
    eprintln!("DATE FORMAT:");
    eprintln!("    YYYY-MM-DD (e.g., 2024-01-01)");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    cargo run --example go_historical_ingestion -- discover");
    eprintln!("    cargo run --example go_historical_ingestion -- check");
    eprintln!("    cargo run --example go_historical_ingestion -- backfill 2024-01-01");
    eprintln!(
        "    cargo run --example go_historical_ingestion -- backfill-range 2024-01-01 2024-12-31"
    );
    eprintln!("    cargo run --example go_historical_ingestion -- ingest 2025-01-01");
}

fn parse_date(date_str: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .with_context(|| format!("Invalid date format: {}. Expected YYYY-MM-DD", date_str))
}

/// Discover and list all available GO versions
async fn discover_versions() -> Result<()> {
    info!("Discovering available GO versions...");

    let config = GoHttpConfig::default();
    let discovery = VersionDiscovery::new(config)?;

    let versions = discovery.discover_all_versions().await?;

    println!("\nFound {} available GO versions:", versions.len());
    println!();
    println!("{:<15} {:<12} {}", "Version", "Date", "URL");
    println!("{:-<80}", "");

    for version in versions.iter().rev().take(20) {
        // Show most recent 20
        println!(
            "{:<15} {:<12} {}",
            version.external_version, version.release_date, version.release_url
        );
    }

    if versions.len() > 20 {
        println!("\n... and {} more versions", versions.len() - 20);
    }

    println!();
    info!("Discovery complete");

    Ok(())
}

/// Check for new versions that need to be ingested
async fn check_for_new_versions() -> Result<()> {
    info!("Checking for new GO versions to ingest...");

    // Load config and connect to database
    let config = load_config().context("Failed to load configuration")?;
    let db = PgPool::connect(&config.database_url)
        .await
        .context("Failed to connect to database")?;

    // Get GO organization (you'll need to adjust this based on your setup)
    let go_org_id = get_go_organization_id(&db).await?;

    // Get GO registry entry ID
    let go_entry_id = get_go_entry_id(&db, go_org_id).await?;

    let go_config = GoHttpConfig::default();
    let discovery = VersionDiscovery::new(go_config)?;

    // Get all discovered versions
    let discovered = discovery.discover_all_versions().await?;
    info!("Discovered {} total versions", discovered.len());

    // Get already ingested versions
    let ingested = discovery.get_ingested_versions(&db, go_entry_id).await?;
    info!("Already ingested {} versions", ingested.len());

    // Filter to new versions
    let new_versions = discovery.filter_new_versions(discovered, ingested);

    if new_versions.is_empty() {
        println!("\n✅ All available GO versions have been ingested!");
        println!();
    } else {
        println!("\n📦 Found {} new versions to ingest:", new_versions.len());
        println!();
        println!("{:<15} {:<12}", "Version", "Date");
        println!("{:-<30}", "");

        for version in &new_versions {
            println!("{:<15} {:<12}", version.external_version, version.release_date);
        }

        println!();
        println!(
            "💡 Run with 'backfill {}' to ingest these versions",
            new_versions
                .first()
                .map(|v| v.external_version.as_str())
                .unwrap_or("YYYY-MM-DD")
        );
        println!();
    }

    Ok(())
}

/// Backfill historical versions within a date range
async fn backfill_versions(start_date: NaiveDate, end_date: Option<NaiveDate>) -> Result<()> {
    info!(
        "Starting GO historical backfill from {} to {}",
        start_date,
        end_date
            .map(|d| d.to_string())
            .unwrap_or_else(|| "latest".to_string())
    );

    // Load config and connect to database
    let config = load_config().context("Failed to load configuration")?;
    let db = PgPool::connect(&config.database_url)
        .await
        .context("Failed to connect to database")?;

    // Get GO organization and entry ID
    let go_org_id = get_go_organization_id(&db).await?;
    let go_entry_id = get_go_entry_id(&db, go_org_id).await?;

    // Discover versions to backfill
    let go_config = GoHttpConfig::default();
    let discovery = VersionDiscovery::new(go_config.clone())?;

    let versions = discovery
        .get_versions_for_backfill(&db, go_entry_id, start_date, end_date)
        .await?;

    if versions.is_empty() {
        println!("\n✅ No new versions to backfill in the specified range");
        return Ok(());
    }

    println!("\n📦 Will ingest {} GO versions:", versions.len());
    for version in &versions {
        println!("  - {} ({})", version.external_version, version.release_date);
    }
    println!();

    // Set up S3 storage
    let s3 = Storage::new_from_config(&config.storage)
        .await
        .context("Failed to initialize S3 storage")?;

    // Create pipeline
    let pipeline = GoPipeline::new(go_config, db.clone(), s3, go_org_id);

    // Ingest each version
    for (idx, version) in versions.iter().enumerate() {
        info!(
            "Processing version {}/{}: {}",
            idx + 1,
            versions.len(),
            version.external_version
        );

        match ingest_version_impl(&pipeline, &version.external_version).await {
            Ok(stats) => {
                info!(
                    "✅ Ingested {}: {} terms, {} relationships",
                    version.external_version, stats.terms_stored, stats.relationships_stored
                );
            },
            Err(e) => {
                warn!("❌ Failed to ingest {}: {}", version.external_version, e);
                // Continue with next version instead of failing entirely
            },
        }
    }

    println!("\n✅ Historical backfill complete!");

    Ok(())
}

/// Ingest a specific GO version
async fn ingest_version(version: &str) -> Result<()> {
    info!("Ingesting GO version: {}", version);

    // Validate version format
    parse_date(version)?;

    // Load config and connect to database
    let config = load_config().context("Failed to load configuration")?;
    let db = PgPool::connect(&config.database_url)
        .await
        .context("Failed to connect to database")?;

    // Get GO organization
    let go_org_id = get_go_organization_id(&db).await?;

    // Set up S3 storage
    let s3 = Storage::new_from_config(&config.storage)
        .await
        .context("Failed to initialize S3 storage")?;

    // Create pipeline with version-specific config
    let go_config = GoHttpConfig::builder()
        .go_release_version(version.to_string())
        .build();

    let pipeline = GoPipeline::new(go_config, db, s3, go_org_id);

    // Run ingestion
    let stats = ingest_version_impl(&pipeline, version).await?;

    println!("\n✅ Ingestion complete!");
    println!("   Terms stored: {}", stats.terms_stored);
    println!("   Relationships stored: {}", stats.relationships_stored);
    println!();

    Ok(())
}

/// Internal helper to ingest a version using a pipeline
async fn ingest_version_impl(
    pipeline: &GoPipeline,
    external_version: &str,
) -> Result<bdp_server::ingest::gene_ontology::PipelineStats> {
    // Use external version as internal version for simplicity
    // In production, you might want different versioning schemes
    let internal_version = external_version;

    pipeline
        .run_ontology_version(internal_version, Some(external_version))
        .await
        .context("Failed to run GO pipeline")
}

/// Get or create GO organization ID
async fn get_go_organization_id(db: &PgPool) -> Result<Uuid> {
    // In a real implementation, you'd query for the actual GO organization
    // For this example, we'll look it up by name
    let result = sqlx::query!(
        r#"
        SELECT id FROM organizations
        WHERE name = 'Gene Ontology Consortium'
        LIMIT 1
        "#
    )
    .fetch_optional(db)
    .await?;

    match result {
        Some(record) => Ok(record.id),
        None => {
            anyhow::bail!("GO organization not found. Please create it first using the server API.")
        },
    }
}

/// Get GO registry entry ID
async fn get_go_entry_id(db: &PgPool, organization_id: Uuid) -> Result<Uuid> {
    let result = sqlx::query!(
        r#"
        SELECT id FROM registry_entries
        WHERE organization_id = $1
          AND name LIKE '%gene-ontology%'
        LIMIT 1
        "#,
        organization_id
    )
    .fetch_optional(db)
    .await?;

    match result {
        Some(record) => Ok(record.id),
        None => {
            anyhow::bail!(
                "GO registry entry not found. Please create it first using the server API."
            )
        },
    }
}
