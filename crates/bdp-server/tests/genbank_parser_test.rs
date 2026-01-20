// GenBank parser tests

use bdp_server::ingest::genbank::models::{GenbankRecord, SourceDatabase};
use bdp_server::ingest::genbank::parser::GenbankParser;
use std::fs;

#[test]
fn test_parse_sample_genbank_file() {
    // Read sample GenBank file
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    // Parse with GenBank parser
    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let records = parser
        .parse_all(data.as_bytes())
        .expect("Failed to parse GenBank file");

    // Should have 1 record
    assert_eq!(records.len(), 1);

    let record = &records[0];

    // Verify LOCUS data
    assert_eq!(record.locus_name, "NC_001416");
    assert_eq!(record.sequence_length, 5386);
    assert_eq!(record.molecule_type, "DNA");

    // Verify DEFINITION
    assert!(record
        .definition
        .contains("Enterobacteria phage lambda"));

    // Verify ACCESSION
    assert_eq!(record.accession, "NC_001416");

    // Verify VERSION
    assert_eq!(record.accession_version, "NC_001416.1");
    assert_eq!(record.version_number, Some(1));

    // Verify ORGANISM
    assert_eq!(
        record.organism,
        Some("Enterobacteria phage lambda".to_string())
    );

    // Verify taxonomy ID extraction
    assert_eq!(record.taxonomy_id, Some(10710));

    // Verify CDS features
    assert_eq!(record.cds_features.len(), 2);

    let cds1 = &record.cds_features[0];
    assert_eq!(cds1.gene, Some("cI".to_string()));
    assert_eq!(cds1.locus_tag, Some("LAMBDA_00001".to_string()));
    assert_eq!(cds1.protein_id, Some("NP_040606.1".to_string()));
    assert_eq!(cds1.product, Some("lambda repressor CI".to_string()));
    assert_eq!(cds1.start, Some(190));
    assert_eq!(cds1.end, Some(255));

    let cds2 = &record.cds_features[1];
    assert_eq!(cds2.gene, Some("cro".to_string()));
    assert_eq!(cds2.locus_tag, Some("LAMBDA_00002".to_string()));
    assert_eq!(cds2.protein_id, Some("NP_040607.1".to_string()));
    assert_eq!(cds2.start, Some(300));
    assert_eq!(cds2.end, Some(500));

    // Verify sequence was extracted
    assert!(!record.sequence.is_empty());
    assert_eq!(record.sequence.len(), 1081); // Should match ORIGIN section length

    // Verify GC content calculation
    assert!(record.gc_content > 0.0 && record.gc_content <= 100.0);
    println!("GC content: {:.2}%", record.gc_content);

    // Verify hash was generated
    assert_eq!(record.sequence_hash.len(), 64); // SHA256 produces 64 hex chars

    // Verify FASTA generation
    let fasta = record.to_fasta();
    assert!(fasta.starts_with(">NC_001416.1"));
    assert!(fasta.contains("Enterobacteria phage lambda"));

    println!("✓ Sample GenBank file parsed successfully");
    println!("  Accession: {}", record.accession_version);
    println!("  Length: {} bp", record.sequence_length);
    println!("  GC: {:.1}%", record.gc_content);
    println!("  CDS features: {}", record.cds_features.len());
    println!("  Taxonomy ID: {:?}", record.taxonomy_id);
}

#[test]
fn test_parse_with_limit() {
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let records = parser
        .parse_with_limit(data.as_bytes(), 1)
        .expect("Failed to parse");

    assert_eq!(records.len(), 1);
    println!("✓ Parse with limit works");
}

#[test]
fn test_extract_methods() {
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let records = parser.parse_all(data.as_bytes()).expect("Failed to parse");
    let record = &records[0];

    // Test extraction methods
    assert_eq!(record.extract_gene_name(), Some("cI".to_string()));
    assert_eq!(
        record.extract_locus_tag(),
        Some("LAMBDA_00001".to_string())
    );
    assert_eq!(
        record.extract_protein_id(),
        Some("NP_040606.1".to_string())
    );
    assert_eq!(
        record.extract_product(),
        Some("lambda repressor CI".to_string())
    );

    println!("✓ Extraction methods work");
}

#[test]
fn test_s3_key_generation() {
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let records = parser.parse_all(data.as_bytes()).expect("Failed to parse");
    let record = &records[0];

    let s3_key = record.generate_s3_key("259");

    // Should match pattern: genbank/release-259/phage/NC_001416.1.fasta
    assert!(s3_key.starts_with("genbank/release-259/"));
    assert!(s3_key.ends_with("NC_001416.1.fasta"));

    println!("✓ S3 key generation works: {}", s3_key);
}
