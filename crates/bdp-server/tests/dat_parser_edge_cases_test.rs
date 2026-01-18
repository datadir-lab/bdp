//! Edge case and error handling tests for UniProt DAT parser

use std::path::PathBuf;
use bdp_server::ingest::uniprot::parser::DatParser;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("uniprot")
}

// ============================================================================
// EDGE CASE TESTS (5 tests)
// ============================================================================

#[test]
fn test_parse_special_chars_in_protein_name() {
    let parser = DatParser::new();
    let path = fixture_path().join("edge_cases.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    // Find entry with special characters in protein name
    let spec1 = entries.iter().find(|e| e.accession == "P00001").unwrap();
    assert_eq!(
        spec1.protein_name,
        "Protein with (parentheses) and [brackets] - test"
    );

    // Find entry with EC classification
    let spec2 = entries.iter().find(|e| e.accession == "P00002").unwrap();
    // The parser should extract just "Enzyme with EC:1.2.3.4 classification"
    // (removing the {ECO:...} flag)
    assert_eq!(
        spec2.protein_name,
        "Enzyme with EC:1.2.3.4 classification"
    );
}

#[test]
fn test_parse_multiple_accessions() {
    let parser = DatParser::new();
    let path = fixture_path().join("edge_cases.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    // Entry with "AC   P00003; P00004; P00005;"
    // Should only take the first accession (P00003)
    let multi = entries.iter().find(|e| e.accession == "P00003").unwrap();
    assert_eq!(multi.accession, "P00003");
    assert_eq!(multi.entry_name, "MULTI_MOUSE");

    // Verify we don't have entries for the secondary accessions
    assert!(entries.iter().all(|e| e.accession != "P00004"));
    assert!(entries.iter().all(|e| e.accession != "P00005"));
}

#[test]
fn test_parse_multi_line_organism_name() {
    let parser = DatParser::new();
    let path = fixture_path().join("edge_cases.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    // Entry with multi-line organism: "OS   Rattus norvegicus\nOS   (Rat)."
    let multi_org = entries.iter().find(|e| e.accession == "P00006").unwrap();

    // Should merge lines: "Rattus norvegicus (Rat)"
    assert_eq!(multi_org.organism_name, "Rattus norvegicus (Rat)");
}

#[test]
fn test_parse_long_sequence() {
    let parser = DatParser::new();
    let path = fixture_path().join("edge_cases.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    // Entry with 500 AA sequence
    let long_seq = entries.iter().find(|e| e.accession == "P00007").unwrap();

    assert_eq!(long_seq.sequence_length, 500);
    assert_eq!(long_seq.sequence.len(), 500);

    // Verify sequence contains no whitespace
    assert!(!long_seq.sequence.contains(' '));
    assert!(!long_seq.sequence.contains('\n'));

    // Verify sequence contains only valid amino acids
    for ch in long_seq.sequence.chars() {
        assert!(ch.is_ascii_uppercase(), "Invalid character: {}", ch);
    }
}

#[test]
fn test_parse_empty_sequence() {
    let parser = DatParser::new();
    let path = fixture_path().join("malformed.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    // Entry with "SQ   SEQUENCE   0 AA" should be skipped (returns None)
    // Verify we don't have an entry with accession P00023
    assert!(entries.iter().all(|e| e.accession != "P00023"));
}

// ============================================================================
// ERROR HANDLING TESTS (5 tests)
// ============================================================================

#[test]
fn test_parse_missing_accession() {
    let parser = DatParser::new();
    let path = fixture_path().join("malformed.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    // Entry without AC line should be skipped gracefully
    // The entry has ID NOACC_TEST but no accession
    // Verify no entry exists with entry_name "NOACC_TEST"
    assert!(entries.iter().all(|e| e.entry_name != "NOACC_TEST"));
}

#[test]
fn test_parse_missing_taxonomy_id() {
    let parser = DatParser::new();
    let path = fixture_path().join("malformed.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    // Entry without OX line (P00020) should be skipped
    assert!(entries.iter().all(|e| e.accession != "P00020"));
}

#[test]
fn test_parse_invalid_taxonomy_id() {
    let parser = DatParser::new();
    let path = fixture_path().join("invalid_taxonomy.dat");

    // Entry with "OX   NCBI_TaxID=invalid;" should cause a parse error
    // The parser should return an error when trying to parse the invalid taxonomy ID
    let result = parser.parse_file(&path);

    // Should error due to invalid integer parse
    assert!(result.is_err());
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(err_msg.contains("taxonomy") || err_msg.contains("invalid digit"));
}

#[test]
fn test_parse_malformed_sequence_line() {
    let parser = DatParser::new();
    let path = fixture_path().join("malformed_sequence.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    // Entry P00022 has sequence with numbers: "MKTAYIAK12 34QISFVKSH..."
    // The parser currently includes these in the sequence
    // This test verifies the current behavior - the parser just takes what's there
    if let Some(bad_seq) = entries.iter().find(|e| e.accession == "P00022") {
        // The sequence will contain the numbers since parser doesn't filter
        // In a real UniProt file, this wouldn't happen
        // This test just verifies we don't crash
        assert!(!bad_seq.sequence.is_empty());
    }
}

#[test]
fn test_parse_truncated_file() {
    let parser = DatParser::new();
    let path = fixture_path().join("malformed.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    // Last entry (P00024 TRUNC_TEST) ends without "//" marker
    // Parser should skip this incomplete entry gracefully
    assert!(entries.iter().all(|e| e.accession != "P00024"));
}

// ============================================================================
// COMPREHENSIVE VALIDATION TEST
// ============================================================================

#[test]
fn test_edge_cases_all_valid_entries() {
    let parser = DatParser::new();
    let path = fixture_path().join("edge_cases.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    // Should have 5 valid entries from edge_cases.dat
    assert_eq!(entries.len(), 5);

    // All entries should have valid required fields
    for entry in &entries {
        assert!(!entry.accession.is_empty(), "Accession is empty");
        assert!(!entry.entry_name.is_empty(), "Entry name is empty");
        assert!(!entry.protein_name.is_empty(), "Protein name is empty");
        assert!(!entry.organism_name.is_empty(), "Organism name is empty");
        assert!(entry.taxonomy_id > 0, "Taxonomy ID is invalid");
        assert!(!entry.sequence.is_empty(), "Sequence is empty");
        assert!(entry.sequence_length > 0, "Sequence length is invalid");
        assert!(entry.mass_da > 0, "Mass is invalid");

        // Validate entry
        entry.validate().expect("Entry validation failed");
    }
}

#[test]
fn test_malformed_graceful_handling() {
    let parser = DatParser::new();
    let path = fixture_path().join("malformed.dat");

    // Parser should not panic on malformed file
    let result = parser.parse_file(&path);

    // Should return Ok with skipped malformed entries
    // malformed.dat contains: NOACC (no accession), P00020 (no taxonomy),
    // P00023 (empty sequence), P00024 (truncated)
    let entries = result.expect("Parser should handle malformed gracefully");

    // None of the malformed entries should be in the result
    let malformed_accessions = vec!["P00020", "P00023", "P00024"];
    for acc in malformed_accessions {
        assert!(
            entries.iter().all(|e| e.accession != acc),
            "Malformed entry {} was not skipped",
            acc
        );
    }

    // Also verify the entry without accession is skipped
    assert!(entries.iter().all(|e| e.entry_name != "NOACC_TEST"));
}
