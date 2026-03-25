// crates/bdp-ingest/tests/obo_integration.rs

use bdp_ingest::common::obo::OboParser;

/// Download and parse the real GO OBO file.
/// Run with: cargo test -p bdp-ingest --test obo_integration -- --ignored --nocapture
#[tokio::test]
#[ignore = "downloads ~50MB from internet"]
async fn test_parse_real_go_obo() {
    let url = "https://purl.obolibrary.org/obo/go/go-basic.obo";
    let content = bdp_ingest::common::http::download_text(url, 3)
        .await
        .expect("failed to download GO OBO");

    let terms = OboParser::parse(&content, None)
        .expect("failed to parse GO OBO");

    // GO has ~45,000 terms
    assert!(terms.len() > 40_000, "expected >40k terms, got {}", terms.len());

    // Spot check a known stable term
    let bp_root = terms.iter().find(|t| t.id == "GO:0008150");
    assert!(bp_root.is_some(), "biological_process root term not found");
    let bp = bp_root.unwrap();
    assert_eq!(bp.name, "biological_process");
    assert_eq!(bp.namespace.as_deref(), Some("biological_process"));

    // Count non-obsolete terms
    let active = terms.iter().filter(|t| !t.is_obsolete).count();
    assert!(active > 38_000, "expected >38k active terms, got {}", active);

    println!("Parsed {} total terms, {} active", terms.len(), active);
}

/// Parse a MONDO OBO slice (uses same format as GO)
#[tokio::test]
#[ignore = "downloads from internet"]
async fn test_parse_real_mondo_obo() {
    let url = "https://purl.obolibrary.org/obo/mondo.obo";
    let content = bdp_ingest::common::http::download_text(url, 3)
        .await
        .expect("failed to download MONDO OBO");

    let terms = OboParser::parse(&content, Some(1000))
        .expect("failed to parse MONDO OBO");

    // Just check we can parse 1000 terms without errors
    assert_eq!(terms.len(), 1000);
    // All terms should have non-empty IDs
    let terms_with_ids: Vec<_> = terms.iter().filter(|t| !t.id.is_empty()).collect();
    assert_eq!(terms_with_ids.len(), 1000, "all terms should have IDs");
    // MONDO OBO contains terms from multiple ontologies (BFO, CHEBI, MONDO etc.)
    // Just verify we have valid ontology IDs with colon notation
    let valid_ids: Vec<_> = terms.iter().filter(|t| t.id.contains(':')).collect();
    assert!(!valid_ids.is_empty(), "expected ontology IDs with colon notation");
    println!("Parsed {} MONDO OBO terms (first 1000)", terms.len());
    println!("Sample IDs: {:?}", terms.iter().take(5).map(|t| &t.id).collect::<Vec<_>>());
}

#[tokio::test]
#[ignore = "downloads from reactome.org"]
async fn test_parse_reactome_pathways() {
    use bdp_ingest::pipelines::reactome::parser;

    let url = "https://reactome.org/download/current/ReactomePathways.txt";
    let content = bdp_ingest::common::http::download_text(url, 3)
        .await
        .expect("failed to download ReactomePathways.txt");
    let pathways = parser::parse_pathways(&content, "114").expect("failed to parse pathways");

    assert!(pathways.len() > 20_000, "expected >20K pathways, got {}", pathways.len());

    let human = pathways.iter().filter(|p| p.species_name == "Homo sapiens").count();
    assert!(human > 2_000, "expected >2K human pathways, got {}", human);

    println!("Reactome: {} total pathways, {} human", pathways.len(), human);
}

#[tokio::test]
#[ignore = "downloads ~100MB from reactome.org"]
async fn test_parse_reactome_uniprot_human() {
    use bdp_ingest::pipelines::reactome::parser;

    let url = "https://reactome.org/download/current/UniProt2Reactome.txt";
    let content = bdp_ingest::common::http::download_text(url, 3)
        .await
        .expect("failed to download UniProt2Reactome.txt");
    let links =
        parser::parse_uniprot_reactome(&content, "114", Some("Homo sapiens")).expect("failed to parse links");

    assert!(links.len() > 100_000, "expected >100K human links, got {}", links.len());

    // TP53 (P04637) should map to many pathways
    let tp53_links: Vec<_> = links.iter().filter(|l| l.uniprot_acc == "P04637").collect();
    assert!(!tp53_links.is_empty(), "P04637 (TP53) should map to pathways");

    println!(
        "Reactome: {} human protein->pathway links, {} TP53 pathways",
        links.len(),
        tp53_links.len()
    );
}
