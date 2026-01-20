// Gene Ontology Test Binary - Human Annotations Only
// Tests GO annotation ingestion with smaller human-specific file

use anyhow::{Context, Result};
use bdp_server::config::Config;
use bdp_server::ingest::gene_ontology::{GoHttpConfig, GoPipeline};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting GO Test - Human Annotations");

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

    // Configuration for human annotations (smaller file ~10MB)
    let config = GoHttpConfig::builder()
        .ontology_base_url("ftp://ftp.ebi.ac.uk/pub/databases/GO/goa".to_string())
        .annotation_base_url("ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/HUMAN".to_string())
        .go_release_version("current".to_string())
        .goa_release_version("current".to_string())
        .parse_limit(1000) // Limit to first 1000 annotations for testing
        .build();

    info!("Test configuration:");
    info!("  Annotation source: HUMAN (goa_human.gaf.gz ~10MB)");
    info!("  Parse limit: {:?}", config.parse_limit);

    // Create pipeline
    let pipeline = GoPipeline::new(db.clone(), org_id, config)
        .context("Failed to create GO pipeline")?;

    // Check if we have proteins in DB
    let protein_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM protein_metadata")
        .fetch_one(&db)
        .await?;

    if protein_count == 0 {
        warn!("No proteins found in database - skipping annotation test");
        warn!("Run UniProt ingestion first to populate protein_metadata");
        return Ok(());
    }

    info!("Found {} proteins in database", protein_count);

    // Test: Human annotations ingestion
    info!("\n=== GO Annotations Ingestion (Human) ===");
    match pipeline.run_organism_annotations("human").await {
        Ok(stats) => {
            info!("✓ Annotations ingestion succeeded");
            info!("  Annotations stored: {}", stats.annotations_stored);
        }
        Err(e) => {
            warn!("✗ Annotations ingestion failed: {}", e);
            return Err(e.into());
        }
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
             LIMIT 10",
        )
        .fetch_all(&db)
        .await?;

        info!("Sample GO annotations:");
        for (accession, go_id, evidence_code) in sample_anns {
            info!("  {} -> {} ({})", accession, go_id, evidence_code);
        }

        // Show evidence code distribution
        let evidence_dist: Vec<(String, i64)> = sqlx::query_as(
            "SELECT evidence_code, COUNT(*) as count
             FROM go_annotations
             GROUP BY evidence_code
             ORDER BY count DESC
             LIMIT 10",
        )
        .fetch_all(&db)
        .await?;

        info!("\nEvidence code distribution:");
        for (code, count) in evidence_dist {
            info!("  {}: {} annotations", code, count);
        }
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
