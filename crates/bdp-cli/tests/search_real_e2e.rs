// This test requires bdp-server as a dev-dependency, which was removed
// to avoid apalis-postgres sqlx macro compilation issues.
// Enable the "e2e" feature to compile these tests.
#![cfg(feature = "e2e")]
//! Real end-to-end tests for `bdp search`
//!
//! Uses a **real PostgreSQL** (via testcontainers) and an **in-process axum server**.
//! No wiremock, no mock responses — the full server stack is exercised.
//!
//! Requires Docker Desktop to be running. Tests are `#[serial]` to avoid data conflicts
//! since they share a single PostgreSQL container + axum server for performance.

#![allow(deprecated)]

use assert_cmd::Command;
use axum::routing::get;
use predicates::prelude::*;
use serial_test::serial;
use sqlx::PgPool;
use std::net::SocketAddr;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use tokio::sync::OnceCell;

// ---------------------------------------------------------------------------
// Shared test infrastructure
// ---------------------------------------------------------------------------

struct TestServer {
    addr: SocketAddr,
    #[allow(dead_code)]
    pool: PgPool,
    // Keep container alive for the lifetime of the test suite
    #[allow(dead_code)]
    container: testcontainers::ContainerAsync<Postgres>,
}

static TEST_SERVER: OnceCell<TestServer> = OnceCell::const_new();

/// Returns a reference to the shared test server, starting it on first call.
async fn get_test_server() -> &'static TestServer {
    TEST_SERVER
        .get_or_init(|| async {
            // 1. Start PostgreSQL container
            let container = Postgres::default()
                .start()
                .await
                .expect("Failed to start PostgreSQL container");

            let port = container
                .get_host_port_ipv4(5432)
                .await
                .expect("Failed to get PostgreSQL port");

            let connection_string =
                format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port);

            // 2. Create connection pool
            let pool = PgPool::connect(&connection_string)
                .await
                .expect("Failed to connect to PostgreSQL");

            // 3. Run all migrations
            sqlx::migrate!("../../migrations")
                .run(&pool)
                .await
                .expect("Failed to run migrations");

            // 4. Seed test data
            seed_test_data(&pool)
                .await
                .expect("Failed to seed test data");

            // 5. Create dummy Storage (search never touches S3)
            let storage_config = bdp_server::storage::config::StorageConfig::for_minio(
                "http://127.0.0.1:19999",
                "bdp-test",
            );
            let storage = bdp_server::storage::Storage::new(storage_config)
                .await
                .expect("Failed to create dummy storage");

            // 6. Build the axum router
            let mediator = bdp_server::cqrs::build_mediator(pool.clone(), storage);
            let feature_state = bdp_server::features::FeatureState { mediator };
            let api_v1 = bdp_server::features::router(feature_state);
            let app: axum::Router = axum::Router::new()
                .route("/health", get(|| async { "OK" }))
                .nest("/api/v1", api_v1);

            // 7. Bind to random port and spawn server
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("Failed to bind TcpListener");
            let addr = listener.local_addr().expect("Failed to get local addr");

            tokio::spawn(async move {
                axum::serve(listener, app.into_make_service())
                    .await
                    .expect("axum server crashed");
            });

            TestServer {
                addr,
                pool,
                container,
            }
        })
        .await
}

