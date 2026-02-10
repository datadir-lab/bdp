//! Full workflow E2E tests for `bdp` CLI
//!
//! Exercises the complete workflow: search → source add → pull → clean → pull
//! Uses a **real PostgreSQL** (via testcontainers) and an **in-process axum server**
//! with a custom download route for testing.
//!
//! Requires Docker Desktop to be running.

#![allow(deprecated)]

use assert_cmd::Command;
use axum::{
    extract::{Path as AxumPath, Query},
    routing::get,
    Router,
};
use predicates::prelude::*;
use serial_test::serial;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use std::net::SocketAddr;
use tempfile::TempDir;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use tokio::sync::OnceCell;

// ---------------------------------------------------------------------------
// Synthetic download content helpers
// ---------------------------------------------------------------------------

/// Generate deterministic synthetic file content for a given org/name/format/version.
fn synthetic_content(org: &str, name: &str, format: &str, version: &str) -> Vec<u8> {
    format!("{}:{}-{}@{}\n", org, name, format, version).into_bytes()
}

/// Compute SHA-256 hex digest of bytes.
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// Shared test infrastructure
// ---------------------------------------------------------------------------

struct TestServer {
    addr: SocketAddr,
    #[allow(dead_code)]
    pool: PgPool,
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

            // 4. Seed test data with real checksums
            seed_test_data(&pool)
                .await
                .expect("Failed to seed test data");

            // 5. Create dummy Storage (search/resolve never touch S3)
            let storage_config = bdp_server::storage::config::StorageConfig::for_minio(
                "http://127.0.0.1:19999",
                "bdp-test",
            );
            let storage = bdp_server::storage::Storage::new(storage_config)
                .await
                .expect("Failed to create dummy storage");

            // 6. Build the axum router with standard features + custom download route
            let feature_state = bdp_server::features::FeatureState {
                db: pool.clone(),
                storage,
            };
            let api_v1 = bdp_server::features::router(feature_state);

            // Custom download route that returns synthetic content
            let download_router = Router::new().route(
                "/api/v1/data-sources/{org}/{name}/{version}/download",
                get(download_handler),
            );

            let app: Router = Router::new()
                .route("/health", get(|| async { "OK" }))
                .merge(download_router)
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

/// Download handler that returns deterministic synthetic content.
async fn download_handler(
    AxumPath((org, name, version)): AxumPath<(String, String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Vec<u8> {
    let format = params.get("format").map(|s| s.as_str()).unwrap_or("bin");
    synthetic_content(&org, &name, format, &version)
}

/// Seed test data: 3 organizations, 8 registry entries, versions, and files.
/// Uses real SHA-256 checksums computed from the synthetic download content.
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

    // ── Helper: insert data source with real checksums ───────────────────
    #[allow(clippy::too_many_arguments)]
    async fn insert_data_source(
        pool: &PgPool,
        org_id: uuid::Uuid,
        org_slug: &str,
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
            // Compute real checksum from synthetic content
            let content = synthetic_content(org_slug, slug, fmt, version);
            let checksum = sha256_hex(&content);
            let size = content.len() as i64;

            let filename = format!("{}.{}", slug, fmt);
            let s3_key = format!("data/{}/{}/{}", slug, version, filename);
            sqlx::query(
                "INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(version_id)
            .bind(*fmt)
            .bind(&s3_key)
            .bind(&checksum)
            .bind(size)
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
        "uniprot",
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
        "uniprot",
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
        "uniprot",
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
        "ncbi",
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
        "ncbi",
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
        "ncbi",
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

/// Build a CLI command pointing at the test server, running in the given directory.
fn bdp_cmd(server_url: &str, working_dir: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("bdp").expect("bdp binary not found");
    cmd.arg("--server-url").arg(server_url);
    cmd.current_dir(working_dir);
    // Use a unique cache dir per test to avoid cross-test interference
    let cache_dir = working_dir.join(".bdp-cache");
    cmd.env("BDP_CACHE_DIR", cache_dir.to_str().unwrap());
    cmd
}

/// Build the server URL string from the shared test server address.
fn server_url(addr: SocketAddr) -> String {
    format!("http://{}", addr)
}

// ===========================================================================
// Workflow Tests
// ===========================================================================

#[tokio::test]
#[serial]
async fn test_init_creates_manifest() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    let mut cmd = bdp_cmd(&url, temp.path());
    cmd.arg("init")
        .arg("--name")
        .arg("test-project")
        .arg("--force");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialized BDP project"));

    // Verify bdp.yml was created
    assert!(temp.path().join("bdp.yml").exists());
    // Verify .bdp directory was created
    assert!(temp.path().join(".bdp").exists());
}

#[tokio::test]
#[serial]
async fn test_source_add_and_list() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Init project
    bdp_cmd(&url, temp.path())
        .arg("init")
        .arg("--name")
        .arg("test-project")
        .arg("--force")
        .assert()
        .success();

    // Add a source
    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("add")
        .arg("uniprot:P01308-fasta@1.0")
        .assert()
        .success()
        .stdout(predicate::str::contains("Added source"));

    // List sources
    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("uniprot:P01308-fasta@1.0"))
        .stdout(predicate::str::contains("1 source(s)"));
}

#[tokio::test]
#[serial]
async fn test_source_add_duplicate() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Init project
    bdp_cmd(&url, temp.path())
        .arg("init")
        .arg("--name")
        .arg("test-project")
        .arg("--force")
        .assert()
        .success();

    // Add a source
    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("add")
        .arg("uniprot:P01308-fasta@1.0")
        .assert()
        .success();

    // Add the same source again
    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("add")
        .arg("uniprot:P01308-fasta@1.0")
        .assert()
        .success()
        .stdout(predicate::str::contains("already exists"));
}

#[tokio::test]
#[serial]
async fn test_source_remove() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Init + add
    bdp_cmd(&url, temp.path())
        .arg("init")
        .arg("--name")
        .arg("test-project")
        .arg("--force")
        .assert()
        .success();

    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("add")
        .arg("uniprot:P01308-fasta@1.0")
        .assert()
        .success();

