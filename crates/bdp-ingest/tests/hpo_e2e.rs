mod common;

use bdp_ingest::pipelines::hpo::{
    models::{DiseaseAnnotation, HpoRelationship, HpoSynonym, HpoTerm, SynonymScope},
    storage::HpoStorage,
};

fn sample_terms(release: &str) -> Vec<HpoTerm> {
    vec![
        HpoTerm {
            hpo_id: "HP:0000001".to_string(),
            hpo_accession: 1,
            name: "All".to_string(),
            definition: Some("Root of all HPO terms.".to_string()),
            comment: None,
            is_obsolete: false,
            replaced_by: None,
            synonyms: vec![],
            xrefs: vec![],
            alt_ids: vec![],
            subset: vec![],
            hpo_release_version: release.to_string(),
        },
        HpoTerm {
            hpo_id: "HP:0000118".to_string(),
            hpo_accession: 118,
            name: "Phenotypic abnormality".to_string(),
            definition: Some("A phenotypic abnormality.".to_string()),
            comment: None,
            is_obsolete: false,
            replaced_by: None,
            synonyms: vec![HpoSynonym {
                scope: SynonymScope::Exact,
                text: "Organ abnormality".to_string(),
            }],
            xrefs: vec!["UMLS:C4021819".to_string()],
            alt_ids: vec![],
            subset: vec!["hposlim_core".to_string()],
            hpo_release_version: release.to_string(),
        },
        HpoTerm {
            hpo_id: "HP:0000924".to_string(),
            hpo_accession: 924,
            name: "Abnormality of the skeletal system".to_string(),
            definition: None,
            comment: None,
            is_obsolete: false,
            replaced_by: None,
            synonyms: vec![],
            xrefs: vec![],
            alt_ids: vec!["HP:0003011".to_string()],
            subset: vec![],
            hpo_release_version: release.to_string(),
        },
    ]
}

fn sample_relationships(release: &str) -> Vec<HpoRelationship> {
    vec![
        HpoRelationship {
            subject_hpo_id: "HP:0000118".to_string(),
            object_hpo_id: "HP:0000001".to_string(),
            relationship_type: "is_a".to_string(),
            hpo_release_version: release.to_string(),
        },
        HpoRelationship {
            subject_hpo_id: "HP:0000924".to_string(),
            object_hpo_id: "HP:0000118".to_string(),
            relationship_type: "is_a".to_string(),
            hpo_release_version: release.to_string(),
        },
    ]
}

fn sample_annotations(release: &str) -> Vec<DiseaseAnnotation> {
    vec![
        DiseaseAnnotation {
            disease_db: "OMIM".to_string(),
            disease_id: "114500".to_string(),
            disease_name: "Breast-ovarian cancer".to_string(),
            hpo_id: "HP:0003002".to_string(),
            qualifier: None,
            reference: Some("OMIM:114500".to_string()),
            evidence: Some("PCS".to_string()),
            onset: None,
            frequency: Some("HP:0040280".to_string()),
            sex: None,
            modifier: None,
            aspect: Some("P".to_string()),
            biocuration: Some("HPO:probinson[2009-02-17]".to_string()),
            hpo_release_version: release.to_string(),
        },
        DiseaseAnnotation {
            disease_db: "ORPHA".to_string(),
            disease_id: "68335".to_string(),
            disease_name: "Neonatal diabetes".to_string(),
            hpo_id: "HP:0000819".to_string(),
            qualifier: Some("NOT".to_string()),
            reference: Some("PMID:12345678".to_string()),
            evidence: Some("IEA".to_string()),
            onset: None,
            frequency: None,
            sex: None,
            modifier: None,
            aspect: Some("P".to_string()),
            biocuration: None,
            hpo_release_version: release.to_string(),
        },
    ]
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_hpo_storage_e2e() {
    let release = "test-2026-03-01";
    let pg = common::TestPostgres::start().await.expect("start postgres");
    let org_id = common::create_test_org(&pg.pool, "hpo-test")
        .await
        .expect("create org");

    let terms = sample_terms(release);
    let relationships = sample_relationships(release);
    let annotations = sample_annotations(release);

    let storage = HpoStorage::new(pg.pool.clone(), org_id);

    // Store ontology (terms + relationships)
    let stats = storage
        .store_ontology(&terms, &relationships, release, "1.0")
        .await
        .expect("store_ontology");
    assert_eq!(stats.terms_stored, 3, "store_ontology should report 3 terms");
    assert_eq!(stats.relationships_stored, 2, "store_ontology should report 2 relationships");

    // hpo_term_metadata: 3 rows
    let term_count = common::count_rows(&pg.pool, "hpo_term_metadata")
        .await
        .unwrap();
    assert_eq!(term_count, 3, "expected 3 hpo terms");

    // hpo_relationships: 2 rows
    let rel_count = common::count_rows(&pg.pool, "hpo_relationships")
        .await
        .unwrap();
    assert_eq!(rel_count, 2, "expected 2 hpo relationships");

    // Spot-check term name
    let name: String =
        sqlx::query_scalar("SELECT name FROM hpo_term_metadata WHERE hpo_id = 'HP:0000118'")
            .fetch_one(&pg.pool)
            .await
            .expect("fetch phenotypic abnormality");
    assert_eq!(name, "Phenotypic abnormality");

    // Store annotations
    let stored = storage
        .store_annotations(&annotations, release)
        .await
        .expect("store_annotations");
    assert_eq!(stored, 2, "should have stored 2 annotations");

    // disease_phenotype_annotations: 2 rows
    let ann_count = common::count_rows(&pg.pool, "disease_phenotype_annotations")
        .await
        .unwrap();
    assert_eq!(ann_count, 2, "expected 2 disease_phenotype_annotations");

    // Spot-check NOT qualifier
    let qualifier: Option<String> = sqlx::query_scalar(
        "SELECT qualifier FROM disease_phenotype_annotations WHERE disease_db = 'ORPHA'",
    )
    .fetch_one(&pg.pool)
    .await
    .expect("fetch ORPHA annotation");
    assert_eq!(qualifier.as_deref(), Some("NOT"));

    // Idempotent: second store should not duplicate
    storage
        .store_ontology(&terms, &relationships, release, "1.0")
        .await
        .expect("second ontology");
    let term_count2 = common::count_rows(&pg.pool, "hpo_term_metadata")
        .await
        .unwrap();
    assert_eq!(term_count2, 3, "idempotent: still 3 terms");

    storage
        .store_annotations(&annotations, release)
        .await
        .expect("second annotations");
    let ann_count2 = common::count_rows(&pg.pool, "disease_phenotype_annotations")
        .await
        .unwrap();
    assert_eq!(ann_count2, 2, "idempotent: still 2 annotations");
}
