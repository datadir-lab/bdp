//! NCBI Taxonomy parser unit tests

use bdp_server::ingest::ncbi_taxonomy::parser::TaxdumpParser;

#[test]
fn test_parse_rankedlineage_human() {
    let parser = TaxdumpParser::new();

    let line = "9606\t|\tHomo sapiens\t|\tHomo sapiens\t|\tHomo\t|\tHominidae\t|\tPrimates\t|\tMammalia\t|\tChordata\t|\tMetazoa\t|\tEukaryota\t|";

    let result = parser.parse_rankedlineage_line(line, 1);
    if let Err(e) = &result {
        eprintln!("Parse error: {}", e);
    }
    assert!(result.is_ok(), "Parse failed: {:?}", result.err());

    let entry_opt = result.unwrap();
    assert!(entry_opt.is_some(), "Entry was None");

    let entry = entry_opt.unwrap();
    assert_eq!(entry.taxonomy_id, 9606);
    assert_eq!(entry.scientific_name, "Homo sapiens");
    assert_eq!(entry.common_name, Some("Homo sapiens".to_string()));
    assert_eq!(entry.rank, "species");
    assert!(entry.lineage.contains("Eukaryota"));
    assert!(entry.lineage.contains("Homo sapiens"));
    assert!(entry.lineage.contains("Mammalia"));
}

#[test]
fn test_parse_rankedlineage_mouse() {
    let parser = TaxdumpParser::new();

    let line = "10090\t|\tMus musculus\t|\tMus musculus\t|\tMus\t|\tMuridae\t|\tRodentia\t|\tMammalia\t|\tChordata\t|\tMetazoa\t|\tEukaryota\t|";

    let result = parser.parse_rankedlineage_line(line, 1);
    assert!(result.is_ok());

    let entry = result.unwrap().unwrap();
    assert_eq!(entry.taxonomy_id, 10090);
    assert_eq!(entry.scientific_name, "Mus musculus");
    assert_eq!(entry.rank, "species");
}

#[test]
fn test_parse_merged_line() {
    let parser = TaxdumpParser::new();

    let line = "123\t|\t456\t|";

    let result = parser.parse_merged_line(line, 1);
    assert!(result.is_ok());

    let merged = result.unwrap();
    assert_eq!(merged.old_taxonomy_id, 123);
    assert_eq!(merged.new_taxonomy_id, 456);
}

#[test]
fn test_parse_delnodes_line() {
    let parser = TaxdumpParser::new();

    let line = "789\t|";

    let result = parser.parse_delnodes_line(line, 1);
    assert!(result.is_ok());

    let deleted = result.unwrap();
    assert_eq!(deleted.taxonomy_id, 789);
}

#[test]
fn test_parse_with_limit() {
    let parser = TaxdumpParser::with_limit(2);

    let rankedlineage = "9606\t|\tHomo sapiens\t|\tHomo sapiens\t|\tHomo\t|\tHominidae\t|\tPrimates\t|\tMammalia\t|\tChordata\t|\tMetazoa\t|\tEukaryota\t|\n\
                        10090\t|\tMus musculus\t|\tMus musculus\t|\tMus\t|\tMuridae\t|\tRodentia\t|\tMammalia\t|\tChordata\t|\tMetazoa\t|\tEukaryota\t|\n\
                        7227\t|\tDrosophila melanogaster\t|\tDrosophila melanogaster\t|\tDrosophila\t|\tDrosophilidae\t|\tDiptera\t|\tInsecta\t|\tArthropoda\t|\tMetazoa\t|\tEukaryota\t|";

    let result = parser.parse_rankedlineage(rankedlineage);
    assert!(result.is_ok());

    let entries = result.unwrap();
    assert_eq!(entries.len(), 2); // Should stop at limit
}

#[test]
fn test_parse_full_taxdump() {
    let parser = TaxdumpParser::new();

    let rankedlineage = "9606\t|\tHomo sapiens\t|\tHomo sapiens\t|\tHomo\t|\tHominidae\t|\tPrimates\t|\tMammalia\t|\tChordata\t|\tMetazoa\t|\tEukaryota\t|";
    let merged = "123\t|\t456\t|";
    let delnodes = "789\t|";

    let result = parser.parse(rankedlineage, merged, delnodes, "2026-01-15".to_string());
    assert!(result.is_ok());

    let taxdump = result.unwrap();
    assert_eq!(taxdump.entries.len(), 1);
    assert_eq!(taxdump.merged.len(), 1);
    assert_eq!(taxdump.deleted.len(), 1);
    assert_eq!(taxdump.external_version, "2026-01-15");
}

