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

/// Parse real MONDO OBO (full, no limit) and verify counts.
/// Run: cargo test -p bdp-ingest --test obo_integration test_parse_full_mondo -- --ignored --nocapture
#[tokio::test]
#[ignore = "downloads ~50MB from internet"]
async fn test_parse_full_mondo() {
    let url = bdp_ingest::pipelines::mondo::MONDO_OBO_URL;
    let content = bdp_ingest::common::http::download_text(url, 3)
        .await
        .expect("download MONDO");

    let parsed = bdp_ingest::pipelines::mondo::parser::parse_obo(&content, "test", None)
        .expect("parse MONDO");

    // MONDO has ~27K MONDO-prefixed terms
    assert!(
        parsed.term_count() > 20_000,
        "expected >20K MONDO terms, got {}",
        parsed.term_count()
    );

    // Should have many relationships
    assert!(
        parsed.relationship_count() > 15_000,
        "expected >15K relationships, got {}",
        parsed.relationship_count()
    );

    // Spot check: cancer term (MONDO:0004992)
    let cancer = parsed.terms.iter().find(|t| t.mondo_id == "MONDO:0004992");
    assert!(cancer.is_some(), "MONDO:0004992 (cancer) not found");
    let cancer = cancer.unwrap();
    assert_eq!(cancer.name, "cancer");
    // Verify the term has xrefs (OMIM or others — MONDO releases may vary)
    assert!(
        !cancer.xrefs.is_empty() || cancer.omim_id.is_some(),
        "cancer should have at least some xrefs"
    );

    println!(
        "MONDO: {} terms, {} relationships, {} obsolete",
        parsed.term_count(),
        parsed.relationship_count(),
        parsed.terms.iter().filter(|t| t.is_obsolete).count()
    );
}

/// Parse the first 1000 ChEBI terms from the live OBO file.
/// Run with: cargo test -p bdp-ingest --test obo_integration test_parse_chebi_sample -- --ignored --nocapture
#[tokio::test]
#[ignore = "downloads ~280MB from EBI FTP"]
async fn test_parse_chebi_sample() {
    use bdp_ingest::pipelines::chebi::parser;

    // Parse only first 1000 terms to keep test fast
    let url = "https://ftp.ebi.ac.uk/pub/databases/chebi/ontology/chebi.obo";
    let content = bdp_ingest::common::http::download_text(url, 3)
        .await
        .expect("failed to download ChEBI OBO");
    let parsed = parser::parse_obo(&content, "test", Some(1000))
        .expect("failed to parse ChEBI OBO");

    assert_eq!(parsed.terms.len(), 1000, "expected 1000 ChEBI terms");

    // Check that InChIKey extraction works on real data
    let with_inchikey: Vec<_> = parsed.terms.iter().filter(|t| t.inchikey.is_some()).collect();
    assert!(!with_inchikey.is_empty(), "expected some terms with InChIKey");

    println!(
        "ChEBI sample: {} terms, {} with InChIKey, {} rels",
        parsed.terms.len(),
        with_inchikey.len(),
        parsed.relationships.len()
    );
}

/// Parse real HPO OBO (full, no limit) and verify counts.
/// Run: cargo test -p bdp-ingest --test obo_integration test_parse_full_hpo -- --ignored --nocapture
#[tokio::test]
#[ignore = "downloads ~7MB from github.com"]
async fn test_parse_full_hpo() {
    use bdp_ingest::pipelines::hpo::parser::HpoParser;

    let url = bdp_ingest::pipelines::hpo::HPO_OBO_URL;
    let content = bdp_ingest::common::http::download_text(url, 3)
        .await
        .expect("download HPO OBO");

    let parsed = HpoParser::parse_obo(&content, "test", None).expect("parse HPO OBO");

    // HPO has ~17K terms
    assert!(
        parsed.terms.len() > 15_000,
        "expected >15K HPO terms, got {}",
        parsed.terms.len()
    );

    // HPO has many is_a relationships
    assert!(
        parsed.relationships.len() > 15_000,
        "expected >15K HPO relationships, got {}",
        parsed.relationships.len()
    );

    // Spot check: HP:0000001 is the root term "All"
    let root = parsed.terms.iter().find(|t| t.hpo_id == "HP:0000001");
    assert!(root.is_some(), "HP:0000001 (root) not found");
    assert_eq!(root.unwrap().name, "All");

    // Count non-obsolete terms
    let active = parsed.terms.iter().filter(|t| !t.is_obsolete).count();
    assert!(active > 14_000, "expected >14K active HPO terms, got {}", active);

    println!(
        "HPO: {} terms ({} active), {} relationships",
        parsed.terms.len(),
        active,
        parsed.relationships.len()
    );
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

    // UniProt2Reactome_All_Levels.txt includes all hierarchy levels — larger dataset
    let url = "https://reactome.org/download/current/UniProt2Reactome_All_Levels.txt";
    let content = bdp_ingest::common::http::download_text(url, 3)
        .await
        .expect("failed to download UniProt2Reactome_All_Levels.txt");
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
