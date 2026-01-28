//! End-to-end tests for bdp query command
//!
//! These tests validate the full query workflow including:
//! - Entity alias resolution
//! - SQL building from flags
//! - Raw SQL execution
//! - Output formats (table, json, csv, tsv, compact)
//! - Error handling
//! - Dry run mode

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::json;
use wiremock::{
    matchers::{body_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

/// Helper to create a mock query response
fn mock_query_response() -> serde_json::Value {
    json!({
        "success": true,
        "data": {
            "columns": ["id", "name", "version"],
            "rows": [
                ["123e4567-e89b-12d3-a456-426614174000", "UniProt Human Proteome", "2024.1"],
                ["223e4567-e89b-12d3-a456-426614174001", "E. coli Genome", "1.0"]
            ]
        }
    })
}

/// Helper to create a mock empty response
fn empty_query_response() -> serde_json::Value {
    json!({
        "success": true,
        "data": {
            "columns": [],
            "rows": []
        }
    })
}

/// Helper to create a mock error response
fn error_query_response(message: &str) -> serde_json::Value {
    json!({
        "success": false,
        "error": message
    })
}

// ============================================================================
// Raw SQL Tests
// ============================================================================

#[tokio::test]
async fn test_query_raw_sql() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .and(body_json(json!({
            "sql": "SELECT id, name, version FROM data_sources LIMIT 10"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_query_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("--sql")
        .arg("SELECT id, name, version FROM data_sources LIMIT 10")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("id"))
        .stdout(predicate::str::contains("name"))
        .stdout(predicate::str::contains("version"));
}

#[tokio::test]
async fn test_query_dry_run() {
    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--select")
        .arg("id,name,version")
        .arg("--where")
        .arg("organism=human")
        .arg("--limit")
        .arg("10")
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("SELECT"))
        .stdout(predicate::str::contains("FROM"))
        .stdout(predicate::str::contains("protein_metadata"));
}

// ============================================================================
// Entity Alias Tests
// ============================================================================

#[tokio::test]
async fn test_query_protein_alias() {
    let mock_server = MockServer::start().await;

    // Should transform to SELECT with protein metadata join
    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_query_response()))
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--limit")
        .arg("5")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert().success();
}

#[tokio::test]
async fn test_query_gene_alias() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_query_response()))
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("gene")
        .arg("--limit")
        .arg("5")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert().success();
}

#[tokio::test]
async fn test_query_genome_alias() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_query_response()))
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("genome")
        .arg("--limit")
        .arg("5")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert().success();
}

// ============================================================================
// Output Format Tests
// ============================================================================

#[tokio::test]
async fn test_query_table_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_query_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--format")
        .arg("table")
        .arg("--limit")
        .arg("5")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("│")) // Table borders
        .stdout(predicate::str::contains("id"))
        .stdout(predicate::str::contains("UniProt Human Proteome"));
}

#[tokio::test]
async fn test_query_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_query_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--format")
        .arg("json")
        .arg("--limit")
        .arg("5")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::is_match(r#"\[.*\]"#).unwrap()) // JSON array
        .stdout(predicate::str::contains("UniProt Human Proteome"));
}

#[tokio::test]
async fn test_query_csv_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_query_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--format")
        .arg("csv")
        .arg("--limit")
        .arg("5")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("id,name,version"))
        .stdout(predicate::str::contains("UniProt Human Proteome"));
}

#[tokio::test]
async fn test_query_tsv_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_query_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--format")
        .arg("tsv")
        .arg("--limit")
        .arg("5")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("id\tname\tversion"))
        .stdout(predicate::str::contains("UniProt Human Proteome"));
}

#[tokio::test]
async fn test_query_compact_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_query_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--format")
        .arg("compact")
        .arg("--limit")
        .arg("5")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("UniProt Human Proteome"))
        .stdout(predicate::str::contains("E. coli Genome"));
}

#[tokio::test]
async fn test_query_no_header() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_query_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--format")
        .arg("csv")
        .arg("--no-header")
        .arg("--limit")
        .arg("5")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("id,name,version").not())
        .stdout(predicate::str::contains("UniProt Human Proteome"));
}

// ============================================================================
// Query Builder Tests
// ============================================================================

#[tokio::test]
async fn test_query_with_select() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_query_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--select")
        .arg("id,name,version")
        .arg("--limit")
        .arg("10")
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("SELECT id,name,version"));
}

#[tokio::test]
async fn test_query_with_where_simple() {
    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--where")
        .arg("organism=human")
        .arg("--limit")
        .arg("10")
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("WHERE"))
        .stdout(predicate::str::contains("organism"));
}

#[tokio::test]
async fn test_query_with_where_multiple() {
    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--where")
        .arg("organism=human")
        .arg("--where")
        .arg("status=published")
        .arg("--limit")
        .arg("10")
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("WHERE"))
        .stdout(predicate::str::contains("AND"));
}

#[tokio::test]
async fn test_query_with_order_by() {
    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--order-by")
        .arg("name:asc")
        .arg("--limit")
        .arg("10")
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ORDER BY"))
        .stdout(predicate::str::contains("ASC"));
}

#[tokio::test]
async fn test_query_with_limit() {
    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--limit")
        .arg("50")
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("LIMIT 50"));
}

#[tokio::test]
async fn test_query_with_offset() {
    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--limit")
        .arg("10")
        .arg("--offset")
        .arg("20")
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("OFFSET 20"));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_query_missing_entity_and_sql() {
    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("entity"))
        .stderr(predicate::str::contains("sql"));
}

#[tokio::test]
async fn test_query_invalid_entity() {
    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("invalid_entity_name")
        .arg("--limit")
        .arg("10");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Unknown entity"));
}

#[tokio::test]
async fn test_query_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(error_query_response("DROP statements are not allowed")),
        )
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("--sql")
        .arg("DROP TABLE data_sources")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[tokio::test]
async fn test_query_server_unavailable() {
    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--limit")
        .arg("10")
        .arg("--server-url")
        .arg("http://localhost:9999"); // Non-existent server

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

// ============================================================================
// File Output Tests
// ============================================================================

#[tokio::test]
async fn test_query_output_to_file() {
    use std::fs;
    use tempfile::TempDir;

    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_query_response()))
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.csv");

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--format")
        .arg("csv")
        .arg("--output")
        .arg(&output_file)
        .arg("--limit")
        .arg("5")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert().success();

    // Verify file was created and contains data
    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("id,name,version"));
    assert!(content.contains("UniProt Human Proteome"));
}

// ============================================================================
// Empty Results Tests
// ============================================================================

#[tokio::test]
async fn test_query_empty_results() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_query_response()))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--where")
        .arg("organism=nonexistent")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No results"));
}

// ============================================================================
// EXPLAIN Tests
// ============================================================================

#[tokio::test]
async fn test_query_explain() {
    let mock_server = MockServer::start().await;

    let explain_response = json!({
        "success": true,
        "data": {
            "columns": ["QUERY PLAN"],
            "rows": [
                ["Seq Scan on data_sources  (cost=0.00..35.50 rows=2550 width=32)"]
            ]
        }
    });

    Mock::given(method("POST"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(explain_response))
        .mount(&mock_server)
        .await;

    let mut cmd = Command::cargo_bin("bdp").unwrap();
    cmd.arg("query")
        .arg("protein")
        .arg("--explain")
        .arg("--limit")
        .arg("10")
        .arg("--server-url")
        .arg(&mock_server.uri());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("EXPLAIN"));
}
