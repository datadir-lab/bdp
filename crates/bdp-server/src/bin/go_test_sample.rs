// Gene Ontology Test Binary
// Tests GO ingestion with sample/limited data

use anyhow::{Context, Result};
use bdp_server::config::Config;
use bdp_server::ingest::gene_ontology::{GoHttpConfig, GoPipeline};
use bdp_server::storage::{config::StorageConfig, Storage};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting GO Test");

    // Load configuration
    let app_config = Config::load().context("Failed to load config")?;

    // Connect to database
    info!("Connecting to database...");
    let db = PgPool::connect(&app_config.database.url)
        .await
        .context("Failed to connect to database")?;

    // Get or create organization
    let org_id = get_or_create_org(&db).await?;
    info!("Using organization: {}", org_id);

    // Test configuration with parse limit for quick testing
    let config = GoHttpConfig::builder()
        .go_release_version("current".to_string())
        .goa_release_version("current".to_string())
        .parse_limit(100) // Limit to first 100 terms/annotations for testing
        .build();

    info!("Test configuration:");
    info!("  GO release: {}", config.go_release_version);
    info!("  GOA release: {}", config.goa_release_version);
    info!("  Parse limit: {:?}", config.parse_limit);

    // Create storage
    let storage_config = StorageConfig::from_env()?;
    let storage = Storage::new(storage_config).await?;

    // Create pipeline
    let pipeline = GoPipeline::new(config, db.clone(), storage, org_id);

    // Test 1: Ontology ingestion
    info!("\n=== Test 1: GO Ontology Ingestion ===");
    match pipeline.run_ontology("1.0").await {
        Ok(stats) => {
            info!("✓ Ontology ingestion succeeded");
            info!("  Terms stored: {}", stats.terms_stored);
            info!("  Relationships stored: {}", stats.relationships_stored);
        },
        Err(e) => {
            warn!("✗ Ontology ingestion failed: {}", e);
            warn!("  This is expected if network is unavailable or URL has changed");
        },
    }

    // Verify terms in database
    let term_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM go_term_metadata")
        .fetch_one(&db)
        .await?;
    info!("Database verification: {} GO terms stored", term_count);

    if term_count > 0 {
        // Show sample terms
        let sample_terms: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT go_id, name, namespace FROM go_term_metadata ORDER BY go_id LIMIT 5",
        )
        .fetch_all(&db)
        .await?;

        info!("Sample GO terms:");
        for (go_id, name, namespace) in sample_terms {
            info!("  {} - {} ({})", go_id, name, namespace);
        }

        // Show relationship count
        let rel_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM go_relationships")
            .fetch_one(&db)
            .await?;
        info!("Relationships stored: {}", rel_count);
    }

    // Test 2: Annotations ingestion (if we have proteins in DB)
    let protein_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM protein_metadata")
        .fetch_one(&db)
        .await?;

    if protein_count > 0 {
        info!("\n=== Test 2: GO Annotations Ingestion ===");
        info!("Found {} proteins in database", protein_count);

        match pipeline.run_annotations().await {
            Ok(stats) => {
                info!("✓ Annotations ingestion succeeded");
                info!("  Annotations stored: {}", stats.annotations_stored);
            },
            Err(e) => {
                warn!("✗ Annotations ingestion failed: {}", e);
                warn!("  This is expected if network is unavailable or URL has changed");
            },
        }

        // Verify annotations
        let ann_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM go_annotations")
            .fetch_one(&db)
            .await?;
        info!("Database verification: {} GO annotations stored", ann_count);

        if ann_count > 0 {
            // Show sample annotations
            let sample_anns: Vec<(String, String, String)> = sqlx::query_as(
                "SELECT p.accession, a.go_id, a.evidence_code
                 FROM go_annotations a
                 JOIN protein_metadata p ON p.data_source_id = a.entity_id
                 LIMIT 5",
            )
            .fetch_all(&db)
            .await?;

            info!("Sample GO annotations:");
            for (accession, go_id, evidence_code) in sample_anns {
                info!("  {} -> {} ({})", accession, go_id, evidence_code);
            }
        }
    } else {
        info!("\n=== Test 2: Skipping annotations (no proteins in database) ===");
        info!("To test annotations, first ingest some proteins with UniProt");
    }

    info!("\n=== GO Test Complete ===");

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
