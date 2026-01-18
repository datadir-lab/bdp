//! Manual trigger for UniProt ingestion
//!
//! Usage: cargo run --example run_uniprot_ingestion

use anyhow::Result;
use bdp_server::ingest::uniprot::{
    UniProtFtpConfig, IdempotentUniProtPipeline, VersionDiscovery,
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

    println!("=== Running UniProt Protein Ingestion ===\n");

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

    // Initialize S3/MinIO storage
    let storage_config = StorageConfig::from_env()?;
    let storage = Storage::new(storage_config).await?;
    println!("✓ Storage client initialized\n");

    // Create ingestion pipeline
    let ftp_config = UniProtFtpConfig::default();
    let batch_config = BatchConfig::default();

    let pipeline = IdempotentUniProtPipeline::new(
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

    // Check for new versions
    info!("Checking for available protein data versions...");
    let discovery = VersionDiscovery::new(ftp_config);

    match discovery.check_for_newer_version(&pool, org_id).await? {
        Some(version) => {
            println!("✓ Found version to ingest: {} (current: {})",
                version.external_version, version.is_current);
            println!("\nStarting ingestion...");
            println!("This will download and process UniProt protein data.");
            println!("Note: Full ingestion can take hours for complete dataset.\n");

            // Run idempotent ingestion
            let stats = pipeline.run_idempotent().await?;

            println!("\n=== Ingestion Complete ===");
            println!("Versions discovered: {}", stats.discovered_count);
            println!("Already ingested: {}", stats.already_ingested_count);
            println!("Newly ingested: {}", stats.newly_ingested_count);
            println!("Failed: {}", stats.failed_count);
        }
        None => {
            let last = discovery.get_last_ingested_version(&pool, org_id).await?;
            match last {
                Some(v) => println!("✓ Already up to date (version: {})", v),
                None => {
                    warn!("No versions found to ingest. This might indicate:");
                    warn!("  - FTP connection issues");
                    warn!("  - No data available");
                    warn!("  - Configuration problems");
                }
            }
        }
    }

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
