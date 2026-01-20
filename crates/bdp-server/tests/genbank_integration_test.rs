// GenBank integration tests
//
// These tests verify the complete ingestion pipeline with real data

use bdp_server::ingest::genbank::{
    config::GenbankFtpConfig,
    models::{Division, SourceDatabase},
    parser::GenbankParser,
};
use std::fs;
use std::path::PathBuf;

// Helper to get the correct fixture path
fn get_fixture_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../tests/fixtures/genbank/sample.gbk");
    path
}

#[test]
fn test_parse_sample_file_complete() {
    // This test uses the sample GenBank file fixture
    let sample_path = get_fixture_path();
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let records = parser
        .parse_all(data.as_bytes())
        .expect("Failed to parse");

    assert_eq!(records.len(), 1, "Should parse exactly 1 record");

    let record = &records[0];

    // Verify all critical fields
    assert_eq!(record.accession, "NC_001416");
    assert_eq!(record.accession_version, "NC_001416.1");
    assert_eq!(record.sequence_length, 5386);
    assert_eq!(record.molecule_type, "DNA");
    assert_eq!(record.organism, Some("Enterobacteria phage lambda".to_string()));
    assert_eq!(record.taxonomy_id, Some(10710));

    // Verify CDS features
    assert_eq!(record.cds_features.len(), 2, "Should have 2 CDS features");

    let cds1 = &record.cds_features[0];
    assert_eq!(cds1.gene, Some("cI".to_string()));
    assert_eq!(cds1.protein_id, Some("NP_040606.1".to_string()));
    assert_eq!(cds1.start, Some(190));
    assert_eq!(cds1.end, Some(255));

    // Verify sequence
    assert!(!record.sequence.is_empty(), "Sequence should not be empty");
    assert_eq!(record.sequence.len(), 1140, "Sequence length should match ORIGIN section");

    // Verify calculated fields
    assert!(record.gc_content > 0.0 && record.gc_content <= 100.0, "GC content should be valid percentage");
    assert_eq!(record.sequence_hash.len(), 64, "SHA256 hash should be 64 hex chars");

    // Verify FASTA generation
    let fasta = record.to_fasta();
    assert!(fasta.starts_with(">NC_001416.1"));
    assert!(fasta.contains("Enterobacteria phage lambda"));
    assert!(fasta.contains(&record.sequence[..60])); // First line of sequence
}

#[test]
fn test_parse_with_limit() {
    let sample_path = get_fixture_path();
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    let parser = GenbankParser::new(SourceDatabase::Genbank);

    // Test with limit
    let records = parser.parse_with_limit(data.as_bytes(), 1).expect("Failed to parse");
    assert_eq!(records.len(), 1, "Should respect limit");

    // Test with higher limit than available
    let records = parser.parse_with_limit(data.as_bytes(), 100).expect("Failed to parse");
    assert_eq!(records.len(), 1, "Should return all available records when limit is higher");
}

#[test]
fn test_extraction_methods() {
    let sample_path = get_fixture_path();
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let records = parser.parse_all(data.as_bytes()).expect("Failed to parse");
    let record = &records[0];

    // Test all extraction methods
    assert_eq!(record.extract_gene_name(), Some("cI".to_string()));
    assert_eq!(record.extract_locus_tag(), Some("LAMBDA_00001".to_string()));
    assert_eq!(record.extract_protein_id(), Some("NP_040606.1".to_string()));
    assert_eq!(record.extract_product(), Some("lambda repressor CI".to_string()));
    assert_eq!(record.extract_taxonomy_id(), Some(10710));
}

#[test]
fn test_s3_key_generation() {
    let sample_path = get_fixture_path();
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let records = parser.parse_all(data.as_bytes()).expect("Failed to parse");
    let record = &records[0];

    // Test S3 key format
    let s3_key = record.generate_s3_key("259");
    assert!(s3_key.starts_with("genbank/release-259/"));
    assert!(s3_key.ends_with("/NC_001416.1.fasta"));
    assert!(s3_key.contains("phage") || s3_key.contains("unknown")); // Division
}

#[test]
fn test_fasta_format() {
    let sample_path = get_fixture_path();
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let records = parser.parse_all(data.as_bytes()).expect("Failed to parse");
    let record = &records[0];

    let fasta = record.to_fasta();
    let lines: Vec<&str> = fasta.lines().collect();

    // Check header
    assert!(lines[0].starts_with(">"), "First line should be FASTA header");
    assert!(lines[0].contains("NC_001416.1"), "Header should contain accession");

    // Check sequence lines (max 60 chars)
    for line in &lines[1..] {
        assert!(line.len() <= 60, "Sequence lines should be max 60 chars");
        assert!(line.chars().all(|c| "ACGT".contains(c)), "Should only contain ACGT");
    }
}

#[test]
fn test_config_builder_pattern() {
    let config = GenbankFtpConfig::new()
        .with_genbank()
        .with_parse_limit(1000)
        .with_batch_size(100)
        .with_concurrency(8)
        .with_timeout(600);

    assert_eq!(config.source_database, SourceDatabase::Genbank);
    assert_eq!(config.parse_limit, Some(1000));
    assert_eq!(config.batch_size, 100);
    assert_eq!(config.concurrency, 8);
    assert_eq!(config.timeout_seconds, 600);
}

