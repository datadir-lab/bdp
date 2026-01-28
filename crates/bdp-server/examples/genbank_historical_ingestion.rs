//! GenBank/RefSeq historical ingestion example
//!
//! Demonstrates how to ingest multiple historical versions of GenBank/RefSeq
//!
//! Usage:
//! ```bash
//! # Ingest all available versions
//! cargo run --example genbank_historical_ingestion -- --database genbank
//!
//! # Ingest from a specific release onwards
//! cargo run --example genbank_historical_ingestion -- --database genbank --from-release 255
//!
//! # Dry run to see what would be ingested
//! cargo run --example genbank_historical_ingestion -- --database genbank --dry-run
//! ```

use anyhow::Result;
use bdp_server::config::ServerConfig;
use bdp_server::ingest::genbank::{GenbankFtpConfig, GenbankPipeline, VersionDiscovery};
use bdp_server::storage::Storage;
use clap::Parser;
use sqlx::PgPool;
use std::time::Instant;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Source database (genbank or refseq)
    #[clap(long, default_value = "genbank")]
    database: String,

    /// Organization ID (default: NCBI)
    #[clap(long)]
    organization_id: Option<Uuid>,

    /// Division to ingest (default: phage for testing)
    #[clap(long, default_value = "phage")]
    division: String,

    /// Filter to versions from this release number onwards
    #[clap(long)]
    from_release: Option<i32>,

    /// Dry run - discover versions but don't ingest
    #[clap(long)]
    dry_run: bool,

    /// Parse limit per file (for testing)
    #[clap(long)]
    parse_limit: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();

    // Load configuration
    let config = ServerConfig::from_env()?;
    let db = PgPool::connect(&config.database_url).await?;
    let s3 = Storage::from_config(&config.storage).await?;

    // Default to NCBI organization
    let organization_id = args.organization_id.unwrap_or_else(|| {
        // In a real application, this would be looked up from the database
        Uuid::parse_str("00000000-0000-0000-0000-000000000001")
            .expect("Invalid default organization ID")
    });

    // Create configuration based on database type
    let mut ftp_config = match args.database.to_lowercase().as_str() {
        "genbank" => {
            info!("Historical ingestion for GenBank");
            GenbankFtpConfig::new().with_genbank()
        }
        "refseq" => {
            info!("Historical ingestion for RefSeq");
            GenbankFtpConfig::new().with_refseq()
        }
        _ => {
            eprintln!("Invalid database type. Use 'genbank' or 'refseq'");
            std::process::exit(1);
        }
    };

    // Apply parse limit if specified
    if let Some(limit) = args.parse_limit {
        ftp_config = ftp_config.with_parse_limit(limit);
    }

    // Parse division
    let division = match args.division.to_lowercase().as_str() {
        "phage" => bdp_server::ingest::genbank::models::Division::Phage,
        "viral" => bdp_server::ingest::genbank::models::Division::Viral,
        "bacterial" => bdp_server::ingest::genbank::models::Division::Bacterial,
        _ => {
            eprintln!("Invalid division. Use 'phage', 'viral', or 'bacterial'");
            std::process::exit(1);
        }
    };

    // Discover versions
    info!("Discovering available versions...");
    let discovery = VersionDiscovery::new(ftp_config.clone());
    let mut versions = discovery.discover_all_versions().await?;

    // Apply release filter if specified
    if let Some(from_release) = args.from_release {
        info!("Filtering versions from release {} onwards", from_release);
        versions = discovery.filter_from_release(versions, from_release);
    }

    // Get already ingested versions from database
    // This would query the versions table in a real implementation
    let ingested_versions: Vec<String> = Vec::new();

    // Filter to only new versions
    let new_versions = discovery.filter_new_versions(versions.clone(), ingested_versions);

    // Display what will be ingested
    info!("Found {} total versions", versions.len());
    info!("Found {} new versions to ingest", new_versions.len());

    if new_versions.is_empty() {
        info!("No new versions to ingest. All up-to-date!");
        return Ok(());
    }

    println!("\nVersions to ingest:");
    println!("{:<20} {:<15} {:<12}", "Version", "Release #", "Date");
    println!("{}", "-".repeat(50));
    for version in &new_versions {
        println!(
            "{:<20} {:<15} {:<12}",
            version.external_version,
            version.release_number,
            version.release_date.format("%Y-%m-%d")
        );
    }

    if args.dry_run {
        info!("Dry run complete. No data was ingested.");
        return Ok(());
    }

    // Ingest each version
    let pipeline = GenbankPipeline::new(ftp_config, db.clone(), s3);
    let total_start = Instant::now();

    for (i, version) in new_versions.iter().enumerate() {
        info!(
            "Processing version {}/{}: {}",
            i + 1,
            new_versions.len(),
            version.external_version
        );

        match pipeline
            .run_division(organization_id, division.clone(), &version.external_version)
            .await
        {
            Ok(result) => {
                info!(
                    "Successfully ingested {}: {} records, {} sequences, {:.2}s",
                    version.external_version,
                    result.records_processed,
                    result.sequences_inserted,
                    result.duration_seconds
                );
            }
            Err(e) => {
                warn!(
                    "Failed to ingest {}: {}",
                    version.external_version, e
                );
                // Continue with next version
                continue;
            }
        }
    }

    let total_duration = total_start.elapsed();
    info!(
        "Historical ingestion complete: {} versions in {:.2}s",
        new_versions.len(),
        total_duration.as_secs_f64()
    );

    Ok(())
}
