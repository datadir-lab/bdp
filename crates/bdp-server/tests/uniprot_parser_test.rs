//! Integration tests for UniProt DAT parser

use std::path::PathBuf;
use bdp_server::ingest::uniprot::{models::UniProtEntry, parser::DatParser};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn test_parse_sample_dat_file() {
    let parser = DatParser::new();
    let path = fixture_path().join("uniprot_sample_10.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    assert_eq!(entries.len(), 10);

    // Check first entry
    let first = &entries[0];
    assert_eq!(first.accession, "P15711");
    assert_eq!(first.entry_name, "104K_THEPA");
    assert_eq!(first.protein_name, "104 kDa microneme-rhoptry antigen");
    assert_eq!(first.gene_name, Some("TA02".to_string()));
    assert_eq!(first.organism_name, "Theileria parva");
    assert_eq!(first.taxonomy_id, 5875);
    assert_eq!(first.sequence_length, 104);
    assert_eq!(first.mass_da, 11937);
}

#[test]
fn test_parse_gzipped_dat_file() {
    let parser = DatParser::new();
    let path = fixture_path().join("uniprot_sample_10.dat.gz");

    let entries = parser.parse_file(&path).expect("Failed to parse gzipped DAT file");

    assert_eq!(entries.len(), 10);

    // Verify we got the same data
    let first = &entries[0];
    assert_eq!(first.accession, "P15711");
}

#[test]
fn test_parse_with_limit() {
    let parser = DatParser::with_limit(3);
    let path = fixture_path().join("uniprot_sample_10.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    assert_eq!(entries.len(), 3);
}

#[test]
fn test_all_entries_have_required_fields() {
    let parser = DatParser::new();
    let path = fixture_path().join("uniprot_sample_10.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    for entry in &entries {
        assert!(!entry.accession.is_empty(), "Accession is empty");
        assert!(!entry.entry_name.is_empty(), "Entry name is empty");
        assert!(!entry.protein_name.is_empty(), "Protein name is empty");
        assert!(!entry.organism_name.is_empty(), "Organism name is empty");
        assert!(entry.taxonomy_id > 0, "Taxonomy ID is invalid");
        assert!(!entry.sequence.is_empty(), "Sequence is empty");
        assert!(entry.sequence_length > 0, "Sequence length is invalid");
        assert!(entry.mass_da > 0, "Mass is invalid");
    }
}

#[test]
fn test_specific_entries() {
    let parser = DatParser::new();
    let path = fixture_path().join("uniprot_sample_10.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    // Test human 14-3-3 beta/alpha
    let human_143b = entries.iter().find(|e| e.accession == "P31946").unwrap();
    assert_eq!(human_143b.entry_name, "143B_HUMAN");
    assert_eq!(human_143b.protein_name, "14-3-3 protein beta/alpha");
    assert_eq!(human_143b.gene_name, Some("YWHAB".to_string()));
    assert_eq!(human_143b.organism_name, "Homo sapiens (Human)");
    assert_eq!(human_143b.taxonomy_id, 9606);
    assert_eq!(human_143b.sequence_length, 245);

    // Test another human entry
    let human_143e = entries.iter().find(|e| e.accession == "P62258").unwrap();
    assert_eq!(human_143e.entry_name, "143E_HUMAN");
    assert_eq!(human_143e.organism_name, "Homo sapiens (Human)");
    assert_eq!(human_143e.taxonomy_id, 9606);
}

#[test]
fn test_entries_without_gene_names() {
    let parser = DatParser::new();
    let path = fixture_path().join("uniprot_sample_10.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    // 10KD_VIGUN doesn't have a gene name
    let no_gene = entries.iter().find(|e| e.accession == "P18646").unwrap();
    assert_eq!(no_gene.entry_name, "10KD_VIGUN");
    assert_eq!(no_gene.gene_name, None);
}

#[test]
fn test_sequence_content() {
    let parser = DatParser::new();
    let path = fixture_path().join("uniprot_sample_10.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    let entry = &entries[0];

    // Sequence should not contain whitespace
    assert!(!entry.sequence.contains(' '));
    assert!(!entry.sequence.contains('\n'));

    // Sequence length should match actual length
    assert_eq!(entry.sequence.len(), entry.sequence_length as usize);

    // Sequence should contain only valid amino acids
    for ch in entry.sequence.chars() {
        assert!(ch.is_ascii_uppercase(), "Invalid character in sequence: {}", ch);
    }
}

#[test]
fn test_fasta_generation() {
    let parser = DatParser::new();
    let path = fixture_path().join("uniprot_sample_10.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    for entry in &entries {
        let fasta = entry.to_fasta();

        // FASTA should start with '>'
        assert!(fasta.starts_with('>'));

        // Should contain accession and entry name
        assert!(fasta.contains(&entry.accession));
        assert!(fasta.contains(&entry.entry_name));

        // Should contain organism info
        assert!(fasta.contains("OS="));
        assert!(fasta.contains("OX="));

        // Should contain the sequence
        assert!(fasta.contains(&entry.sequence));
    }
}

#[test]
fn test_json_generation() {
    let parser = DatParser::new();
    let path = fixture_path().join("uniprot_sample_10.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    for entry in &entries {
        let json = entry.to_json().expect("Failed to generate JSON");

        // JSON should contain key fields
        assert!(json.contains(&entry.accession));
        assert!(json.contains(&entry.entry_name));
        assert!(json.contains(&entry.protein_name));
        assert!(json.contains(&entry.organism_name));

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("Generated JSON is not valid");

        assert_eq!(parsed["accession"], entry.accession);
        assert_eq!(parsed["taxonomy_id"], entry.taxonomy_id);
    }
}

#[test]
fn test_sequence_checksum() {
    let parser = DatParser::new();
    let path = fixture_path().join("uniprot_sample_10.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    for entry in &entries {
        let checksum = entry.sequence_checksum();

        // SHA-256 should be 64 hex characters
        assert_eq!(checksum.len(), 64);

        // Should be consistent
        assert_eq!(checksum, entry.sequence_checksum());

        // All characters should be hex
        for ch in checksum.chars() {
            assert!(ch.is_ascii_hexdigit());
        }
    }
}

#[test]
fn test_entry_validation() {
    let parser = DatParser::new();
    let path = fixture_path().join("uniprot_sample_10.dat");

    let entries = parser.parse_file(&path).expect("Failed to parse DAT file");

    for entry in &entries {
        entry.validate().expect("Entry validation failed");
    }
}

#[test]
fn test_parse_bytes() {
    let parser = DatParser::new();

    // Read the file as bytes
    let path = fixture_path().join("uniprot_sample_10.dat");
    let data = std::fs::read(&path).expect("Failed to read file");

    let entries = parser.parse_bytes(&data).expect("Failed to parse bytes");

    assert_eq!(entries.len(), 10);
}

#[test]
fn test_parse_gzipped_bytes() {
    let parser = DatParser::new();

    // Read the gzipped file as bytes
    let path = fixture_path().join("uniprot_sample_10.dat.gz");
    let data = std::fs::read(&path).expect("Failed to read file");

    let entries = parser.parse_bytes(&data).expect("Failed to parse gzipped bytes");

    assert_eq!(entries.len(), 10);
}
