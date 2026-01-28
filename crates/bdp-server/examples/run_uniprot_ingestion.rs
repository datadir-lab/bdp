//! Manual trigger for UniProt ingestion
//!
//! Usage: cargo run --example run_uniprot_ingestion

use anyhow::Result;
use bdp_server::ingest::framework::BatchConfig;
use bdp_server::ingest::uniprot::{IdempotentUniProtPipeline, UniProtFtpConfig, VersionDiscovery};
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

    info!("=== Running UniProt Protein Ingestion ===");

    // Connect to database (Docker)
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_dev_password@localhost:5432/bdp".to_string());

    info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await?;
    info!("Connected to database");

    // Get or create organization
    let org_id = get_or_create_organization(&pool).await?;
    info!(org_id = %org_id, "Using organization");

    // Initialize S3/MinIO storage
    let storage_config = StorageConfig::from_env()?;
    let storage = Storage::new(storage_config).await?;
    info!("Storage client initialized");

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

    info!(
        ftp_host = %ftp_config.ftp_host,
        ftp_path = %ftp_config.ftp_base_path,
        parse_batch_size = 1000,
        store_batch_size = 100,
        "Configuration"
    );

    // Check for new versions
    info!("Checking for available protein data versions...");
    let discovery = VersionDiscovery::new(ftp_config);

    match discovery.check_for_newer_version(&pool, org_id).await? {
        Some(version) => {
            info!(
                version = %version.external_version,
                is_current = version.is_current,
                "Found version to ingest"
            );
            info!("Starting ingestion... This will download and process UniProt protein data.");
            info!("Note: Full ingestion can take hours for complete dataset.");

            // Run idempotent ingestion
            let stats = pipeline.run_idempotent().await?;

            info!("=== Ingestion Complete ===");
            info!(
                discovered = stats.discovered_count,
                already_ingested = stats.already_ingested_count,
                newly_ingested = stats.newly_ingested_count,
                failed = stats.failed_count,
                "Statistics"
            );
        },
        None => {
            let last = discovery.get_last_ingested_version(&pool, org_id).await?;
            match last {
                Some(v) => info!(version = %v, "Already up to date"),
                None => {
                    warn!("No versions found to ingest. This might indicate:");
                    warn!("  - FTP connection issues");
                    warn!("  - No data available");
                    warn!("  - Configuration problems");
                },
            }
        },
    }

    Ok(())
}

async fn get_or_create_organization(pool: &sqlx::PgPool) -> Result<Uuid> {
    const UNIPROT_SLUG: &str = "uniprot";

    // Check for existing UniProt organization by slug (unique identifier)
    let result = sqlx::query!(r#"SELECT id FROM organizations WHERE slug = $1"#, UNIPROT_SLUG)
        .fetch_optional(pool)
        .await?;

    if let Some(record) = result {
        Ok(record.id)
    } else {
        // Create organization with full metadata
        let id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (
                id, name, slug, description, website, is_system,
                license, license_url, citation, citation_url,
                version_strategy, version_description,
                data_source_url, documentation_url, contact_email
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (slug) DO NOTHING
            "#,
            id,
            "Universal Protein Resource",
            UNIPROT_SLUG,
            "UniProt Knowledgebase - Protein sequences and functional information",
            Some("https://www.uniprot.org"),
            true,
            Some("CC-BY-4.0"),
            Some("https://creativecommons.org/licenses/by/4.0/"),
            Some("UniProt Consortium (2023). UniProt: the Universal Protein Knowledgebase in 2023. Nucleic Acids Research."),
            Some("https://www.uniprot.org/help/publications"),
            Some("date-based"),
            Some("UniProt releases follow YYYY_MM format (e.g., 2025_01). Each release is a complete snapshot of the database."),
            Some("https://ftp.uniprot.org/pub/databases/uniprot/"),
            Some("https://www.uniprot.org/help"),
            Some("help@uniprot.org")
        )
        .execute(pool)
        .await?;

        // Fetch the ID in case another process created it concurrently
        let record = sqlx::query!(r#"SELECT id FROM organizations WHERE slug = $1"#, UNIPROT_SLUG)
            .fetch_one(pool)
            .await?;

        Ok(record.id)
    }
}
