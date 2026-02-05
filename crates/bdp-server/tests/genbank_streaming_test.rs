#![allow(clippy::unwrap_used, clippy::expect_used)]
// GenBank streaming decompression integration tests

use bdp_server::ingest::genbank::config::GenbankFtpConfig;
use bdp_server::ingest::genbank::models::SourceDatabase;
use bdp_server::ingest::genbank::parser::GenbankParser;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::io::{Cursor, Write};

#[test]
fn test_streaming_with_real_genbank_sample() {
    // Read sample GenBank file
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    // Compress it
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data.as_bytes())
        .expect("Failed to compress");
    let compressed = encoder.finish().expect("Failed to finish compression");

    println!("Original size: {} bytes", data.len());
    println!("Compressed size: {} bytes", compressed.len());
    println!(
        "Compression ratio: {:.2}%",
        (compressed.len() as f64 / data.len() as f64) * 100.0
    );

    // Create streaming decoder
    let cursor = Cursor::new(compressed);
    let decoder = flate2::read::GzDecoder::new(cursor);

    // Parse using streaming reader
    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let records = parser
        .parse_all(decoder)
        .expect("Failed to parse with streaming");

    // Verify results match non-streaming approach
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].accession, "NC_001416");
    assert_eq!(records[0].sequence_length, 5386);

    println!("✓ Streaming decompression works with real GenBank data");
    println!("  Parsed {} records", records.len());
}

#[test]
fn test_streaming_memory_usage_simulation() {
    // Simulate a large GenBank file by repeating the sample
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let sample_data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    // Create a "large" file by repeating the sample 100 times
    let mut large_data = String::new();
    for i in 0..100 {
        // Modify accession to avoid duplicates
        let modified = sample_data.replace("NC_001416", &format!("NC_{:06}", i));
        large_data.push_str(&modified);
    }

    println!(
        "Simulated large file size: {} bytes (~{} MB)",
        large_data.len(),
        large_data.len() / 1_048_576
    );

    // Compress it
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(large_data.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    println!(
        "Compressed size: {} bytes (~{} MB)",
        compressed.len(),
        compressed.len() / 1_048_576
    );

    // Parse with streaming (this should use minimal memory)
    let cursor = Cursor::new(compressed);
    let decoder = flate2::read::GzDecoder::new(cursor);

    let parser = GenbankParser::new(SourceDatabase::Genbank);

    // Parse with limit to avoid long test runtime
    let records = parser
        .parse_with_limit(decoder, 10)
        .expect("Failed to parse");

    assert_eq!(records.len(), 10);
    println!("✓ Successfully parsed large file with streaming (limited to 10 records)");
}

#[test]
fn test_streaming_vs_nonstreaming_correctness() {
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    // Compress
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    let parser = GenbankParser::new(SourceDatabase::Genbank);

    // Method 1: Non-streaming (decompress to Vec<u8> first)
    let cursor1 = Cursor::new(compressed.clone());
    let mut decoder1 = flate2::read::GzDecoder::new(cursor1);
    let mut decompressed = Vec::new();
    std::io::Read::read_to_end(&mut decoder1, &mut decompressed).unwrap();
    let records1 = parser.parse_all(decompressed.as_slice()).unwrap();

    // Method 2: Streaming (parse directly from decoder)
    let cursor2 = Cursor::new(compressed);
    let decoder2 = flate2::read::GzDecoder::new(cursor2);
    let records2 = parser.parse_all(decoder2).unwrap();

    // Both methods should produce identical results
    assert_eq!(records1.len(), records2.len());
    assert_eq!(records1[0].accession, records2[0].accession);
    assert_eq!(records1[0].sequence, records2[0].sequence);
    assert_eq!(records1[0].sequence_hash, records2[0].sequence_hash);

    println!("✓ Streaming and non-streaming produce identical results");
}

#[test]
fn test_streaming_with_parse_limit() {
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    // Create data with multiple records
    let mut multi_record = String::new();
    for i in 0..5 {
        let modified = data.replace("NC_001416", &format!("NC_00141{}", i));
        multi_record.push_str(&modified);
    }

    // Compress
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(multi_record.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    // Parse with limit
    let cursor = Cursor::new(compressed);
    let decoder = flate2::read::GzDecoder::new(cursor);

    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let records = parser.parse_with_limit(decoder, 3).unwrap();

    assert_eq!(records.len(), 3);
    println!("✓ Parse limit works correctly with streaming");
}

#[test]
fn test_streaming_error_handling() {
    // Test with corrupted gzip data
    let corrupted_data = b"This is not valid gzip data at all!";

    let cursor = Cursor::new(corrupted_data.to_vec());
    let decoder = flate2::read::GzDecoder::new(cursor);

    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let result = parser.parse_all(decoder);

    // Should error gracefully
    assert!(result.is_err());
    println!("✓ Streaming handles corrupted data correctly");
}

#[test]
fn test_streaming_with_multiple_files_simulation() {
    // Simulate processing multiple files sequentially
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let mut total_records = 0;

    // Simulate 3 files
    for file_num in 0..3 {
        // Compress data
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        // Stream and parse
        let cursor = Cursor::new(compressed);
        let decoder = flate2::read::GzDecoder::new(cursor);
        let records = parser.parse_all(decoder).unwrap();

        total_records += records.len();
        println!("File {}: Parsed {} records", file_num + 1, records.len());
    }

    assert_eq!(total_records, 3); // 1 record per file × 3 files
    println!("✓ Sequential streaming of multiple files works correctly");
}

#[test]
fn test_streaming_compression_ratios() {
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    // Test different compression levels
    let levels = vec![Compression::fast(), Compression::default(), Compression::best()];

    for (idx, level) in levels.iter().enumerate() {
        let mut encoder = GzEncoder::new(Vec::new(), *level);
        encoder.write_all(data.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        let ratio = (compressed.len() as f64 / data.len() as f64) * 100.0;
        println!(
            "Compression level {}: {} -> {} bytes ({:.2}%)",
            idx,
            data.len(),
            compressed.len(),
            ratio
        );

        // Verify decompression works
        let cursor = Cursor::new(compressed);
        let decoder = flate2::read::GzDecoder::new(cursor);
        let parser = GenbankParser::new(SourceDatabase::Genbank);
        let records = parser.parse_all(decoder).unwrap();

        assert_eq!(records.len(), 1);
    }

    println!("✓ All compression levels decompress correctly");
}
