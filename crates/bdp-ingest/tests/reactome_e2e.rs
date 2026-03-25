mod common;

use bdp_ingest::pipelines::reactome::{
    models::{Pathway, ProteinPathwayLink},
    storage::ReactomeStorage,
};

fn sample_pathways() -> Vec<Pathway> {
    vec![
        Pathway {
            reactome_id: "R-HSA-109581".to_string(),
            name: "Apoptosis".to_string(),
            species_name: "Homo sapiens".to_string(),
            reactome_release: "114".to_string(),
        },
        Pathway {
            reactome_id: "R-HSA-5633007".to_string(),
            name: "Regulation of TP53 Expression and Degradation".to_string(),
            species_name: "Homo sapiens".to_string(),
            reactome_release: "114".to_string(),
        },
    ]
}

fn sample_links() -> Vec<ProteinPathwayLink> {
    vec![
        ProteinPathwayLink {
            uniprot_acc: "P04637".to_string(), // TP53
            reactome_id: "R-HSA-5633007".to_string(),
            pathway_name: "Regulation of TP53 Expression and Degradation".to_string(),
            evidence_type: Some("IEA".to_string()),
            species_name: "Homo sapiens".to_string(),
            reactome_release: "114".to_string(),
        },
        ProteinPathwayLink {
            uniprot_acc: "P04637".to_string(), // TP53
            reactome_id: "R-HSA-109581".to_string(),
            pathway_name: "Apoptosis".to_string(),
            evidence_type: Some("IEA".to_string()),
            species_name: "Homo sapiens".to_string(),
            reactome_release: "114".to_string(),
        },
    ]
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_reactome_storage_e2e() {
    let pg = common::TestPostgres::start().await.expect("start postgres");
    let org_id = common::create_test_org(&pg.pool, "reactome-test")
        .await
        .expect("create org");

    let pathways = sample_pathways();
    let links = sample_links();

    let storage = ReactomeStorage::new(pg.pool.clone());
    storage
        .ingest_release(org_id, "114", &pathways, &links)
        .await
        .expect("ingest_release");

    // pathway_terms: 2 rows
    let pathway_count = common::count_rows(&pg.pool, "pathway_terms").await.unwrap();
    assert_eq!(pathway_count, 2, "expected 2 pathway_terms");

    // protein_pathway_associations: 2 rows
    let assoc_count = common::count_rows(&pg.pool, "protein_pathway_associations")
        .await
        .unwrap();
    assert_eq!(assoc_count, 2, "expected 2 protein_pathway_associations");

    // TP53 (P04637) should be in both pathways
    let tp53_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM protein_pathway_associations WHERE uniprot_acc = 'P04637'",
    )
    .fetch_one(&pg.pool)
    .await
    .expect("count TP53 associations");
    assert_eq!(tp53_count, 2, "TP53 should map to 2 pathways");

    // Pathway name spot-check
    let apoptosis_name: String =
        sqlx::query_scalar("SELECT name FROM pathway_terms WHERE reactome_id = 'R-HSA-109581'")
            .fetch_one(&pg.pool)
            .await
            .expect("fetch apoptosis pathway");
    assert_eq!(apoptosis_name, "Apoptosis");

    // Idempotent: second ingest should not duplicate
    storage
        .ingest_release(org_id, "114", &pathways, &links)
        .await
        .expect("second ingest");
    let assoc_count2 = common::count_rows(&pg.pool, "protein_pathway_associations")
        .await
        .unwrap();
    assert_eq!(assoc_count2, 2, "idempotent: still 2 associations");
}
