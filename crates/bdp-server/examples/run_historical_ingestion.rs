//! Historical ingestion for specific UniProt versions
//!
//! This example fetches specific historical versions from UniProt FTP
//! without relying on current_release (which has FTP passive mode issues).
//!
//! Usage: cargo run --example run_historical_ingestion

use anyhow::Result;
use bdp_server::ingest::uniprot::{
    UniProtPipeline, UniProtFtpConfig, VersionDiscovery,
};
use bdp_server::ingest::framework::BatchConfig;
use bdp_server::storage::{config::StorageConfig, Storage};
use sqlx::postgres::PgPoolOptions;
use std::{sync::Arc, time::Duration};
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,sqlx=warn".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("=== Historical UniProt Ingestion ===\n");

    // Parse command line arguments or use defaults
    let versions_to_fetch = std::env::args()
        .skip(1)
        .collect::<Vec<_>>();

    let target_versions = if versions_to_fetch.is_empty() {
        // Default: Last two 2025 versions
        vec!["2025_01".to_string(), "2025_02".to_string()]
    } else {
        versions_to_fetch
    };

    println!("Target versions: {}", target_versions.join(", "));
    println!();

    // Connect to database (Docker)
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_dev_password@localhost:5432/bdp".to_string());

    info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await?;
    println!("✓ Connected to database\n");

    // Get or create organization
    let org_id = get_or_create_organization(&pool).await?;
    println!("✓ Using organization: {}\n", org_id);
    println!("  Organization slug: uniprot\n");

    // Initialize S3/MinIO storage
    let storage_config = StorageConfig::from_env()?;
    let storage = Storage::new(storage_config).await?;
    println!("✓ Storage client initialized\n");

    // Create ingestion pipeline
    let ftp_config = UniProtFtpConfig::default();
    let batch_config = BatchConfig::default();

    let pipeline = UniProtPipeline::new(
        Arc::new(pool.clone()),
        org_id,
        ftp_config.clone(),
        batch_config,
        storage,
    );

    println!("Configuration:");
    println!("  FTP Host: {}", ftp_config.ftp_host);
    println!("  FTP Path: {}", ftp_config.ftp_base_path);
    println!("  Parse batch size: 1000");
    println!("  Store batch size: 100\n");

    // Discover all available versions
    info!("Discovering available versions from FTP (previous releases)...");
    let discovery = VersionDiscovery::new(ftp_config);

    let all_versions = match discovery.discover_previous_versions_only().await {
        Ok(versions) => {
            info!("Found {} historical versions", versions.len());
            versions
        }
        Err(e) => {
            warn!("Failed to discover versions: {}", e);
            return Err(e);
        }
    };

    if all_versions.is_empty() {
        warn!("No historical versions found!");
        return Ok(());
    }

    // Filter to only requested versions
    let versions_to_ingest: Vec<_> = all_versions
        .into_iter()
        .filter(|v| target_versions.contains(&v.external_version))
        .collect();

    if versions_to_ingest.is_empty() {
        warn!("None of the requested versions were found on FTP!");
        warn!("Requested: {:?}", target_versions);
        return Ok(());
    }

    println!("\n=== Versions to Ingest ===");
    for version in &versions_to_ingest {
        println!("  - {} ({})", version.external_version, version.release_date);
    }
    println!();

    // Ingest each version
    let mut total_succeeded = 0;
    let mut total_failed = 0;

    for version in versions_to_ingest {
        println!("\n▶ Starting ingestion for version: {}", version.external_version);
        println!("  Release date: {}", version.release_date);
        println!("  FTP path: {}", version.ftp_path);
        println!();

        match pipeline.ingest_version(&version).await {
            Ok(job_id) => {
                println!("✓ Ingestion completed successfully!");
                println!("  Job ID: {}", job_id);
                total_succeeded += 1;
            }
            Err(e) => {
                println!("✗ Ingestion failed: {}", e);
                // Print full error chain for debugging
                println!("\nFull error chain:");
                let mut current = e.source();
                let mut depth = 1;
                while let Some(err) = current {
                    println!("  {} {}", depth, err);
                    current = err.source();
                    depth += 1;
                }
                total_failed += 1;
            }
        }
    }

    println!("\n=== Ingestion Summary ===");
    println!("Succeeded: {}", total_succeeded);
    println!("Failed: {}", total_failed);

    Ok(())
}

async fn get_or_create_organization(pool: &sqlx::PgPool) -> Result<Uuid> {
    const UNIPROT_SLUG: &str = "uniprot";

    // Check for existing UniProt organization by slug (unique identifier)
    let result = sqlx::query!(
        r#"SELECT id FROM organizations WHERE slug = $1"#,
        UNIPROT_SLUG
    )
    .fetch_optional(pool)
    .await?;

    if let Some(record) = result {
        Ok(record.id)
    } else {
        // Create organization - this will fail if slug already exists (unique constraint)
        let id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, name, slug, description, is_system)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (slug) DO NOTHING
            "#,
            id,
            "Universal Protein Resource",
            UNIPROT_SLUG,
            "UniProt Knowledgebase - Protein sequences and functional information",
            true
        )
        .execute(pool)
        .await?;

        // Fetch the ID in case another process created it concurrently
        let record = sqlx::query!(
            r#"SELECT id FROM organizations WHERE slug = $1"#,
            UNIPROT_SLUG
        )
        .fetch_one(pool)
        .await?;

        Ok(record.id)
    }
}