    // Remove source
    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("remove")
        .arg("uniprot:P01308-fasta@1.0")
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed source"));

    // List should show no sources
    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No sources defined"));
}

#[tokio::test]
#[serial]
async fn test_pull_downloads_sources() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Init + add source
    bdp_cmd(&url, temp.path())
        .arg("init")
        .arg("--name")
        .arg("test-project")
        .arg("--force")
        .assert()
        .success();

    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("add")
        .arg("uniprot:P01308-fasta@1.0")
        .assert()
        .success();

    // Pull
    bdp_cmd(&url, temp.path())
        .arg("pull")
        .assert()
        .success()
        .stdout(predicate::str::contains("Resolving dependencies"))
        .stdout(predicate::str::contains("verified"))
        .stdout(predicate::str::contains("All sources downloaded"));

    // Verify lockfile exists
    assert!(temp.path().join("bdp.lock").exists());
}

#[tokio::test]
#[serial]
async fn test_pull_cached_skips_download() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Init + add source + first pull
    bdp_cmd(&url, temp.path())
        .arg("init")
        .arg("--name")
        .arg("test-project")
        .arg("--force")
        .assert()
        .success();

    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("add")
        .arg("uniprot:P01308-fasta@1.0")
        .assert()
        .success();

    bdp_cmd(&url, temp.path()).arg("pull").assert().success();

    // Second pull should show "(cached)"
    bdp_cmd(&url, temp.path())
        .arg("pull")
        .assert()
        .success()
        .stdout(predicate::str::contains("cached"));
}

// ===========================================================================
// Clean + Re-pull Tests
// ===========================================================================

#[tokio::test]
#[serial]
async fn test_clean_all_removes_cache() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Init + add + pull
    bdp_cmd(&url, temp.path())
        .arg("init")
        .arg("--name")
        .arg("test-project")
        .arg("--force")
        .assert()
        .success();

    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("add")
        .arg("uniprot:P01308-fasta@1.0")
        .assert()
        .success();

    bdp_cmd(&url, temp.path()).arg("pull").assert().success();

    // Clean all
    bdp_cmd(&url, temp.path())
        .arg("clean")
        .arg("--all")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleared"));
}

#[tokio::test]
#[serial]
async fn test_pull_after_clean_redownloads() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Init + add + pull
    bdp_cmd(&url, temp.path())
        .arg("init")
        .arg("--name")
        .arg("test-project")
        .arg("--force")
        .assert()
        .success();

    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("add")
        .arg("uniprot:P01308-fasta@1.0")
        .assert()
        .success();

    bdp_cmd(&url, temp.path()).arg("pull").assert().success();

    // Clean all
    bdp_cmd(&url, temp.path())
        .arg("clean")
        .arg("--all")
        .assert()
        .success();

    // Pull again — should re-download (not cached)
    bdp_cmd(&url, temp.path())
        .arg("pull")
        .assert()
        .success()
        .stdout(predicate::str::contains("Downloading"))
        .stdout(predicate::str::contains("verified"));
}

