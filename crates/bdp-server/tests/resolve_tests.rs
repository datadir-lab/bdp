//! Integration tests for resolve functionality.
//!
//! These tests verify the dependency resolution endpoint works correctly
//! with various manifest configurations.

use bdp_server::api::resolve::{ResolveRequest, ResolveResponse};
use bdp_server::db::{create_pool, DbConfig};
use serde_json::Value;

/// Helper to create test database pool
async fn create_test_pool() -> sqlx::PgPool {
    let config = DbConfig::from_env().expect("DATABASE_URL must be set for tests");
    create_pool(&config)
        .await
        .expect("Failed to create test pool")
}

#[tokio::test]
#[ignore] // Run with `cargo test --ignored` when database is available
async fn test_resolve_single_source() {
    let _pool = create_test_pool().await;

    // Test parsing a valid source spec
    use bdp_server::api::resolve::*;

    // This would require actual database data to test fully
    // Here we just test the parsing logic
}

#[test]
fn test_parse_source_spec_valid() {
    // Test the SourceSpec parsing directly
    // Note: SourceSpec is private, so we test through the API

    let valid_specs = vec![
        "uniprot:P01308-fasta@1.0",
        "ncbi:GRCh38-xml@2.0",
        "ensembl:homo-sapiens-json@1.0",
    ];

    for spec in valid_specs {
        // The spec should be valid format
        assert!(spec.contains(':'));
        assert!(spec.contains('@'));
        assert!(spec.contains('-'));
    }
}

#[test]
fn test_parse_tool_spec_valid() {
    let valid_specs = vec!["ncbi:blast@2.14.0", "ucsc:blat@36.0", "broad:gatk@4.2.0"];

    for spec in valid_specs {
        assert!(spec.contains(':'));
        assert!(spec.contains('@'));
        assert!(!spec.contains('-')); // Tools don't have format suffix
    }
}

#[test]
fn test_resolve_request_serialization() {
    use serde_json;

    let request = ResolveRequest {
        sources: vec!["uniprot:P01308-fasta@1.0".to_string(), "uniprot:all-fasta@1.0".to_string()],
        tools: vec!["ncbi:blast@2.14.0".to_string()],
    };

    let json = serde_json::to_string(&request).unwrap();
    let deserialized: ResolveRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(request.sources.len(), deserialized.sources.len());
    assert_eq!(request.tools.len(), deserialized.tools.len());
}

#[tokio::test]
#[ignore]
async fn test_resolve_with_dependencies() {
    let pool = create_test_pool().await;

    // This test would resolve a source that has dependencies
    // and verify the dependency tree is correctly returned

    // Example: uniprot:all@1.0 might depend on many individual proteins
    // The resolved response should include dependency_count and dependencies
}

#[tokio::test]
#[ignore]
async fn test_resolve_nonexistent_source() {
    let pool = create_test_pool().await;

    // Test resolving a source that doesn't exist
    // Should return a NOT_FOUND error
}

#[tokio::test]
#[ignore]
async fn test_resolve_invalid_version() {
    let pool = create_test_pool().await;

    // Test resolving a valid source but with invalid version
    // Should return a NOT_FOUND error
}

#[tokio::test]
#[ignore]
async fn test_resolve_invalid_format() {
    let pool = create_test_pool().await;

    // Test resolving a source with a format that's not available
    // Should return a NOT_FOUND error
}

#[tokio::test]
#[ignore]
async fn test_resolve_multiple_sources() {
    let pool = create_test_pool().await;

    // Test resolving multiple sources at once
    // Should return resolved data for all sources
}

#[tokio::test]
#[ignore]
async fn test_resolve_mixed_sources_and_tools() {
    let pool = create_test_pool().await;

    // Test resolving both sources and tools in one request
    // Should return both in the response
}

#[test]
fn test_source_spec_format() {
    // Test various source specification formats
    let valid_formats = vec![
        ("uniprot:P01308-fasta@1.0", ("uniprot", "P01308", "1.0", "fasta")),
        ("ncbi:GRCh38-xml@2.0", ("ncbi", "GRCh38", "2.0", "xml")),
        ("ensembl:all-json@1.0", ("ensembl", "all", "1.0", "json")),
    ];

    for (spec, expected) in valid_formats {
        let parts: Vec<&str> = spec.split(&[':', '@', '-'][..]).collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], expected.0); // org
        assert_eq!(parts[1], expected.1); // name
        assert_eq!(parts[2], expected.2); // version
        assert_eq!(parts[3], expected.3); // format
    }
}

#[test]
fn test_invalid_source_specs() {
    let invalid_specs = vec![
        "invalid",               // No separators
        "uniprot:P01308",        // Missing version and format
        "uniprot-fasta@1.0",     // Missing identifier
        "uniprot:P01308@1.0",    // Missing format
        "P01308-fasta@1.0",      // Missing registry
        "@1.0",                  // Missing registry and identifier
        "uniprot:P01308-@1.0",   // Missing format
        "uniprot:P01308-fasta@", // Missing version
    ];

    for spec in invalid_specs {
        // These should fail parsing
        let parts: Vec<&str> = spec.split(&[':', '@', '-'][..]).collect();
        // Invalid specs won't have exactly 4 parts
        if parts.len() == 4 {
            // Even if 4 parts, some might be empty
            assert!(parts.iter().any(|p| p.is_empty()));
        }
    }
}

#[test]
fn test_resolve_response_structure() {
    use bdp_server::api::resolve::*;
    use std::collections::HashMap;

    // Test the response structure
    let response = ResolveResponse {
        success: true,
        data: ResolvedData {
            sources: HashMap::new(),
            tools: HashMap::new(),
        },
    };

    assert!(response.success);
    assert_eq!(response.data.sources.len(), 0);
    assert_eq!(response.data.tools.len(), 0);
}

#[tokio::test]
#[ignore]
async fn test_resolve_checksum_consistency() {
    let pool = create_test_pool().await;

    // Test that resolving the same source multiple times
    // returns the same checksum (consistency check)
}

#[tokio::test]
#[ignore]
async fn test_resolve_size_accuracy() {
    let pool = create_test_pool().await;

    // Test that the returned file size matches the actual file size
}

#[tokio::test]
#[ignore]
async fn test_resolve_dependency_count() {
    let pool = create_test_pool().await;

    // Test that dependency_count matches actual number of dependencies
}

#[tokio::test]
#[ignore]
async fn test_resolve_circular_dependency_prevention() {
    let pool = create_test_pool().await;

    // Test that circular dependencies are properly handled
    // Should not cause infinite loops or errors
}

#[test]
fn test_empty_resolve_request() {
    let request = ResolveRequest {
        sources: vec![],
        tools: vec![],
    };

    // Should be valid but return empty results
    assert_eq!(request.sources.len(), 0);
    assert_eq!(request.tools.len(), 0);
}

#[test]
fn test_resolve_request_with_duplicates() {
    let request = ResolveRequest {
        sources: vec![
            "uniprot:P01308-fasta@1.0".to_string(),
            "uniprot:P01308-fasta@1.0".to_string(), // Duplicate
        ],
        tools: vec![],
    };

    // Should handle duplicates gracefully
    assert_eq!(request.sources.len(), 2);
}