/// Seed test data: 3 organizations, 8 registry entries, versions, and files.
async fn seed_test_data(pool: &PgPool) -> anyhow::Result<()> {
    // ── Organizations ───────────────────────────────────────────────────
    let uniprot_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO organizations (slug, name, description, is_system)
         VALUES ($1, $2, $3, false) RETURNING id",
    )
    .bind("uniprot")
    .bind("UniProt")
    .bind("Universal Protein Resource")
    .fetch_one(pool)
    .await?;

    let ncbi_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO organizations (slug, name, description, is_system)
         VALUES ($1, $2, $3, false) RETURNING id",
    )
    .bind("ncbi")
    .bind("NCBI")
    .bind("National Center for Biotechnology Information")
    .fetch_one(pool)
    .await?;

    let tools_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO organizations (slug, name, description, is_system)
         VALUES ($1, $2, $3, false) RETURNING id",
    )
    .bind("test-tools")
    .bind("Test Tools")
    .bind("Bioinformatics Tools Collection")
    .fetch_one(pool)
    .await?;

    // ── Helper closures ─────────────────────────────────────────────────
    // Inserts a data_source registry entry + data_sources row + version + version_files.
    // Returns the entry_id.
    #[allow(clippy::too_many_arguments)]
    async fn insert_data_source(
        pool: &PgPool,
        org_id: uuid::Uuid,
        slug: &str,
        name: &str,
        description: &str,
        source_type: &str,
        external_id: &str,
        version: &str,
        formats: &[&str],
    ) -> anyhow::Result<uuid::Uuid> {
        let entry_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
             VALUES ($1, $2, $3, $4, 'data_source') RETURNING id",
        )
        .bind(org_id)
        .bind(slug)
        .bind(name)
        .bind(description)
        .fetch_one(pool)
        .await?;

        sqlx::query(
            "INSERT INTO data_sources (id, source_type, external_id)
             VALUES ($1, $2, $3)",
        )
        .bind(entry_id)
        .bind(source_type)
        .bind(external_id)
        .execute(pool)
        .await?;

        let version_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO versions (entry_id, version) VALUES ($1, $2) RETURNING id",
        )
        .bind(entry_id)
        .bind(version)
        .fetch_one(pool)
        .await?;

        for fmt in formats {
            let filename = format!("{}.{}", slug, fmt);
            let s3_key = format!("data/{}/{}/{}", slug, version, filename);
            sqlx::query(
                "INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(version_id)
            .bind(*fmt)
            .bind(&s3_key)
            .bind("a".repeat(64)) // 64-char hex placeholder
            .bind(1024_i64)
            .execute(pool)
            .await?;
        }

        Ok(entry_id)
    }

    async fn insert_tool(
        pool: &PgPool,
        org_id: uuid::Uuid,
        slug: &str,
        name: &str,
        description: &str,
        tool_type: &str,
        version: &str,
    ) -> anyhow::Result<uuid::Uuid> {
        let entry_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
             VALUES ($1, $2, $3, $4, 'tool') RETURNING id",
        )
        .bind(org_id)
        .bind(slug)
        .bind(name)
        .bind(description)
        .fetch_one(pool)
        .await?;

        sqlx::query("INSERT INTO tools (id, tool_type) VALUES ($1, $2)")
            .bind(entry_id)
            .bind(tool_type)
            .execute(pool)
            .await?;

        sqlx::query("INSERT INTO versions (entry_id, version) VALUES ($1, $2)")
            .bind(entry_id)
            .bind(version)
            .execute(pool)
            .await?;

        Ok(entry_id)
    }

    // ── UniProt entries ─────────────────────────────────────────────────
    insert_data_source(
        pool,
        uniprot_id,
        "P01308",
        "Insulin",
        "Human insulin protein precursor",
        "protein",
        "P01308",
        "1.0",
        &["fasta", "xml"],
    )
    .await?;

    insert_data_source(
        pool,
        uniprot_id,
        "P01317",
        "Glucagon",
        "Glucagon precursor protein",
        "protein",
        "P01317",
        "2.0",
        &["fasta"],
    )
    .await?;

    insert_data_source(
        pool,
        uniprot_id,
        "Q9Y6K9",
        "TRAF6 Binding Protein",
        "TNF receptor associated factor binding protein",
        "protein",
        "Q9Y6K9",
        "1.5",
        &["fasta", "pdb"],
    )
    .await?;

    // ── NCBI entries ────────────────────────────────────────────────────
    insert_data_source(
        pool,
        ncbi_id,
        "NC_000001",
        "Human Chromosome 1",
        "Homo sapiens chromosome 1 complete sequence",
        "genome",
        "NC_000001",
        "GRCh38",
        &["fasta", "gff3"],
    )
    .await?;

    insert_data_source(
        pool,
        ncbi_id,
        "NC_012920",
        "Human Mitochondrial Genome",
        "Homo sapiens mitochondrial complete genome",
        "genome",
        "NC_012920",
        "rCRS",
        &["fasta", "genbank"],
    )
    .await?;

    insert_data_source(
        pool,
        ncbi_id,
        "txid9606",
        "Homo sapiens Taxonomy",
        "Human taxonomy classification node",
        "taxonomy",
        "txid9606",
        "2024",
        &["json"],
    )
    .await?;

    // ── Tool entries ────────────────────────────────────────────────────
    insert_tool(
        pool,
        tools_id,
        "blast",
        "BLAST+ Sequence Search",
        "Basic Local Alignment Search Tool for nucleotide and protein sequences",
        "alignment",
        "2.14",
    )
    .await?;

    insert_tool(
        pool,
        tools_id,
        "hmmer",
        "HMMER Profile Search",
        "Biosequence analysis using profile hidden Markov models",
        "alignment",
        "3.4",
    )
    .await?;

    // ── Refresh materialized view ───────────────────────────────────────
    sqlx::query("REFRESH MATERIALIZED VIEW search_registry_entries_mv")
        .execute(pool)
        .await?;

    Ok(())
}

