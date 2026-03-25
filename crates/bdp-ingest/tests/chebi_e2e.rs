mod common;

use bdp_ingest::pipelines::chebi::{
    models::{CompoundRelationship, CompoundTerm, ParsedChebi},
    storage::ChebiStorage,
};

/// Minimal inline ChEBI data for testing (3 compound terms + 1 relationship)
fn sample_chebi() -> ParsedChebi {
    ParsedChebi {
        terms: vec![
            CompoundTerm {
                chebi_id: "CHEBI:15422".to_string(),
                chebi_accession: 15422,
                name: "ATP".to_string(),
                definition: Some("Adenosine 5'-triphosphate".to_string()),
                comment: None,
                is_obsolete: false,
                inchikey: Some("ZKHQWZAMYRWXGA-KQYNXXCUSA-N".to_string()),
                smiles: Some("Nc1ncnc2c1ncn2...".to_string()),
                inchi: None,
                formula: Some("C10H16N5O13P3".to_string()),
                mass_mono: Some(506.9957),
                charge: Some(0),
                chebi_release: "test-2026".to_string(),
            },
            CompoundTerm {
                chebi_id: "CHEBI:30616".to_string(),
                chebi_accession: 30616,
                name: "ATP(4-)".to_string(),
                definition: None,
                comment: None,
                is_obsolete: false,
                inchikey: None,
                smiles: None,
                inchi: None,
                formula: Some("C10H12N5O13P3".to_string()),
                mass_mono: None,
                charge: Some(-4),
                chebi_release: "test-2026".to_string(),
            },
            CompoundTerm {
                chebi_id: "CHEBI:26078".to_string(),
                chebi_accession: 26078,
                name: "obsolete compound".to_string(),
                definition: None,
                comment: None,
                is_obsolete: true,
                inchikey: None,
                smiles: None,
                inchi: None,
                formula: None,
                mass_mono: None,
                charge: None,
                chebi_release: "test-2026".to_string(),
            },
        ],
        relationships: vec![CompoundRelationship {
            subject_chebi_id: "CHEBI:30616".to_string(),
            object_chebi_id: "CHEBI:15422".to_string(),
            relationship_type: "is_a".to_string(),
            chebi_release: "test-2026".to_string(),
        }],
    }
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_chebi_storage_e2e() {
    let pg = common::TestPostgres::start().await.expect("start postgres");
    let org_id = common::create_test_org(&pg.pool, "chebi-test")
        .await
        .expect("create org");

    let parsed = sample_chebi();
    let storage = ChebiStorage::new(pg.pool.clone());
    storage
        .ingest_release(org_id, "test-2026", &parsed)
        .await
        .expect("ingest_release");

    // registry_entries: at least 1 row for 'chebi'
    let reg_count = common::count_rows(&pg.pool, "registry_entries")
        .await
        .unwrap();
    assert!(reg_count >= 1, "registry_entries should have >=1 row");

    // compound_terms: 3 rows
    let term_count = common::count_rows(&pg.pool, "compound_terms")
        .await
        .unwrap();
    assert_eq!(term_count, 3, "expected 3 compound_terms");

    // compound_relationships: 1 row
    let rel_count = common::count_rows(&pg.pool, "compound_relationships")
        .await
        .unwrap();
    assert_eq!(rel_count, 1, "expected 1 compound_relationship");

    // Spot-check: ATP should be findable by chebi_id
    let name: String =
        sqlx::query_scalar("SELECT name FROM compound_terms WHERE chebi_id = 'CHEBI:15422'")
            .fetch_one(&pg.pool)
            .await
            .expect("fetch ATP");
    assert_eq!(name, "ATP");

    // Idempotent: run again, counts should stay the same
    storage
        .ingest_release(org_id, "test-2026", &parsed)
        .await
        .expect("second ingest");
    let term_count2 = common::count_rows(&pg.pool, "compound_terms")
        .await
        .unwrap();
    assert_eq!(term_count2, 3, "idempotent: still 3 terms after second run");
}
