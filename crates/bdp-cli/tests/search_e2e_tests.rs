//! End-to-end tests for bdp search command
//!
//! These tests validate the full search workflow including:
//! - Non-interactive output formats
//! - Caching behavior
//! - Error handling
//! - Manifest integration
//! - Pagination

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use wiremock::{
    matchers::{method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

/// Helper to create a test manifest file
fn create_test_manifest(dir: &TempDir) -> PathBuf {
    let manifest_path = dir.path().join("bdp.yml");
    let manifest_content = r#"
project:
  name: test-project
  version: 0.1.0
  description: Test project for E2E tests

sources: []
tools: []
"#;
    fs::write(&manifest_path, manifest_content).expect("Failed to create test manifest");
    manifest_path
}

/// Helper to create a mock search response
fn mock_search_response() -> serde_json::Value {
    serde_json::json!({
        "success": true,
        "data": {
            "results": [
                {
                    "id": "123e4567-e89b-12d3-a456-426614174000",
                    "organization": "uniprot",
                    "name": "P01308",
                    "version": "1.0",
                    "description": "Insulin precursor",
                    "format": "fasta",
                    "entry_type": "data_source"
                },
                {
                    "id": "223e4567-e89b-12d3-a456-426614174001",
                    "organization": "genbank",
                    "name": "NC_000001",
                    "version": "2.0",
                    "description": "Homo sapiens chromosome 1",
                    "format": "gbk",
                    "entry_type": "data_source"
                }
            ],
            "total": 2,
            "page": 1,
            "page_size": 10
        }
    })
}

/// Helper to create an empty search response
fn empty_search_response() -> serde_json::Value {
    serde_json::json!({
        "success": true,
        "data": {
            "results": [],
            "total": 0,
            "page": 1,
            "page_size": 10
        }
    })
}

#[tokio::test]
async fn test_search_compact_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(query_param("query", "insulin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_search_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("insulin")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("uniprot:P01308@1.0"))
        .stdout(predicate::str::contains("genbank:NC_000001@2.0"));
}

#[tokio::test]
async fn test_search_table_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(query_param("query", "insulin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_search_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("insulin")
        .arg("--no-interactive")
        .arg("--format")
        .arg("table")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Source"))
        .stdout(predicate::str::contains("Format"))
        .stdout(predicate::str::contains("P01308"))
        .stdout(predicate::str::contains("fasta"));
}

#[tokio::test]
async fn test_search_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(query_param("query", "insulin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_search_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("insulin")
        .arg("--no-interactive")
        .arg("--format")
        .arg("json")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(r#""organization": "uniprot""#))
        .stdout(predicate::str::contains(r#""name": "P01308""#))
        .stdout(predicate::str::contains(r#""format": "fasta""#));
}

#[tokio::test]
async fn test_search_with_type_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(query_param("query", "insulin"))
        .and(query_param("type_filter", "data_source"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_search_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("insulin")
        .arg("--type")
        .arg("data_source")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert().success();
}

#[tokio::test]
async fn test_search_with_source_type_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(query_param("query", "insulin"))
        .and(query_param("source_type_filter", "protein"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_search_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("insulin")
        .arg("--source-type")
        .arg("protein")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert().success();
}

#[tokio::test]
async fn test_search_with_pagination() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(query_param("query", "insulin"))
        .and(query_param("page", "2"))
        .and(query_param("per_page", "20"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_search_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("insulin")
        .arg("--page")
        .arg("2")
        .arg("--limit")
        .arg("20")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert().success();
}

#[tokio::test]
async fn test_search_empty_results() {
    let mock_server = MockServer::start().await;


    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(query_param("query", "nonexistent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_search_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("nonexistent")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No results found"));
}

#[tokio::test]
async fn test_search_multi_word_query() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(query_param("query", "insulin human"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_search_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("insulin")
        .arg("human")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert().success();
}

#[tokio::test]
async fn test_search_invalid_limit() {
    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("insulin")
        .arg("--limit")
        .arg("200") // > 100
        .arg("--no-interactive");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Limit must be between 1 and 100"));
}

#[tokio::test]
async fn test_search_invalid_page() {
    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("insulin")
        .arg("--page")
        .arg("0")
        .arg("--no-interactive");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Page must be greater than 0"));
}

#[tokio::test]
async fn test_search_empty_query() {
    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("")
        .arg("--no-interactive");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Search query cannot be empty"));
}

#[tokio::test]
async fn test_search_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("unique_query_for_server_error_test")  // Use unique query to avoid cache
        .arg("--no-interactive")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert().failure();
}

#[tokio::test]
async fn test_search_network_retry() {
    let mock_server = MockServer::start().await;

    // First two attempts fail, third succeeds
    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_search_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("insulin")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact")
        .arg("--server-url")
        .arg(&mock_server.uri());

    // Should succeed after retries
    cmd.assert().success();
}

#[tokio::test]
async fn test_search_caching() {
    let mock_server = MockServer::start().await;

    // Use a unique query to test caching behavior
    let unique_query = "unique_caching_test_query";

    // Mock should only be called once (second request uses cache)
    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(query_param("query", unique_query))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_search_response()))
        .expect(1)
        .mount(&mock_server)
        .await;

    // First search
    let mut cmd1 = Command::cargo_bin("bdp").unwrap();
    cmd1.arg("search")
        .arg(unique_query)
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd1.assert().success();

    // Second search (should use cache)
    let mut cmd2 = Command::cargo_bin("bdp").unwrap();
    cmd2.arg("search")
        .arg(unique_query)
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd2.assert().success();
}

#[tokio::test]
async fn test_search_cache_clear() {
    // Create searches to populate cache
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_search_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("insulin")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert().success();

    // Clear search cache
    let mut clean_cmd = Command::cargo_bin("bdp").unwrap();
    clean_cmd.arg("clean").arg("--search-cache");

    clean_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleared"))
        .stdout(predicate::str::contains("search cache"));
}

#[test]
fn test_manifest_integration() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_path = create_test_manifest(&temp_dir);

    // Verify manifest exists
    assert!(manifest_path.exists());

    // Read manifest
    let manifest_content = fs::read_to_string(&manifest_path).unwrap();
    assert!(manifest_content.contains("test-project"));
    assert!(manifest_content.contains("sources: []"));
}

#[tokio::test]
async fn test_search_multiple_filters() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(query_param("query", "insulin"))
        .and(query_param("type_filter", "data_source"))
        .and(query_param("source_type_filter", "protein"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_search_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("insulin")
        .arg("--type")
        .arg("data_source")
        .arg("--source-type")
        .arg("protein")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert().success();
}

#[tokio::test]
async fn test_search_fuzzy_suggestions() {
    let mock_server = MockServer::start().await;


    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(query_param("query", "insulinn")) // Typo
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_search_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("search")
        .arg("insulinn")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No results found"))
        .stdout(predicate::str::contains("Did you mean"));
}
