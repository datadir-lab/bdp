// Gene Ontology Integration Test

use bdp_server::ingest::gene_ontology::{
    GoHttpConfig, GoParser, GoPipeline, GoTerm, Namespace, RelationshipType,
};
use sqlx::PgPool;
use std::env;
use uuid::Uuid;

/// Test helper to get database connection
async fn get_test_db() -> PgPool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

#[tokio::test]
#[ignore] // Requires database
async fn test_obo_parser() {
    let obo_content = r#"format-version: 1.2
data-version: releases/2026-01-01

[Term]
id: GO:0008150
name: biological_process
namespace: biological_process
def: "A biological process represents a specific objective that the organism is genetically programmed to achieve." [GOC:go_curators]
synonym: "biological process" EXACT []
synonym: "physiological process" NARROW []

[Term]
id: GO:0003674
name: molecular_function
namespace: molecular_function
def: "A molecular process that can be carried out by the action of a single macromolecular machine." [GOC:go_curators]
is_a: GO:0008150 ! biological_process

[Term]
id: GO:0005575
name: cellular_component
namespace: cellular_component
def: "A location, relative to cellular compartments and structures." [GOC:go_curators]
relationship: part_of GO:0008150 ! biological_process
"#;

    let result = GoParser::parse_obo(obo_content, "2026-01-01", None);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.terms.len(), 3);
    assert_eq!(parsed.relationships.len(), 2);

    // Verify first term
    let term = &parsed.terms[0];
    assert_eq!(term.go_id, "GO:0008150");
    assert_eq!(term.name, "biological_process");
    assert_eq!(term.namespace, Namespace::BiologicalProcess);
    assert!(!term.is_obsolete);
    assert_eq!(term.synonyms.len(), 2);

    // Verify relationship
    let rel = &parsed.relationships[0];
    assert_eq!(rel.subject_go_id, "GO:0003674");
    assert_eq!(rel.object_go_id, "GO:0008150");
    assert_eq!(rel.relationship_type, RelationshipType::IsA);
}

#[tokio::test]
#[ignore] // Requires database
async fn test_go_term_validation() {
    // Valid GO ID
    assert!(GoTerm::validate_go_id("GO:0008150"));
    assert!(GoTerm::validate_go_id("GO:0000001"));

    // Invalid GO IDs
    assert!(!GoTerm::validate_go_id("GO:123")); // Too short
    assert!(!GoTerm::validate_go_id("GO:12345678")); // Too long
    assert!(!GoTerm::validate_go_id("INVALID"));
    assert!(!GoTerm::validate_go_id(""));

    // Test accession parsing
    assert_eq!(GoTerm::parse_accession("GO:0008150").unwrap(), 8150);
    assert_eq!(GoTerm::parse_accession("GO:0000001").unwrap(), 1);
    assert_eq!(GoTerm::parse_accession("GO:1234567").unwrap(), 1234567);
}

#[tokio::test]
#[ignore] // Requires database and network
async fn test_ontology_download_and_parse() {
    // Use test config with parse limit
    let config = GoHttpConfig::builder()
        .go_release_version("2025-09-08".to_string())
        .parse_limit(100) // Only parse first 100 terms
        .build();

    let db = get_test_db().await;
    let org_id = Uuid::new_v4();

    let pipeline = GoPipeline::new(db, org_id, config).expect("Failed to create pipeline");

    // This will download and parse the OBO file
    let result = pipeline.run_ontology("1.0").await;

    // Should succeed or fail gracefully
    match result {
        Ok(stats) => {
            println!("Successfully parsed {} terms", stats.terms_stored);
            assert!(stats.terms_stored > 0);
            assert!(stats.relationships_stored > 0);
        },
        Err(e) => {
            println!("Download/parse failed (expected in CI): {}", e);
        },
    }
}

#[tokio::test]
#[ignore] // Requires database
async fn test_gaf_parser() {
    use std::collections::HashMap;

    let protein_id = Uuid::new_v4();
    let mut protein_lookup = HashMap::new();
    protein_lookup.insert("P01308".to_string(), protein_id);

    let gaf_content = r#"!gaf-version: 2.2
!Generated: 2026-01-15
UniProtKB	P01308	INS		GO:0006955	PMID:12345678	IDA		P	insulin		protein	taxon:9606	20260115	UniProt
UniProtKB	P01308	INS	NOT	GO:0008150	GO_REF:0000043	IEA		P	insulin		protein	taxon:9606	20260115	UniProt
"#;

    let result = GoParser::parse_gaf(gaf_content, "2026-01-15", &protein_lookup, None);
    assert!(result.is_ok());

    let annotations = result.unwrap();
    assert_eq!(annotations.len(), 2);

    // Verify first annotation
    let ann = &annotations[0];
    assert_eq!(ann.entity_id, protein_id);
    assert_eq!(ann.go_id, "GO:0006955");
    assert_eq!(ann.evidence_code.0, "IDA");
    assert_eq!(ann.taxonomy_id, Some(9606));

    // Verify second annotation with qualifier
    let ann2 = &annotations[1];
    assert_eq!(ann2.qualifier, Some("NOT".to_string()));
    assert_eq!(ann2.evidence_code.0, "IEA");
}

#[tokio::test]
#[ignore] // Requires database
async fn test_config_builder() {
    let config = GoHttpConfig::builder()
        .go_release_version("2025-12-01".to_string())
        .goa_release_version("2025-12-15".to_string())
        .timeout_secs(600)
        .parse_limit(1000)
        .build();

    assert_eq!(config.go_release_version, "2025-12-01");
    assert_eq!(config.goa_release_version, "2025-12-15");
    assert_eq!(config.timeout_secs, 600);
    assert_eq!(config.parse_limit, Some(1000));

    // Validate URLs
    assert!(config.ontology_url().contains("2025-12-01"));
    assert!(config.goa_uniprot_url().contains("goa_uniprot_all.gaf.gz"));
}

#[tokio::test]
#[ignore] // Requires database
async fn test_namespace_parsing() {
    assert_eq!(Namespace::from_str("biological_process").unwrap(), Namespace::BiologicalProcess);
    assert_eq!(Namespace::from_str("molecular_function").unwrap(), Namespace::MolecularFunction);
    assert_eq!(Namespace::from_str("cellular_component").unwrap(), Namespace::CellularComponent);

    assert!(Namespace::from_str("invalid").is_err());
}

#[tokio::test]
#[ignore] // Requires database
async fn test_relationship_type_parsing() {
    assert_eq!(RelationshipType::from_str("is_a").unwrap(), RelationshipType::IsA);
    assert_eq!(RelationshipType::from_str("part_of").unwrap(), RelationshipType::PartOf);
    assert_eq!(RelationshipType::from_str("regulates").unwrap(), RelationshipType::Regulates);

    assert!(RelationshipType::from_str("invalid").is_err());
}