#[test]
fn test_division_file_patterns() {
    let config = GenbankFtpConfig::new();

    assert_eq!(config.get_division_file_pattern(&Division::Viral), "gbvrl*.seq.gz");
    assert_eq!(config.get_division_file_pattern(&Division::Bacterial), "gbbct*.seq.gz");
    assert_eq!(config.get_division_file_pattern(&Division::Phage), "gbphg*.seq.gz");
    assert_eq!(config.get_division_file_pattern(&Division::Plant), "gbpln*.seq.gz");
}

#[test]
fn test_genbank_vs_refseq_paths() {
    let genbank_config = GenbankFtpConfig::new().with_genbank();
    assert_eq!(genbank_config.get_base_path(), "/genbank");
    assert_eq!(genbank_config.get_release_number_path(), "/genbank/GB_Release_Number");

    let refseq_config = GenbankFtpConfig::new().with_refseq();
    assert_eq!(refseq_config.get_base_path(), "/refseq/release");
    assert_eq!(refseq_config.get_release_number_path(), "/refseq/release/RELEASE_NUMBER");
}

#[test]
fn test_all_divisions_available() {
    let all_divs = GenbankFtpConfig::get_all_divisions();
    assert!(all_divs.len() >= 9, "Should have at least 9 primary divisions");

    assert!(all_divs.contains(&Division::Viral));
    assert!(all_divs.contains(&Division::Bacterial));
    assert!(all_divs.contains(&Division::Phage));
    assert!(all_divs.contains(&Division::Plant));
    assert!(all_divs.contains(&Division::Mammalian));
}

#[test]
fn test_test_division_is_phage() {
    let test_div = GenbankFtpConfig::get_test_division();
    assert_eq!(test_div, Division::Phage, "Test division should be phage (smallest)");
}

// Performance characteristic tests

#[test]
fn test_parser_performance_characteristics() {
    let sample_path = get_fixture_path();
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    let parser = GenbankParser::new(SourceDatabase::Genbank);

    // Measure parse time
    let start = std::time::Instant::now();
    let _records = parser.parse_all(data.as_bytes()).expect("Failed to parse");
    let duration = start.elapsed();

    // Should parse sample file very quickly (<10ms)
    assert!(duration.as_millis() < 100, "Parsing should be fast for small files");
}

#[test]
fn test_gc_content_calculation_accuracy() {
    let parser = GenbankParser::new(SourceDatabase::Genbank);

    // Test with known sequences
    let test_cases = vec![
        ("AAAA", 0.0),    // 0% GC
        ("GGCC", 100.0),  // 100% GC
        ("ATGC", 50.0),   // 50% GC
        ("AAATGC", 33.333), // 33.33% GC (2/6)
    ];

    for (seq, expected_gc) in test_cases {
        let calculated = bdp_server::ingest::genbank::parser::GenbankParser::calculate_gc_content(seq);
        assert!((calculated - expected_gc).abs() < 0.01,
            "GC content for {} should be ~{}, got {}", seq, expected_gc, calculated);
    }
}

#[test]
fn test_hash_deterministic() {
    let sample_path = get_fixture_path();
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    let parser = GenbankParser::new(SourceDatabase::Genbank);

    // Parse same data twice
    let records1 = parser.parse_all(data.as_bytes()).expect("Failed to parse");
    let records2 = parser.parse_all(data.as_bytes()).expect("Failed to parse");

    // Hashes should be identical
    assert_eq!(records1[0].sequence_hash, records2[0].sequence_hash,
        "Hash should be deterministic");
}

#[test]
fn test_different_sequences_different_hashes() {
    // Create two different "records" with different sequences
    use bdp_server::ingest::genbank::parser::GenbankParser;

    let hash1 = GenbankParser::calculate_hash("ATGC");
    let hash2 = GenbankParser::calculate_hash("CGTA");

    assert_ne!(hash1, hash2, "Different sequences should have different hashes");
    assert_eq!(hash1.len(), 64, "SHA256 should produce 64 hex characters");
    assert_eq!(hash2.len(), 64, "SHA256 should produce 64 hex characters");
}

#[cfg(test)]
mod models_tests {
    use super::*;

    #[test]
    fn test_source_database_as_str() {
        assert_eq!(SourceDatabase::Genbank.as_str(), "genbank");
        assert_eq!(SourceDatabase::Refseq.as_str(), "refseq");
    }

    #[test]
    fn test_division_as_str() {
        assert_eq!(Division::Viral.as_str(), "viral");
        assert_eq!(Division::Bacterial.as_str(), "bacterial");
        assert_eq!(Division::Phage.as_str(), "phage");
    }

    #[test]
    fn test_division_file_prefix() {
        assert_eq!(Division::Viral.file_prefix(), "gbvrl");
        assert_eq!(Division::Bacterial.file_prefix(), "gbbct");
        assert_eq!(Division::Phage.file_prefix(), "gbphg");
    }
}