#[tokio::test]
#[serial]
async fn test_clean_search_cache() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Just run clean --search-cache (doesn't require init)
    bdp_cmd(&url, temp.path())
        .arg("clean")
        .arg("--search-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("search cache"));
}

// ===========================================================================
// Full Integration Tests
// ===========================================================================

#[tokio::test]
#[serial]
async fn test_search_then_add_then_pull() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Search for insulin
    let output = bdp_cmd(&url, temp.path())
        .arg("search")
        .arg("insulin")
        .arg("--format")
        .arg("compact")
        .arg("--no-interactive")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("non-UTF8");
    assert!(stdout.contains("uniprot:P01308"));

    // Init project
    bdp_cmd(&url, temp.path())
        .arg("init")
        .arg("--name")
        .arg("search-then-pull")
        .arg("--force")
        .assert()
        .success();

    // Add the found source
    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("add")
        .arg("uniprot:P01308-fasta@1.0")
        .assert()
        .success();

    // Pull
    bdp_cmd(&url, temp.path())
        .arg("pull")
        .assert()
        .success()
        .stdout(predicate::str::contains("All sources downloaded"));
}

#[tokio::test]
#[serial]
async fn test_pull_multiple_sources() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Init
    bdp_cmd(&url, temp.path())
        .arg("init")
        .arg("--name")
        .arg("multi-source")
        .arg("--force")
        .assert()
        .success();

    // Add two sources
    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("add")
        .arg("uniprot:P01308-fasta@1.0")
        .assert()
        .success();

    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("add")
        .arg("ncbi:NC_000001-fasta@GRCh38")
        .assert()
        .success();

    // Pull both
    let output = bdp_cmd(&url, temp.path())
        .arg("pull")
        .assert()
        .success()
        .stdout(predicate::str::contains("All sources downloaded"))
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("non-UTF8");
    // Should have resolved 2 sources
    assert!(stdout.contains("2 source(s)"));
}

#[tokio::test]
#[serial]
async fn test_pull_nonexistent_source() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Init
    bdp_cmd(&url, temp.path())
        .arg("init")
        .arg("--name")
        .arg("nonexistent")
        .arg("--force")
        .assert()
        .success();

    // Add a nonexistent source
    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("add")
        .arg("fake:nothing-txt@0.0")
        .assert()
        .success();

    // Pull should fail gracefully
    bdp_cmd(&url, temp.path()).arg("pull").assert().failure();
}

// ===========================================================================
// Edge Case Tests
// ===========================================================================

#[tokio::test]
#[serial]
async fn test_pull_without_init() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Pull without init — should fail
    bdp_cmd(&url, temp.path())
        .arg("pull")
        .assert()
        .failure()
        .stderr(predicate::str::contains("bdp.yml").or(predicate::str::contains("init")));
}

#[tokio::test]
#[serial]
async fn test_pull_empty_manifest() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Init (creates manifest with no sources)
    bdp_cmd(&url, temp.path())
        .arg("init")
        .arg("--name")
        .arg("empty")
        .arg("--force")
        .assert()
        .success();

    // Pull with no sources
    bdp_cmd(&url, temp.path())
        .arg("pull")
        .assert()
        .success()
        .stdout(predicate::str::contains("No sources to pull"));
}

#[tokio::test]
#[serial]
async fn test_pull_force_redownloads() {
    let ts = get_test_server().await;
    let url = server_url(ts.addr);
    let temp = TempDir::new().expect("temp dir");

    // Init + add + pull
    bdp_cmd(&url, temp.path())
        .arg("init")
        .arg("--name")
        .arg("force-test")
        .arg("--force")
        .assert()
        .success();

    bdp_cmd(&url, temp.path())
        .arg("source")
        .arg("add")
        .arg("uniprot:P01308-fasta@1.0")
        .assert()
        .success();

    bdp_cmd(&url, temp.path()).arg("pull").assert().success();

    // Pull with --force should re-download (not use cache)
    bdp_cmd(&url, temp.path())
        .arg("pull")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Downloading"))
        .stdout(predicate::str::contains("verified"));
}