#[test]
fn test_taxdump_data_get_entry() {
    let parser = TaxdumpParser::new();

    let rankedlineage = "9606\t|\tHomo sapiens\t|\tHomo sapiens\t|\tHomo\t|\tHominidae\t|\tPrimates\t|\tMammalia\t|\tChordata\t|\tMetazoa\t|\tEukaryota\t|";
    let merged = "";
    let delnodes = "";

    let taxdump = parser.parse(rankedlineage, merged, delnodes, "2026-01-15".to_string()).unwrap();

    assert!(taxdump.get_entry(9606).is_some());
    assert!(taxdump.get_entry(9999).is_none());
}

#[test]
fn test_taxdump_data_is_merged() {
    let parser = TaxdumpParser::new();

    let rankedlineage = "";
    let merged = "123\t|\t456\t|";
    let delnodes = "";

    let taxdump = parser.parse(rankedlineage, merged, delnodes, "2026-01-15".to_string()).unwrap();

    assert_eq!(taxdump.is_merged(123), Some(456));
    assert_eq!(taxdump.is_merged(456), None);
}

#[test]
fn test_taxdump_data_is_deleted() {
    let parser = TaxdumpParser::new();

    let rankedlineage = "";
    let merged = "";
    let delnodes = "789\t|";

    let taxdump = parser.parse(rankedlineage, merged, delnodes, "2026-01-15".to_string()).unwrap();

    assert!(taxdump.is_deleted(789));
    assert!(!taxdump.is_deleted(123));
}

#[test]
fn test_taxonomy_entry_to_json() {
    use bdp_server::ingest::ncbi_taxonomy::models::TaxonomyEntry;

    let entry = TaxonomyEntry::new(
        9606,
        "Homo sapiens".to_string(),
        Some("human".to_string()),
        "species".to_string(),
        "Eukaryota;Metazoa;Chordata;Mammalia;Primates;Hominidae;Homo;Homo sapiens".to_string(),
    );

    let json = entry.to_json();
    assert!(json.is_ok());

    let json_str = json.unwrap();
    assert!(json_str.contains("\"taxonomy_id\": 9606"));
    assert!(json_str.contains("\"scientific_name\": \"Homo sapiens\""));
    assert!(json_str.contains("\"common_name\": \"human\""));
}

#[test]
fn test_taxonomy_entry_to_tsv() {
    use bdp_server::ingest::ncbi_taxonomy::models::TaxonomyEntry;

    let entry = TaxonomyEntry::new(
        9606,
        "Homo sapiens".to_string(),
        Some("human".to_string()),
        "species".to_string(),
        "Eukaryota;Metazoa;Chordata;Mammalia;Primates;Hominidae;Homo;Homo sapiens".to_string(),
    );

    let tsv = entry.to_tsv();
    assert_eq!(
        tsv,
        "9606\tHomo sapiens\thuman\tspecies\tEukaryota;Metazoa;Chordata;Mammalia;Primates;Hominidae;Homo;Homo sapiens"
    );
}

#[test]
fn test_taxonomy_entry_validate() {
    use bdp_server::ingest::ncbi_taxonomy::models::TaxonomyEntry;

    // Valid entry
    let entry = TaxonomyEntry::new(
        9606,
        "Homo sapiens".to_string(),
        Some("human".to_string()),
        "species".to_string(),
        "Eukaryota;Metazoa;Chordata;Mammalia;Primates;Hominidae;Homo;Homo sapiens".to_string(),
    );
    assert!(entry.validate().is_ok());

    // Invalid taxonomy ID
    let entry = TaxonomyEntry::new(
        -1,
        "Homo sapiens".to_string(),
        None,
        "species".to_string(),
        "Eukaryota".to_string(),
    );
    assert!(entry.validate().is_err());

    // Empty scientific name
    let entry = TaxonomyEntry::new(
        9606,
        String::new(),
        None,
        "species".to_string(),
        "Eukaryota".to_string(),
    );
    assert!(entry.validate().is_err());
}
