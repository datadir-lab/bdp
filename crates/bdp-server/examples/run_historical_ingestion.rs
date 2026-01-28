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
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,sqlx=warn".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("=== Historical UniProt Ingestion ===");

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

    info!(versions = ?target_versions, "Target versions");

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
    info!(org_id = %org_id, slug = "uniprot", "Using organization");

    // Initialize S3/MinIO storage
    let storage_config = StorageConfig::from_env()?;
    let storage = Storage::new(storage_config).await?;
    info!("Storage client initialized");

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

    info!(
        ftp_host = %ftp_config.ftp_host,
        ftp_path = %ftp_config.ftp_base_path,
        parse_batch_size = 1000,
        store_batch_size = 100,
        "Configuration"
    );

    // Discover all available versions
    info!("Discovering available versions from FTP (previous releases)...");
    let discovery = VersionDiscovery::new(ftp_config);

    let all_versions = match discovery.discover_previous_versions_only().await {
        Ok(versions) => {
            info!(count = versions.len(), "Found historical versions");
            versions
        }
        Err(e) => {
            warn!(error = %e, "Failed to discover versions");
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
        warn!(requested = ?target_versions, "None of the requested versions were found on FTP!");
        return Ok(());
    }

    info!("=== Versions to Ingest ===");
    for version in &versions_to_ingest {
        info!(version = %version.external_version, release_date = %version.release_date, "Version");
    }

    // Ingest each version
    let mut total_succeeded = 0;
    let mut total_failed = 0;

    for version in versions_to_ingest {
        info!(
            version = %version.external_version,
            release_date = %version.release_date,
            ftp_path = %version.ftp_path,
            "Starting ingestion"
        );

        match pipeline.ingest_version(&version).await {
            Ok(job_id) => {
                info!(job_id = %job_id, "Ingestion completed successfully");
                total_succeeded += 1;
            }
            Err(e) => {
                error!(error = %e, "Ingestion failed");
                // Log full error chain for debugging
                let mut current = e.source();
                let mut depth = 1;
                while let Some(err) = current {
                    error!(depth = depth, error = %err, "Error chain");
                    current = err.source();
                    depth += 1;
                }
                total_failed += 1;
            }
        }
    }

    info!(
        succeeded = total_succeeded,
        failed = total_failed,
        "=== Ingestion Summary ==="
    );

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
        let record = sqlx::query!(
            r#"SELECT id FROM organizations WHERE slug = $1"#,
            UNIPROT_SLUG
        )
        .fetch_one(pool)
        .await?;

        Ok(record.id)
    }
}