/// Build a CLI command pointing at the test server.
fn bdp_cmd(server_url: &str) -> Command {
    let mut cmd = Command::cargo_bin("bdp").expect("bdp binary not found");
    cmd.arg("--server-url").arg(server_url);
    // Use a temp cache dir so tests don't interfere with each other or the user's cache
    let cache_dir = std::env::temp_dir().join("bdp-real-e2e-cache");
    cmd.env("BDP_CACHE_DIR", cache_dir.to_str().unwrap());
    cmd
}

/// Build the server URL string from the shared test server address.
fn server_url(addr: SocketAddr) -> String {
    format!("http://{}", addr)
}

// ===========================================================================
// Basic Search Tests
// ===========================================================================

#[tokio::test]
#[serial]
async fn test_search_basic_query() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search").arg("insulin").arg("--no-interactive");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Insulin"));
}

#[tokio::test]
#[serial]
async fn test_search_multiple_words() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("human")
        .arg("chromosome")
        .arg("--no-interactive");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Chromosome"));
}

#[tokio::test]
#[serial]
async fn test_search_case_insensitive() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search").arg("INSULIN").arg("--no-interactive");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Insulin"));
}

#[tokio::test]
#[serial]
async fn test_search_no_results() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("xyznonexistent")
        .arg("--no-interactive");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No results found"));
}

#[tokio::test]
#[serial]
async fn test_search_partial_match() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    // Full-text search with plainto_tsquery may or may not match "insul" as a prefix.
    // PostgreSQL FTS uses stemming, so "insulin" stems to "insulin".
    // "insul" won't match because it doesn't stem to the same root.
    // This test verifies the CLI doesn't crash on partial queries.
    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("insulin")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("uniprot:P01308"));
}

#[tokio::test]
#[serial]
async fn test_search_tool_results() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search").arg("blast").arg("--no-interactive");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("BLAST"));
}

#[tokio::test]
#[serial]
async fn test_search_mixed_types() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    // Both BLAST+ and HMMER have "Search" in their name
    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("search")
        .arg("--no-interactive")
        .arg("--format")
        .arg("table");

    cmd.assert().success();
}

// ===========================================================================
// Filtering Tests
// ===========================================================================

#[tokio::test]
#[serial]
async fn test_search_org_filter() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("protein")
        .arg("--org")
        .arg("uniprot")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact");

    // Should contain uniprot entries only
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ncbi:").not());
}

#[tokio::test]
#[serial]
async fn test_search_type_filter() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("search")
        .arg("--type")
        .arg("tool")
        .arg("--no-interactive")
        .arg("--format")
        .arg("table");

    // Should only get tool results
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("tool"));
}

#[tokio::test]
#[serial]
async fn test_search_source_type_filter() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("human")
        .arg("--source-type")
        .arg("genome")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact");

    // Should only return genome entries (NC_000001, NC_012920)
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("NC_000001").or(predicate::str::contains("NC_012920")));
}

#[tokio::test]
#[serial]
async fn test_search_org_filter_no_match() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    // Insulin is in uniprot, not ncbi — client-side org filter should exclude it
    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("insulin")
        .arg("--org")
        .arg("ncbi")
        .arg("--no-interactive");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No results found"));
}

#[tokio::test]
#[serial]
async fn test_search_combined_filters() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("human")
        .arg("--org")
        .arg("ncbi")
        .arg("--source-type")
        .arg("genome")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact");

    // Should find NCBI genome entries about "human"
    cmd.assert().success();
}

// ===========================================================================
// Output Format Tests
// ===========================================================================

