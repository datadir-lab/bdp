mod common;

use bdp_ingest::pipelines::mondo::{
    models::{
        DiseaseRelationship, DiseaseRelationType, DiseaseSynonym, DiseaseTerm, DiseaseXref,
        ParsedMondo,
    },
    storage::MondoStorage,
};

fn sample_mondo() -> ParsedMondo {
    ParsedMondo {
        terms: vec![
            DiseaseTerm {
                mondo_id: "MONDO:0000001".to_string(),
                mondo_accession: 1,
                name: "disease".to_string(),
                definition: Some("A disposition to undergo pathological processes.".to_string()),
                is_obsolete: false,
                comment: None,
                omim_id: None,
                orphanet_id: None,
                synonyms: vec![DiseaseSynonym {
                    scope: "EXACT".to_string(),
                    text: "condition".to_string(),
                }],
                xrefs: vec![DiseaseXref {
                    source_db: "MeSH".to_string(),
                    source_id: "D004194".to_string(),
                }],
                mondo_release: "test-2026".to_string(),
            },
            DiseaseTerm {
                mondo_id: "MONDO:0004992".to_string(),
                mondo_accession: 4992,
                name: "cancer".to_string(),
                definition: Some("A disease involving uncontrolled cell growth.".to_string()),
                is_obsolete: false,
                comment: None,
                omim_id: Some("114500".to_string()),
                orphanet_id: Some("68335".to_string()),
                synonyms: vec![
                    DiseaseSynonym {
                        scope: "EXACT".to_string(),
                        text: "malignant neoplasm".to_string(),
                    },
                    DiseaseSynonym {
                        scope: "BROAD".to_string(),
                        text: "malignancy".to_string(),
                    },
                ],
                xrefs: vec![
                    DiseaseXref {
                        source_db: "OMIM".to_string(),
                        source_id: "114500".to_string(),
                    },
                    DiseaseXref {
                        source_db: "ORPHA".to_string(),
                        source_id: "68335".to_string(),
                    },
                ],
                mondo_release: "test-2026".to_string(),
            },
            DiseaseTerm {
                mondo_id: "MONDO:0000999".to_string(),
                mondo_accession: 999,
                name: "obsolete disease".to_string(),
                definition: None,
                is_obsolete: true,
                comment: None,
                omim_id: None,
                orphanet_id: None,
                synonyms: vec![],
                xrefs: vec![],
                mondo_release: "test-2026".to_string(),
            },
        ],
        relationships: vec![DiseaseRelationship {
            subject_mondo_id: "MONDO:0004992".to_string(),
            object_mondo_id: "MONDO:0000001".to_string(),
            relationship_type: DiseaseRelationType::IsA,
            mondo_release: "test-2026".to_string(),
        }],
    }
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_mondo_storage_e2e() {
    let pg = common::TestPostgres::start().await.expect("start postgres");
    let org_id = common::create_test_org(&pg.pool, "mondo-test")
        .await
        .expect("create org");

    let parsed = sample_mondo();
    let storage = MondoStorage::new(pg.pool.clone(), org_id);
    storage
        .store_release("test-2026", &parsed)
        .await
        .expect("store_release");

    // disease_terms: 3 rows
    let term_count = common::count_rows(&pg.pool, "disease_terms").await.unwrap();
    assert_eq!(term_count, 3, "expected 3 disease_terms");

    // disease_term_synonyms: 3 total (1 + 2 + 0)
    let syn_count = common::count_rows(&pg.pool, "disease_term_synonyms")
        .await
        .unwrap();
    assert_eq!(syn_count, 3, "expected 3 synonyms");

    // disease_term_xrefs: 3 total (1 + 2 + 0)
    let xref_count = common::count_rows(&pg.pool, "disease_term_xrefs")
        .await
        .unwrap();
    assert_eq!(xref_count, 3, "expected 3 xrefs");

    // disease_relationships: 1 row
    let rel_count = common::count_rows(&pg.pool, "disease_relationships")
        .await
        .unwrap();
    assert_eq!(rel_count, 1, "expected 1 relationship");

    // Spot-check cancer term
    let omim_id: Option<String> =
        sqlx::query_scalar("SELECT omim_id FROM disease_terms WHERE mondo_id = 'MONDO:0004992'")
            .fetch_one(&pg.pool)
            .await
            .expect("fetch cancer term");
    assert_eq!(
        omim_id.as_deref(),
        Some("114500"),
        "cancer should have OMIM 114500"
    );

    // Obsolete term should be stored
    let obsolete_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM disease_terms WHERE is_obsolete = true")
            .fetch_one(&pg.pool)
            .await
            .expect("count obsolete terms");
    assert_eq!(obsolete_count, 1, "expected 1 obsolete term");

    // Idempotent: second run should not duplicate
    storage
        .store_release("test-2026", &parsed)
        .await
        .expect("second store");
    let term_count2 = common::count_rows(&pg.pool, "disease_terms").await.unwrap();
    assert_eq!(term_count2, 3, "idempotent: still 3 terms");
}
