//! Test GO pipeline with local ontology file from Zenodo
//!
//! This test demonstrates the complete GO ingestion workflow:
//! 1. Load GO ontology from local file (go-basic.obo from Zenodo)
//! 2. Download and parse annotations from FTP (human organism)
//! 3. Store in database
//!
//! Prerequisites:
//! - Download Zenodo archive: ./scripts/download_go_zenodo.sh
//! - Set environment variable: GO_LOCAL_ONTOLOGY_PATH=data/go/go-basic.obo
//!
//! Usage:
//!   cargo run --bin go_test_local_ontology

use anyhow::{Context, Result};
use bdp_server::config::Config;
use bdp_server::ingest::gene_ontology::{GoHttpConfig, GoPipeline};
use bdp_server::storage::{config::StorageConfig, Storage};
use sqlx::PgPool;
use std::env;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("=== GO Local Ontology Pipeline Test ===");
    info!("");

    // Load configuration
    let app_config = Config::load().context("Failed to load config")?;

    // Connect to database
    info!("Connecting to database...");
    let pool = PgPool::connect(&app_config.database.url)
        .await
        .context("Failed to connect to database")?;
    info!("✓ Connected to database");

    // Check for local ontology path
    let local_path = env::var("GO_LOCAL_ONTOLOGY_PATH").unwrap_or_else(|_| {
        warn!("GO_LOCAL_ONTOLOGY_PATH not set, using default: data/go/go-basic.obo");
        "data/go/go-basic.obo".to_string()
    });

    info!("Local ontology path: {}", local_path);

    // Check if file exists
    if !std::path::Path::new(&local_path).exists() {
        error!("❌ Local ontology file not found: {}", local_path);
        error!("");
        error!("Please download the Zenodo archive first:");
        error!("  ./scripts/download_go_zenodo.sh");
        error!("");
        error!("Or set GO_LOCAL_ONTOLOGY_PATH to your go-basic.obo location");
        return Err(anyhow::anyhow!("Local ontology file not found"));
    }

    info!("✓ Local ontology file exists");

    // Configure with Zenodo settings + parse limit for testing
    let release_date = env::var("GO_RELEASE_VERSION").unwrap_or_else(|_| "2025-09-08".to_string());
    let zenodo_doi =
        env::var("GO_ZENODO_DOI").unwrap_or_else(|_| "10.5281/zenodo.17382285".to_string());

    let config = GoHttpConfig::builder()
        .local_ontology_path(local_path)
        .annotation_base_url("ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/HUMAN".to_string())
        .go_release_version(release_date.clone())
        .goa_release_version("current".to_string())
        .zenodo_doi(zenodo_doi.clone())
        .citation(format!(
            "Gene Ontology data from the {} release (DOI: {}) is made available under the terms of the Creative Commons Attribution 4.0 International license (CC BY 4.0).",
            release_date, zenodo_doi
        ))
        .parse_limit(1000) // Limit for testing
        .timeout_secs(600)
        .max_retries(3)
        .build();

    info!("Configuration:");
    info!("  GO Release: {}", release_date);
    info!("  Zenodo DOI: {}", zenodo_doi);
    info!("  Parse limit: 1000 (for testing)");
    info!("");

    // Get or create organization for testing
    let org_id = get_or_create_org(&pool).await?;
    info!("Using organization: {}", org_id);

    // Create storage
    let storage_config = StorageConfig::from_env()?;
    let storage = Storage::new(storage_config).await?;

    // Create pipeline
    let pipeline = GoPipeline::new(config, pool.clone(), storage, org_id);

    // Test 1: Ingest GO Ontology from local file
    info!("=== Test 1: GO Ontology Ingestion (Local File) ===");
    match pipeline.run_ontology("1.0").await {
        Ok(stats) => {
            info!("✓ Ontology ingestion succeeded");
            info!("  Terms stored: {}", stats.terms_stored);
            info!("  Relationships stored: {}", stats.relationships_stored);
        },
        Err(e) => {
            warn!("✗ Ontology ingestion failed: {}", e);
            error!("Stopping test due to ontology ingestion failure");
            return Err(e.into());
        },
    }

    info!("");

    // Test 2: Ingest human annotations
    info!("=== Test 2: Human Annotations Ingestion (FTP) ===");
    match pipeline.run_organism_annotations("human").await {
        Ok(stats) => {
            info!("✓ Annotations ingestion succeeded");
            info!("  Annotations stored: {}", stats.annotations_stored);
        },
        Err(e) => {
            warn!("✗ Annotations ingestion failed: {}", e);
        },
    }

    info!("");

    // Verify data in database
    info!("=== Verification ===");

    let term_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM go_term_metadata")
        .fetch_one(&pool)
        .await?;
    info!("GO terms in database: {}", term_count);

    let rel_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM go_relationships")
        .fetch_one(&pool)
        .await?;
    info!("GO relationships in database: {}", rel_count);

    let ann_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM go_annotations")
        .fetch_one(&pool)
        .await?;
    info!("GO annotations in database: {}", ann_count);

    info!("");
    info!("=== Test Complete ===");
    info!("");
    info!("Attribution Notice:");
    info!("  Gene Ontology data from the {} release (DOI: {})", release_date, zenodo_doi);
    info!("  is made available under the terms of the CC BY 4.0 license.");
    info!("");
    info!("To remove parse limit and ingest full dataset:");
    info!("  1. Edit config.parse_limit to None");
    info!("  2. Re-run this test");

    Ok(())
}

/// Get or create organization for testing
async fn get_or_create_org(db: &PgPool) -> Result<Uuid> {
    // Try to get existing organization
    let existing: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM organizations WHERE name = 'test' LIMIT 1")
            .fetch_optional(db)
            .await?;

    if let Some(org_id) = existing {
        return Ok(org_id);
    }

    // Create new organization
    let org_id: Uuid = sqlx::query_scalar(
        "INSERT INTO organizations (id, name, slug, description, created_at, updated_at)
         VALUES (gen_random_uuid(), 'test', 'test', 'Test organization', NOW(), NOW())
         RETURNING id",
    )
    .fetch_one(db)
    .await?;

    Ok(org_id)
}