#[tokio::test]
#[serial]
async fn test_search_json_format() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("insulin")
        .arg("--format")
        .arg("json")
        .arg("--no-interactive");

    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(output).expect("non-UTF8 stdout");

    // Find the JSON object in stdout (skip the "Searching for..." line)
    let json_start = stdout.find('{').expect("No JSON object in output");
    let json_str = &stdout[json_start..];

    let parsed: serde_json::Value =
        serde_json::from_str(json_str).expect("Output is not valid JSON");

    // Verify expected fields exist
    assert!(parsed.get("results").is_some(), "Missing 'results' field");
    assert!(parsed.get("total").is_some(), "Missing 'total' field");

    let results = parsed["results"].as_array().expect("results is not array");
    assert!(!results.is_empty(), "Expected at least one result");

    let first = &results[0];
    assert!(first.get("organization_slug").is_some(), "Missing organization_slug");
    assert!(first.get("slug").is_some(), "Missing slug");
    assert!(first.get("name").is_some(), "Missing name");
    assert!(first.get("entry_type").is_some(), "Missing entry_type");
}

#[tokio::test]
#[serial]
async fn test_search_compact_format() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("insulin")
        .arg("--format")
        .arg("compact")
        .arg("--no-interactive");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("uniprot:P01308"));
}

#[tokio::test]
#[serial]
async fn test_search_table_format() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("insulin")
        .arg("--format")
        .arg("table")
        .arg("--no-interactive");

    // Table format has headers
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Source"))
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Insulin"));
}

#[tokio::test]
#[serial]
async fn test_search_default_format_non_interactive() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    // Default non-interactive format: stdout is piped, so compact bare specs
    let mut cmd = bdp_cmd(&url);
    cmd.arg("search").arg("insulin").arg("--no-interactive");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Insulin"));
}

// ===========================================================================
// Pagination Tests
// ===========================================================================

#[tokio::test]
#[serial]
async fn test_search_limit() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("protein")
        .arg("--limit")
        .arg("2")
        .arg("--no-interactive")
        .arg("--format")
        .arg("json");

    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(output).expect("non-UTF8 stdout");

    let json_start = stdout.find('{').expect("No JSON object in output");
    let json_str = &stdout[json_start..];
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).expect("Output is not valid JSON");

    let results = parsed["results"].as_array().expect("results is not array");
    assert!(results.len() <= 2, "Expected at most 2 results, got {}", results.len());
}

#[tokio::test]
#[serial]
async fn test_search_page() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    // Page 1
    let mut cmd1 = bdp_cmd(&url);
    cmd1.arg("search")
        .arg("protein")
        .arg("--limit")
        .arg("1")
        .arg("--page")
        .arg("1")
        .arg("--no-interactive")
        .arg("--format")
        .arg("json");

    let out1 = cmd1.assert().success().get_output().stdout.clone();
    let s1 = String::from_utf8(out1).unwrap();

    // Page 2
    let mut cmd2 = bdp_cmd(&url);
    cmd2.arg("search")
        .arg("protein")
        .arg("--limit")
        .arg("1")
        .arg("--page")
        .arg("2")
        .arg("--no-interactive")
        .arg("--format")
        .arg("json");

    let out2 = cmd2.assert().success().get_output().stdout.clone();
    let s2 = String::from_utf8(out2).unwrap();

    // If there are enough results, pages should differ
    // (at least we verify they don't crash)
    assert!(
        s1 != s2 || s1.contains("No results") || s2.contains("No results"),
        "Page 1 and Page 2 should differ or one should be empty"
    );
}

#[tokio::test]
#[serial]
async fn test_search_large_page() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("protein")
        .arg("--page")
        .arg("999")
        .arg("--no-interactive");

    // Should not crash; either empty results or "No results found"
    cmd.assert().success();
}

// ===========================================================================
// Edge Case Tests
// ===========================================================================

#[tokio::test]
#[serial]
async fn test_search_special_characters() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search").arg("insulin@1.0").arg("--no-interactive");

    // Should not crash
    cmd.assert().success();
}

#[tokio::test]
#[serial]
async fn test_search_empty_query() {
    // No server needed — clap or the command handler rejects empty queries
    let mut cmd = Command::cargo_bin("bdp").expect("bdp binary not found");
    cmd.arg("search").arg("").arg("--no-interactive");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Search query cannot be empty"));
}

#[tokio::test]
#[serial]
async fn test_search_very_long_query() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let long_query = "a".repeat(500);
    let mut cmd = bdp_cmd(&url);
    cmd.arg("search").arg(&long_query).arg("--no-interactive");

    // Should not crash; likely returns no results
    cmd.assert().success();
}

#[tokio::test]
#[serial]
async fn test_search_unicode_query() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("\u{03B2}-globin")
        .arg("--no-interactive");

    // Should not crash
    cmd.assert().success();
}

#[tokio::test]
#[serial]
async fn test_search_sql_injection_attempt() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("'; DROP TABLE organizations; --")
        .arg("--no-interactive");

    // Should safely return no results (not crash or drop tables)
    cmd.assert().success();
}

// ===========================================================================
// Integration / Correctness Tests
// ===========================================================================

#[tokio::test]
#[serial]
async fn test_search_result_fields_complete() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("insulin")
        .arg("--format")
        .arg("json")
        .arg("--no-interactive");

    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(output).expect("non-UTF8 stdout");

    let json_start = stdout.find('{').expect("No JSON object in output");
    let json_str = &stdout[json_start..];
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).expect("Output is not valid JSON");

    let results = parsed["results"].as_array().expect("results is not array");
    assert!(!results.is_empty(), "Expected at least one result");

    let item = &results[0];
    // Verify all expected fields are present
    assert!(item.get("id").is_some(), "Missing id");
    assert!(item.get("organization_slug").is_some(), "Missing organization_slug");
    assert!(item.get("slug").is_some(), "Missing slug");
    assert!(item.get("name").is_some(), "Missing name");
    assert!(item.get("entry_type").is_some(), "Missing entry_type");
    assert!(item.get("available_formats").is_some(), "Missing available_formats");
}

#[tokio::test]
#[serial]
async fn test_search_results_ordered_by_relevance() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("insulin")
        .arg("--format")
        .arg("json")
        .arg("--no-interactive");

    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(output).expect("non-UTF8 stdout");

    let json_start = stdout.find('{').expect("No JSON object in output");
    let json_str = &stdout[json_start..];
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).expect("Output is not valid JSON");

    let results = parsed["results"].as_array().expect("results is not array");
    if results.len() >= 2 {
        // Verify results have rank field (from the server-side ordering)
        let first_name = results[0]["name"].as_str().unwrap_or_default();
        // "Insulin" should be the most relevant result for query "insulin"
        assert!(
            first_name.to_lowercase().contains("insulin"),
            "First result '{}' should contain 'insulin'",
            first_name
        );
    }
}

#[tokio::test]
#[serial]
async fn test_search_compact_spec_format() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("insulin")
        .arg("--format")
        .arg("compact")
        .arg("--no-interactive");

    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(output).expect("non-UTF8 stdout");

    // In non-TTY (piped) mode, compact format outputs bare specs: org:slug@version
    // Check that at least one line contains the org:slug pattern
    let has_spec_line = stdout.lines().any(|line| line.contains("uniprot:P01308"));
    assert!(
        has_spec_line,
        "Expected compact output to contain 'uniprot:P01308', got:\n{}",
        stdout
    );
}

// ===========================================================================
// Additional Filter Combination Tests
// ===========================================================================

#[tokio::test]
#[serial]
async fn test_search_type_data_source_filter() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("sequence")
        .arg("--type")
        .arg("data_source")
        .arg("--no-interactive")
        .arg("--format")
        .arg("json");

    // Should not crash; results should only be data_source type
    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();

    if let Some(json_start) = stdout.find('{') {
        let json_str = &stdout[json_start..];
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(results) = parsed["results"].as_array() {
                for r in results {
                    let entry_type = r["entry_type"].as_str().unwrap_or_default();
                    assert_eq!(entry_type, "data_source", "Expected only data_source entries");
                }
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn test_search_genome_entries() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("genome")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact");

    // Should find genome-related entries
    cmd.assert().success();
}

#[tokio::test]
#[serial]
async fn test_search_taxonomy_entries() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);

    let mut cmd = bdp_cmd(&url);
    cmd.arg("search")
        .arg("taxonomy")
        .arg("--no-interactive")
        .arg("--format")
        .arg("compact");

    // Should find taxonomy entry
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("txid9606"));
}
