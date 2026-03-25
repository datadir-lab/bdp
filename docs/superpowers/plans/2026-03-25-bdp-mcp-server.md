# BDP MCP Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `crates/bdp-mcp` — a Rust binary that exposes the BDP biological knowledge graph as an MCP server usable by Claude Desktop, Claude Code, and custom agents.

**Architecture:** rmcp v1.2.0 handles MCP protocol framing. Tools are defined on `BdpMcpServer` using `#[tool_router]` + `#[tool_handler]` macros. Entity resolution accepts canonical IDs (MONDO:, HP:, CHEBI:, R-HSA-) or free-text via PostgreSQL FTS. stdio transport first, Streamable HTTP second.

**Tech Stack:** Rust, rmcp 1.2.0 (MCP SDK), axum 0.7, sqlx 0.8 (runtime queries only — no macros), PostgreSQL 16, testcontainers for integration tests.

**Spec:** `docs/superpowers/specs/2026-03-25-bdp-mcp-server-design.md`

---

## File Map

| File | Responsibility |
|------|---------------|
| `crates/bdp-mcp/Cargo.toml` | Crate manifest + dependencies |
| `crates/bdp-mcp/src/main.rs` | Binary entry point; `--transport stdio\|http`, `--port` |
| `crates/bdp-mcp/src/config.rs` | Config from env + CLI args |
| `crates/bdp-mcp/src/server.rs` | `BdpMcpServer` struct; `#[tool_router]` + `#[tool_handler]`; common result helpers |
| `crates/bdp-mcp/src/db/mod.rs` | Re-exports db submodules |
| `crates/bdp-mcp/src/db/resolve.rs` | `resolve_entity()` — ID pattern match + FTS fallback |
| `crates/bdp-mcp/src/db/queries.rs` | All `sqlx::query()` calls; one function per tool query |
| `crates/bdp-mcp/src/db/audit.rs` | `log_tool_call()` — writes to `agent_query_log`, swallows errors |
| `crates/bdp-mcp/src/tools/mod.rs` | Re-exports all tool modules |
| `crates/bdp-mcp/src/tools/diseases.rs` | `get_disease`, `get_disease_phenotypes`, stubs |
| `crates/bdp-mcp/src/tools/phenotypes.rs` | `get_phenotype`, `get_phenotype_diseases` |
| `crates/bdp-mcp/src/tools/genes.rs` | `get_gene`, `get_gene_pathways`, stubs |
| `crates/bdp-mcp/src/tools/pathways.rs` | `get_pathway`, `get_pathway_proteins` |
| `crates/bdp-mcp/src/tools/compounds.rs` | `get_compound`, `get_compound_roles`, stubs |
| `crates/bdp-mcp/src/tools/literature.rs` | All stubs |
| `crates/bdp-mcp/src/tools/traversal.rs` | `traverse` (partial live), `find_connection` stub |
| `crates/bdp-mcp/src/lib.rs` | Library target for integration test imports; declares `pub mod config/db/server/tools` |
| `crates/bdp-mcp/tests/common.rs` | `TestPostgres` helper (exact bdp-ingest pattern) |
| `crates/bdp-mcp/tests/integration.rs` | End-to-end DB tests |
| `Cargo.toml` (workspace root) | Add `crates/bdp-mcp` to members |

---

## Task 1: Crate Scaffold

**Files:**
- Create: `crates/bdp-mcp/Cargo.toml`
- Create: `crates/bdp-mcp/src/main.rs` (stub)
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create crate directory**

```bash
mkdir -p D:/dev/datadir/bdp/crates/bdp-mcp/src
```

- [ ] **Step 2: Create `crates/bdp-mcp/Cargo.toml`**

```toml
[package]
name = "bdp-mcp"
version.workspace = true
edition.workspace = true
authors.workspace = true
rust-version.workspace = true

[[bin]]
name = "bdp-mcp"
path = "src/main.rs"

[dependencies]
rmcp = { version = "1.2", features = [
    "server",
    "macros",
    "schemars",
    "transport-streamable-http-server",
    "transport-async-rw",
] }
axum = { workspace = true }
schemars = "0.8"
serde = { workspace = true }
serde_json = { workspace = true }
sqlx = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
uuid = { workspace = true }
base64 = "0.22"
anyhow = { workspace = true }
clap = { version = "4", features = ["derive", "env"] }
chrono = { workspace = true }

[dev-dependencies]
testcontainers = "0.23"
testcontainers-modules = { version = "0.11", features = ["postgres"] }
tokio = { workspace = true, features = ["full"] }
```

- [ ] **Step 3: Create stub `crates/bdp-mcp/src/main.rs`**

```rust
fn main() {
    println!("bdp-mcp stub");
}
```

- [ ] **Step 4: Add crate to workspace root `Cargo.toml`**

Find the `members = [` line and add `"crates/bdp-mcp"` to the list.

- [ ] **Step 5: Verify it compiles**

```bash
cd D:/dev/datadir/bdp
cargo build -p bdp-mcp
```

Expected: compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add crates/bdp-mcp/ Cargo.toml Cargo.lock
git commit -m "chore(mcp): scaffold bdp-mcp crate"
```

---

## Task 2: Config

**Files:**
- Create: `crates/bdp-mcp/src/config.rs`
- Modify: `crates/bdp-mcp/src/main.rs`

- [ ] **Step 1: Write failing test**

Add to `config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        // DATABASE_URL must be set in env
        std::env::set_var("DATABASE_URL", "postgresql://test");
        let cfg = Config::from_env_and_args(&["bdp-mcp".to_string()]);
        assert_eq!(cfg.transport, Transport::Stdio);
        assert_eq!(cfg.port, 3000);
    }
}
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd D:/dev/datadir/bdp
cargo test -p bdp-mcp config -- --nocapture
```

Expected: compile error (Config not defined).

- [ ] **Step 3: Implement `config.rs`**

```rust
// crates/bdp-mcp/src/config.rs

use clap::Parser;

#[derive(Debug, Clone, PartialEq)]
pub enum Transport {
    Stdio,
    Http,
}

impl std::str::FromStr for Transport {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stdio" => Ok(Transport::Stdio),
            "http" => Ok(Transport::Http),
            other => Err(format!("Unknown transport: {other}. Use 'stdio' or 'http'")),
        }
    }
}

#[derive(Debug, Clone, Parser)]
#[command(name = "bdp-mcp", about = "BDP MCP server")]
pub struct Config {
    /// Transport mode: stdio (default) or http
    #[arg(long, env = "BDP_MCP_TRANSPORT", default_value = "stdio")]
    pub transport: Transport,

    /// HTTP port (used when transport=http)
    #[arg(long, env = "BDP_MCP_PORT", default_value = "3000")]
    pub port: u16,

    /// PostgreSQL connection URL
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,

    /// Max DB connections
    #[arg(long, env = "DB_MAX_CONNECTIONS", default_value = "10")]
    pub db_max_connections: u32,
}

impl Config {
    pub fn from_env_and_args(args: &[String]) -> Self {
        Config::parse_from(args)
    }
}
```

- [ ] **Step 4: Run test to confirm it passes**

```bash
cargo test -p bdp-mcp config
```

Expected: PASS.

- [ ] **Step 5: Update `main.rs`**

```rust
// crates/bdp-mcp/src/main.rs

mod config;

use clap::Parser;
use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_env("RUST_LOG")
                .add_directive("bdp_mcp=info".parse()?),
        )
        .init();

    let cfg = Config::parse();
    tracing::info!(transport = ?cfg.transport, port = cfg.port, "bdp-mcp starting");

    Ok(())
}
```

- [ ] **Step 6: Verify compiles**

```bash
cargo build -p bdp-mcp
```

- [ ] **Step 7: Commit**

```bash
git add crates/bdp-mcp/
git commit -m "feat(mcp): add Config with transport + database_url"
```

---

## Task 3: Entity Resolution

**Files:**
- Create: `crates/bdp-mcp/src/db/mod.rs`
- Create: `crates/bdp-mcp/src/db/resolve.rs`

Entity resolution converts any input string ("MONDO:0004975" or "Alzheimer disease") into a DB row ID. Pattern matching for canonical IDs; PostgreSQL FTS for names.

- [ ] **Step 1: Write failing tests**

Create `crates/bdp-mcp/src/db/resolve.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_mondo_id() {
        assert!(matches!(
            detect_id_type("MONDO:0004975"),
            Some(CanonicalId::Mondo("MONDO:0004975"))
        ));
    }

    #[test]
    fn test_detect_hpo_id() {
        assert!(matches!(
            detect_id_type("HP:0001234"),
            Some(CanonicalId::Hpo("HP:0001234"))
        ));
    }

    #[test]
    fn test_detect_chebi_id() {
        assert!(matches!(
            detect_id_type("CHEBI:15422"),
            Some(CanonicalId::Chebi("CHEBI:15422"))
        ));
    }

    #[test]
    fn test_detect_reactome_id() {
        assert!(matches!(
            detect_id_type("R-HSA-109581"),
            Some(CanonicalId::Reactome("R-HSA-109581"))
        ));
    }

    #[test]
    fn test_uniprot_accession() {
        assert!(matches!(
            detect_id_type("P38398"),
            Some(CanonicalId::UniProt("P38398"))
        ));
    }

    #[test]
    fn test_free_text_returns_none() {
        assert!(detect_id_type("Alzheimer disease").is_none());
        assert!(detect_id_type("BRCA1").is_none());
    }

    #[test]
    fn test_input_length_cap() {
        let long = "a".repeat(501);
        assert!(cap_input(&long).len() <= 500);
    }
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test -p bdp-mcp resolve -- --nocapture
```

Expected: compile error.

- [ ] **Step 3: Implement `resolve.rs`**

```rust
// crates/bdp-mcp/src/db/resolve.rs

use sqlx::PgPool;
use uuid::Uuid;

/// Canonical ID type detected from the input string.
#[derive(Debug)]
pub enum CanonicalId<'a> {
    Mondo(&'a str),
    Hpo(&'a str),
    Chebi(&'a str),
    Reactome(&'a str),
    UniProt(&'a str),
}

/// Clamp input to 500 chars to prevent FTS query amplification.
pub fn cap_input(input: &str) -> &str {
    let end = input
        .char_indices()
        .nth(500)
        .map(|(i, _)| i)
        .unwrap_or(input.len());
    &input[..end]
}

/// Detect if the input string is a recognized canonical ID pattern.
/// Returns None if input looks like free text (name/alias).
pub fn detect_id_type(input: &str) -> Option<CanonicalId<'_>> {
    // MONDO:0000000
    if input.starts_with("MONDO:") && input[6..].chars().all(|c| c.is_ascii_digit()) {
        return Some(CanonicalId::Mondo(input));
    }
    // HP:0000000
    if input.starts_with("HP:") && input[3..].chars().all(|c| c.is_ascii_digit()) {
        return Some(CanonicalId::Hpo(input));
    }
    // CHEBI:00000
    if input.starts_with("CHEBI:") && input[6..].chars().all(|c| c.is_ascii_digit()) {
        return Some(CanonicalId::Chebi(input));
    }
    // R-HSA-000000 or R-MMU-000000 etc.
    if input.starts_with("R-") && input.contains('-') {
        return Some(CanonicalId::Reactome(input));
    }
    // UniProt accession: [A-Z][0-9][A-Z0-9]{3}[0-9] (6 chars) or [OPQ][0-9][A-Z0-9]{3}[0-9] (10 chars)
    let bytes = input.as_bytes();
    if (bytes.len() == 6 || bytes.len() == 10)
        && bytes[0].is_ascii_uppercase()
        && bytes[1].is_ascii_digit()
    {
        return Some(CanonicalId::UniProt(input));
    }
    None
}

/// Fuzzy resolution result for FTS name searches.
#[derive(Debug)]
pub struct FtsMatch {
    pub id: Uuid,
    pub name: String,
}

/// Find a disease by MONDO canonical ID.
pub async fn disease_by_mondo_id(pool: &PgPool, mondo_id: &str) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT id FROM disease_terms WHERE mondo_id = $1 AND is_obsolete = FALSE")
        .bind(mondo_id)
        .fetch_optional(pool)
        .await
}

/// Find up to 5 diseases by FTS name match.
/// Uses sqlx::query() runtime — NOT sqlx::query!() macro (bdp-mcp has no offline cache).
pub async fn diseases_by_name(pool: &PgPool, name: &str) -> sqlx::Result<Vec<FtsMatch>> {
    let name = cap_input(name);
    let rows = sqlx::query(
        "SELECT id, name FROM disease_terms,
         plainto_tsquery('english', $1) q
         WHERE to_tsvector('english', name) @@ q AND is_obsolete = FALSE
         ORDER BY ts_rank(to_tsvector('english', name), q) DESC LIMIT 5",
    )
    .bind(name)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|r| FtsMatch { id: r.get("id"), name: r.get("name") })
        .collect())
}

/// Find HPO term by HP: canonical ID.
pub async fn phenotype_by_hpo_id(pool: &PgPool, hpo_id: &str) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT id FROM hpo_term_metadata WHERE hpo_id = $1 AND is_obsolete = FALSE")
        .bind(hpo_id)
        .fetch_optional(pool)
        .await
}

/// Find compound by CHEBI: canonical ID.
pub async fn compound_by_chebi_id(pool: &PgPool, chebi_id: &str) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT id FROM compound_terms WHERE chebi_id = $1 AND is_obsolete = FALSE")
        .bind(chebi_id)
        .fetch_optional(pool)
        .await
}

/// Find pathway by R-HSA-... canonical ID.
pub async fn pathway_by_reactome_id(pool: &PgPool, reactome_id: &str) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT id FROM pathway_terms WHERE reactome_id = $1")
        .bind(reactome_id)
        .fetch_optional(pool)
        .await
}
```

> **Important:** All resolve functions use `sqlx::query()` runtime queries (or `sqlx::query_scalar()` which is runtime-safe). Do NOT use `sqlx::query!()` macros in bdp-mcp — they require the SQLx offline cache which is not configured for this crate.

- [ ] **Step 4: Create `crates/bdp-mcp/src/db/mod.rs`**

```rust
pub mod audit;
pub mod queries;
pub mod resolve;
```

- [ ] **Step 5: Add stubs for audit and queries so it compiles**

Create `crates/bdp-mcp/src/db/audit.rs`:
```rust
// Stub — implemented in Task 7
```

Create `crates/bdp-mcp/src/db/queries.rs`:
```rust
// Stub — implemented in Tasks 4-6
```

- [ ] **Step 6: Add `mod db;` to `main.rs`**

```rust
mod config;
mod db;
```

- [ ] **Step 7: Run unit tests**

```bash
cargo test -p bdp-mcp resolve
```

Expected: all 7 tests PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/bdp-mcp/
git commit -m "feat(mcp): add entity resolution (ID pattern + FTS)"
```

---

## Task 4: Disease Queries

**Files:**
- Modify: `crates/bdp-mcp/src/db/queries.rs`
- Create: `crates/bdp-mcp/tests/common.rs`
- Create: `crates/bdp-mcp/tests/integration.rs`

All queries use `sqlx::query()` (runtime). Return-type structs are plain Rust with manual field mapping.

- [ ] **Step 1: Create test infrastructure**

Create `crates/bdp-mcp/tests/common.rs`:

```rust
// crates/bdp-mcp/tests/common.rs
//
// Exact pattern from crates/bdp-ingest/tests/common.rs.
// Uses .with_tag("16-alpine"), 5432.tcp(), acquire_timeout(30s).

#![allow(dead_code)]

use anyhow::{Context, Result};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use testcontainers::{core::IntoContainerPort, runners::AsyncRunner, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

pub struct TestPostgres {
    _container: testcontainers::ContainerAsync<Postgres>,
    pub pool: PgPool,
}

impl TestPostgres {
    pub async fn start() -> Result<Self> {
        let container = Postgres::default()
            .with_tag("16-alpine")
            .start()
            .await
            .context("start postgres container")?;

        let host = container.get_host().await.context("get host")?;
        let port = container
            .get_host_port_ipv4(5432.tcp())
            .await
            .context("get port")?;

        let url = format!("postgresql://postgres:postgres@{}:{}/postgres", host, port);

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .connect(&url)
            .await
            .context("connect to postgres")?;

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .context("run migrations")?;

        Ok(Self { _container: container, pool })
    }
}

pub async fn create_test_org(pool: &PgPool, slug: &str) -> Result<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO organizations (slug, name, description, is_system)
         VALUES ($1, $2, $3, false) RETURNING id",
    )
    .bind(slug)
    .bind(format!("{slug} (test)"))
    .bind("MCP test organization")
    .fetch_one(pool)
    .await
    .context("create org")?;
    Ok(id)
}

pub async fn count_rows(pool: &PgPool, table: &str) -> Result<i64> {
    let n: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .context("count rows")?;
    Ok(n)
}

pub async fn seed_disease(pool: &PgPool, org_id: uuid::Uuid) -> anyhow::Result<uuid::Uuid> {
    // Insert a minimal registry_entry + data_source + disease_term for testing.
    let re_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO registry_entries (id, org_id, entry_type, slug, name)
         VALUES ($1, $2, 'data_source', 'mondo-test', 'MONDO Test')"
    )
    .bind(re_id)
    .bind(org_id)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO data_sources (id, source_type, external_id)
         VALUES ($1, 'ontology', 'mondo')"
    )
    .bind(re_id)
    .execute(pool)
    .await?;

    let disease_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO disease_terms
         (id, data_source_id, mondo_id, mondo_accession, name, definition, is_obsolete, omim_id, mondo_release)
         VALUES ($1, $2, 'MONDO:0004975', 4975, 'Alzheimer disease',
                 'A progressive brain disorder', FALSE, '104300', '2026-01')"
    )
    .bind(disease_id)
    .bind(re_id)
    .execute(pool)
    .await?;

    Ok(disease_id)
}
```

- [ ] **Step 2: Write failing integration test**

Create `crates/bdp-mcp/tests/integration.rs`:

```rust
mod common;

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_get_disease_by_mondo_id() {
    let pg = common::TestPostgres::start().await.expect("start postgres");
    let org_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, slug, name) VALUES ($1, 'test', 'Test')")
        .bind(org_id)
        .execute(&pg.pool)
        .await
        .unwrap();

    common::seed_disease(&pg.pool, org_id).await.unwrap();

    let result = bdp_mcp::db::queries::get_disease(&pg.pool, "MONDO:0004975")
        .await
        .expect("query ok");

    assert!(result.is_some());
    let d = result.unwrap();
    assert_eq!(d.mondo_id, "MONDO:0004975");
    assert_eq!(d.name, "Alzheimer disease");
    assert_eq!(d.omim_id.as_deref(), Some("104300"));
}
```

- [ ] **Step 3: Run to confirm it fails**

```bash
cargo test -p bdp-mcp test_get_disease -- --nocapture --ignored
```

Expected: compile error (queries module not found).

- [ ] **Step 4: Implement disease queries in `queries.rs`**

```rust
// crates/bdp-mcp/src/db/queries.rs

use sqlx::{PgPool, Row};
use uuid::Uuid;

// ─── Disease ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct DiseaseRow {
    pub id: Uuid,
    pub mondo_id: String,
    pub name: String,
    pub definition: Option<String>,
    pub omim_id: Option<String>,
    pub orphanet_id: Option<String>,
    pub mondo_release: String,
}

#[derive(Debug)]
pub struct DiseaseSynonymRow {
    pub scope: String,
    pub text: String,
}

#[derive(Debug)]
pub struct DiseaseXrefRow {
    pub source_db: String,
    pub source_id: String,
}

/// Fetch a disease term by MONDO ID string (e.g. "MONDO:0004975").
pub async fn get_disease(pool: &PgPool, mondo_id: &str) -> sqlx::Result<Option<DiseaseRow>> {
    let row = sqlx::query(
        "SELECT id, mondo_id, name, definition, omim_id, orphanet_id, mondo_release
         FROM disease_terms
         WHERE mondo_id = $1 AND is_obsolete = FALSE",
    )
    .bind(mondo_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| DiseaseRow {
        id: r.get("id"),
        mondo_id: r.get("mondo_id"),
        name: r.get("name"),
        definition: r.get("definition"),
        omim_id: r.get("omim_id"),
        orphanet_id: r.get("orphanet_id"),
        mondo_release: r.get("mondo_release"),
    }))
}

pub async fn get_disease_synonyms(pool: &PgPool, disease_id: Uuid) -> sqlx::Result<Vec<DiseaseSynonymRow>> {
    let rows = sqlx::query(
        "SELECT scope, text FROM disease_term_synonyms WHERE term_id = $1",
    )
    .bind(disease_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| DiseaseSynonymRow { scope: r.get("scope"), text: r.get("text") })
        .collect())
}

pub async fn get_disease_xrefs(pool: &PgPool, disease_id: Uuid) -> sqlx::Result<Vec<DiseaseXrefRow>> {
    let rows = sqlx::query(
        "SELECT source_db, source_id FROM disease_term_xrefs WHERE term_id = $1",
    )
    .bind(disease_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| DiseaseXrefRow { source_db: r.get("source_db"), source_id: r.get("source_id") })
        .collect())
}

#[derive(Debug)]
pub struct DiseasePhenotypeRow {
    pub hpo_id: String,
    pub hpo_name: String,
    pub frequency: Option<String>,
    pub onset: Option<String>,
    pub evidence: Option<String>,
    pub reference: Option<String>,
}

/// Fetch phenotype annotations for a disease.
/// IMPORTANT: disease_phenotype_annotations uses OMIM/Orphanet IDs, not MONDO IDs.
/// The join bridges through disease_terms.omim_id and disease_terms.orphanet_id.
pub async fn get_disease_phenotypes(
    pool: &PgPool,
    mondo_id: &str,
    offset: i64,
    limit: i64,
) -> sqlx::Result<Vec<DiseasePhenotypeRow>> {
    let rows = sqlx::query(
        r#"
        SELECT dpa.hpo_id, h.name AS hpo_name,
               dpa.frequency, dpa.onset, dpa.evidence, dpa.reference
        FROM disease_terms dt
        JOIN disease_phenotype_annotations dpa ON (
            (dt.omim_id IS NOT NULL    AND dpa.disease_db = 'OMIM'  AND dpa.disease_id = dt.omim_id)
            OR
            (dt.orphanet_id IS NOT NULL AND dpa.disease_db = 'ORPHA' AND dpa.disease_id = dt.orphanet_id)
        )
        JOIN hpo_term_metadata h ON h.hpo_id = dpa.hpo_id
        WHERE dt.mondo_id = $1
        ORDER BY dpa.hpo_id
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(mondo_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| DiseasePhenotypeRow {
            hpo_id: r.get("hpo_id"),
            hpo_name: r.get("hpo_name"),
            frequency: r.get("frequency"),
            onset: r.get("onset"),
            evidence: r.get("evidence"),
            reference: r.get("reference"),
        })
        .collect())
}
```

- [ ] **Step 5: Create `lib.rs` with only the modules that exist at this point**

Create `crates/bdp-mcp/src/lib.rs`. **Only declare `config` and `db`** — `server` and `tools` don't exist yet and would cause a compile error if declared now. They will be added in Tasks 7 and 10.

```rust
// crates/bdp-mcp/src/lib.rs
// Library target so integration tests can import db::queries.
// Add pub mod server in Task 7; pub mod tools in Task 10.

pub mod config;
pub mod db;
```

Update `Cargo.toml` to add the lib target:

```toml
[lib]
name = "bdp_mcp"
path = "src/lib.rs"
```

- [ ] **Step 6: Run the integration test**

```bash
cargo test -p bdp-mcp test_get_disease -- --nocapture --ignored
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/bdp-mcp/
git commit -m "feat(mcp): add disease DB queries + integration test scaffold"
```

---

## Task 5: Phenotype, Gene, Pathway, Compound Queries

**Files:**
- Modify: `crates/bdp-mcp/src/db/queries.rs`
- Modify: `crates/bdp-mcp/tests/integration.rs`

Follow the same pattern as Task 4. Add one integration test per major query.

- [ ] **Step 1: Add phenotype queries to `queries.rs`**

```rust
// ─── Phenotype ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PhenotypeRow {
    pub id: Uuid,
    pub hpo_id: String,
    pub name: String,
    pub definition: Option<String>,
    pub synonyms_json: Option<serde_json::Value>,
    pub alt_ids_json: Option<serde_json::Value>,
}

pub async fn get_phenotype(pool: &PgPool, hpo_id: &str) -> sqlx::Result<Option<PhenotypeRow>> {
    let row = sqlx::query(
        "SELECT id, hpo_id, name, definition, synonyms, alt_ids
         FROM hpo_term_metadata
         WHERE hpo_id = $1 AND is_obsolete = FALSE",
    )
    .bind(hpo_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| PhenotypeRow {
        id: r.get("id"),
        hpo_id: r.get("hpo_id"),
        name: r.get("name"),
        definition: r.get("definition"),
        synonyms_json: r.get("synonyms"),   // JSONB column
        alt_ids_json: r.get("alt_ids"),     // JSONB column
    }))
}

#[derive(Debug)]
pub struct PhenotypesDiseaseRow {
    pub mondo_id: String,
    pub name: String,
    pub definition: Option<String>,
}

/// Reverse bridge: find diseases annotated with a given HPO term.
pub async fn get_phenotype_diseases(
    pool: &PgPool,
    hpo_id: &str,
    offset: i64,
    limit: i64,
) -> sqlx::Result<Vec<PhenotypesDiseaseRow>> {
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT dt.mondo_id, dt.name, dt.definition
        FROM disease_phenotype_annotations dpa
        JOIN disease_terms dt ON (
            (dpa.disease_db = 'OMIM'  AND dt.omim_id      = dpa.disease_id)
            OR
            (dpa.disease_db = 'ORPHA' AND dt.orphanet_id  = dpa.disease_id)
        )
        WHERE dpa.hpo_id = $1
          AND dt.is_obsolete = FALSE
        ORDER BY dt.name
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(hpo_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| PhenotypesDiseaseRow {
            mondo_id: r.get("mondo_id"),
            name: r.get("name"),
            definition: r.get("definition"),
        })
        .collect())
}
```

- [ ] **Step 2: Add gene queries**

```rust
// ─── Gene ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct GeneRow {
    pub uniprot_acc: String,
    pub entry_name: Option<String>,
    pub gene_name: Option<String>,
    pub organism: Option<String>,          // taxonomy_metadata.scientific_name
    pub ncbi_taxon_id: Option<i64>,        // taxonomy_metadata.taxon_id
    pub sequence_length: Option<i32>,
}

/// Fetch gene by UniProt accession.
/// Join path: protein_metadata → taxonomy_metadata (direct via data_source_id).
pub async fn get_gene_by_uniprot(pool: &PgPool, accession: &str) -> sqlx::Result<Option<GeneRow>> {
    let row = sqlx::query(
        r#"
        SELECT pm.accession AS uniprot_acc, pm.entry_name, pm.gene_name,
               tm.scientific_name AS organism, tm.taxonomy_id AS ncbi_taxon_id,
               pm.sequence_length
        FROM protein_metadata pm
        LEFT JOIN taxonomy_metadata tm ON tm.data_source_id = pm.taxonomy_id
        WHERE pm.accession = $1
        "#,
    )
    .bind(accession)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| GeneRow {
        uniprot_acc: r.get("uniprot_acc"),
        entry_name: r.get("entry_name"),
        gene_name: r.get("gene_name"),
        organism: r.get("organism"),
        ncbi_taxon_id: r.get("ncbi_taxon_id"),
        sequence_length: r.get("sequence_length"),
    }))
}

#[derive(Debug)]
pub struct GenePathwayRow {
    pub reactome_id: String,
    pub name: String,
    pub species_name: String,
    pub is_top_level: bool,
}

pub async fn get_gene_pathways(
    pool: &PgPool,
    uniprot_acc: &str,
    offset: i64,
    limit: i64,
) -> sqlx::Result<Vec<GenePathwayRow>> {
    let rows = sqlx::query(
        r#"
        SELECT pt.reactome_id, pt.name, pt.species_name, pt.is_top_level
        FROM protein_pathway_associations ppa
        JOIN pathway_terms pt ON pt.id = ppa.pathway_id
        WHERE ppa.uniprot_acc = $1
        ORDER BY pt.name
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(uniprot_acc)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| GenePathwayRow {
            reactome_id: r.get("reactome_id"),
            name: r.get("name"),
            species_name: r.get("species_name"),
            is_top_level: r.get("is_top_level"),
        })
        .collect())
}
```

- [ ] **Step 3: Add pathway + compound queries**

```rust
// ─── Pathway ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PathwayRow {
    pub reactome_id: String,
    pub name: String,
    pub species_name: String,
    pub is_top_level: bool,
    pub reactome_release: String,
}

pub async fn get_pathway(pool: &PgPool, reactome_id: &str) -> sqlx::Result<Option<PathwayRow>> {
    let row = sqlx::query(
        "SELECT reactome_id, name, species_name, is_top_level, reactome_release
         FROM pathway_terms WHERE reactome_id = $1",
    )
    .bind(reactome_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| PathwayRow {
        reactome_id: r.get("reactome_id"),
        name: r.get("name"),
        species_name: r.get("species_name"),
        is_top_level: r.get("is_top_level"),
        reactome_release: r.get("reactome_release"),
    }))
}

#[derive(Debug)]
pub struct PathwayProteinRow {
    pub uniprot_acc: String,
    pub evidence_type: Option<String>,
}

pub async fn get_pathway_proteins(
    pool: &PgPool,
    reactome_id: &str,
    offset: i64,
    limit: i64,
) -> sqlx::Result<Vec<PathwayProteinRow>> {
    let rows = sqlx::query(
        r#"
        SELECT ppa.uniprot_acc, ppa.evidence_type
        FROM protein_pathway_associations ppa
        JOIN pathway_terms pt ON pt.id = ppa.pathway_id
        WHERE pt.reactome_id = $1
        ORDER BY ppa.uniprot_acc
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(reactome_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| PathwayProteinRow {
            uniprot_acc: r.get("uniprot_acc"),
            evidence_type: r.get("evidence_type"),
        })
        .collect())
}

// ─── Compound ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct CompoundRow {
    pub chebi_id: String,
    pub name: String,
    pub definition: Option<String>,
    pub formula: Option<String>,
    pub inchikey: Option<String>,
    pub smiles: Option<String>,
    pub mass_mono: Option<f64>,
    pub charge: Option<i32>,
}

pub async fn get_compound(pool: &PgPool, chebi_id: &str) -> sqlx::Result<Option<CompoundRow>> {
    let row = sqlx::query(
        "SELECT chebi_id, name, definition, formula, inchikey, smiles, mass_mono, charge
         FROM compound_terms WHERE chebi_id = $1 AND is_obsolete = FALSE",
    )
    .bind(chebi_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| CompoundRow {
        chebi_id: r.get("chebi_id"),
        name: r.get("name"),
        definition: r.get("definition"),
        formula: r.get("formula"),
        inchikey: r.get("inchikey"),
        smiles: r.get("smiles"),
        mass_mono: r.get("mass_mono"),
        charge: r.get("charge"),
    }))
}

#[derive(Debug)]
pub struct CompoundRoleRow {
    pub chebi_id: String,
    pub name: String,
    pub relationship_type: String,
}

/// Fetch roles of a compound (e.g. "has_role" → "anti-inflammatory agent").
/// object_chebi_id = the role; subject_chebi_id = the queried compound.
pub async fn get_compound_roles(
    pool: &PgPool,
    chebi_id: &str,
    offset: i64,
    limit: i64,
) -> sqlx::Result<Vec<CompoundRoleRow>> {
    let rows = sqlx::query(
        r#"
        SELECT cr.object_chebi_id AS chebi_id, ct.name, cr.relationship_type
        FROM compound_relationships cr
        JOIN compound_terms ct ON ct.chebi_id = cr.object_chebi_id
        WHERE cr.subject_chebi_id = $1
          AND cr.relationship_type = 'has_role'
          AND ct.is_obsolete = FALSE
        ORDER BY ct.name
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(chebi_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| CompoundRoleRow {
            chebi_id: r.get("chebi_id"),
            name: r.get("name"),
            relationship_type: r.get("relationship_type"),
        })
        .collect())
}
```

- [ ] **Step 4: Add integration tests for key queries**

Add to `tests/integration.rs`:

```rust
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_get_disease_phenotypes_bridge_join() {
    // Verifies the OMIM bridge join works correctly.
    let pg = common::TestPostgres::start().await.unwrap();
    // ... seed disease + phenotype annotation + hpo term, then query
    // See spec: get_disease_phenotypes requires OMIM/Orphanet bridge
    let rows = bdp_mcp::db::queries::get_disease_phenotypes(
        &pg.pool, "MONDO:0004975", 0, 50
    ).await.unwrap();
    assert!(!rows.is_empty());
    assert_eq!(rows[0].hpo_id.starts_with("HP:"), true);
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_resolve_and_query_roundtrip() {
    let pg = common::TestPostgres::start().await.unwrap();
    // seed data, then test that resolve_entity + get_disease return consistent results
    let id = bdp_mcp::db::resolve::disease_by_mondo_id(&pg.pool, "MONDO:0004975")
        .await.unwrap();
    assert!(id.is_some());
}
```

- [ ] **Step 5: Run all tests**

```bash
cargo test -p bdp-mcp -- --ignored
```

Expected: integration tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/bdp-mcp/
git commit -m "feat(mcp): add phenotype/gene/pathway/compound DB queries"
```

---

## Task 6: Audit Logging

**Files:**
- Modify: `crates/bdp-mcp/src/db/audit.rs`

- [ ] **Step 1: Write failing test**

```rust
// In audit.rs cfg(test):
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_audit_log_writes() {
    // verify INSERT succeeds and row appears
}

#[test]
fn test_audit_swallows_error() {
    // log_tool_call with a dead pool should not panic
}
```

- [ ] **Step 2: Implement `audit.rs`**

```rust
// crates/bdp-mcp/src/db/audit.rs

use serde_json::Value;
use sqlx::PgPool;
use tracing::warn;

pub struct AuditEntry<'a> {
    pub agent_id: Option<&'a str>,
    pub tool_name: &'a str,
    pub query_params: Value,
    pub dataset_versions: Value,
    pub result_count: Option<i32>,
    pub duration_ms: Option<i32>,
}

/// Write a tool call to agent_query_log.
/// If the INSERT fails, logs a warning and returns — never propagates to the caller.
pub async fn log_tool_call(pool: &PgPool, entry: AuditEntry<'_>) {
    let result = sqlx::query(
        "INSERT INTO agent_query_log
         (agent_id, tool_name, query_params, dataset_versions, result_count, duration_ms)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(entry.agent_id.unwrap_or("anonymous"))
    .bind(entry.tool_name)
    .bind(&entry.query_params)
    .bind(&entry.dataset_versions)
    .bind(entry.result_count)
    .bind(entry.duration_ms)
    .execute(pool)
    .await;

    if let Err(e) = result {
        warn!(tool = entry.tool_name, error = %e, "audit write failed — tool result unaffected");
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-mcp/src/db/audit.rs
git commit -m "feat(mcp): add audit logging (swallows errors)"
```

---

## Task 7: Server Struct + Common Types

**Files:**
- Create: `crates/bdp-mcp/src/server.rs`
- Create: `crates/bdp-mcp/src/tools/mod.rs` (stub)

This is where `BdpMcpServer` is defined. The rmcp macros (`#[tool_router]`, `#[tool_handler]`) are applied here.

- [ ] **Step 1: Create `server.rs` with one stub tool to verify rmcp compiles**

```rust
// crates/bdp-mcp/src/server.rs

use rmcp::{
    handler::server::tool::Parameters,
    model::{CallToolResult, Content, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

/// Shared server state — holds DB pool.
#[derive(Clone)]
pub struct BdpMcpServer {
    pool: Arc<PgPool>,
}

impl BdpMcpServer {
    pub fn new(pool: PgPool) -> Self {
        Self { pool: Arc::new(pool) }
    }
}

// ─── Stub tool to verify rmcp macro wiring ────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PingParams {
    /// Optional message to echo back
    pub message: Option<String>,
}

#[tool_router]
impl BdpMcpServer {
    #[tool(description = "Health check — returns 'pong'. Use to verify server is running.")]
    async fn ping(
        &self,
        Parameters(params): Parameters<PingParams>,
    ) -> Result<CallToolResult, rmcp::Error> {
        let msg = params.message.as_deref().unwrap_or("pong");
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }
}

#[tool_handler]
impl ServerHandler for BdpMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "bdp-mcp".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            ..Default::default()
        }
    }
}
```

- [ ] **Step 2: Create stub `tools/mod.rs`**

```rust
// crates/bdp-mcp/src/tools/mod.rs
// Tool modules — added in Tasks 8–14
```

- [ ] **Step 3: Update `lib.rs` — add `server` and `tools` now that both exist**

```rust
pub mod config;
pub mod db;
pub mod server;
pub mod tools;
```

- [ ] **Step 4: Verify rmcp macros compile**

```bash
cargo build -p bdp-mcp 2>&1 | head -30
```

Expected: compiles. If rmcp feature errors appear, check the `Cargo.toml` features list matches Task 1.

- [ ] **Step 5: Commit**

```bash
git add crates/bdp-mcp/src/
git commit -m "feat(mcp): add BdpMcpServer with rmcp tool_router + ping tool"
```

---

## Task 8: stdio Transport (BDP-91 milestone)

**Files:**
- Modify: `crates/bdp-mcp/src/main.rs`

This task makes the server runnable and testable with Claude Desktop.

- [ ] **Step 1: Wire stdio transport in `main.rs`**

```rust
// crates/bdp-mcp/src/main.rs

mod config;
mod db;
mod server;
mod tools;

use clap::Parser;
use config::{Config, Transport};
use server::BdpMcpServer;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_env("RUST_LOG")
                .add_directive("bdp_mcp=info".parse()?),
        )
        .init();

    let cfg = Config::parse();

    let pool = PgPoolOptions::new()
        .max_connections(cfg.db_max_connections)
        .connect(&cfg.database_url)
        .await?;

    tracing::info!(transport = ?cfg.transport, "bdp-mcp starting");

    match cfg.transport {
        Transport::Stdio => {
            use rmcp::ServiceExt;
            let server = BdpMcpServer::new(pool);
            server.serve(rmcp::transport::stdio()).await?;
        }
        Transport::Http => {
            anyhow::bail!("HTTP transport not yet implemented — use --transport stdio");
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Build release binary**

```bash
cargo build -p bdp-mcp --release
```

Expected: produces `target/release/bdp-mcp.exe` (Windows) or `target/release/bdp-mcp` (Linux).

- [ ] **Step 3: Test with Claude Desktop**

Add to Claude Desktop's `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "bdp": {
      "command": "D:/dev/datadir/bdp/target/release/bdp-mcp",
      "args": ["--transport", "stdio"],
      "env": {
        "DATABASE_URL": "postgresql://bdp:bdp_dev_password@localhost:5432/bdp"
      }
    }
  }
}
```

Restart Claude Desktop. Expected: "bdp" appears in the MCP tools panel. Ask Claude: "Use the ping tool." Expected: "pong".

- [ ] **Step 4: Commit**

```bash
git add crates/bdp-mcp/src/main.rs
git commit -m "feat(mcp): wire stdio transport — server runnable with Claude Desktop"
```

---

## Task 9: Disease Tools

**Files:**
- Create: `crates/bdp-mcp/src/tools/diseases.rs`
- Modify: `crates/bdp-mcp/src/server.rs`

- [ ] **Step 1: Implement `diseases.rs`**

```rust
// crates/bdp-mcp/src/tools/diseases.rs

use rmcp::{handler::server::tool::Parameters, model::{CallToolResult, Content}};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;

use crate::db::{audit, queries, resolve};

// ─── Inputs ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDiseaseParams {
    /// MONDO ID (e.g. "MONDO:0004975") or disease name (e.g. "Alzheimer disease")
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDiseasePhenotypesParams {
    /// MONDO ID or disease name
    pub id: String,
    /// Pagination cursor (omit for first page)
    pub cursor: Option<String>,
    /// Results per page (default 50, max 200)
    pub limit: Option<i64>,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub fn decode_cursor(cursor: Option<&str>) -> i64 {
    cursor
        .and_then(|c| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, c).ok())
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v["offset"].as_i64())
        .unwrap_or(0)
}

pub fn encode_cursor(offset: i64) -> String {
    let json = serde_json::json!({"offset": offset}).to_string();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, json)
}

pub fn clamp_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(50).min(200).max(1)
}

// ─── Tool implementations ─────────────────────────────────────────────────────

pub struct DiseaseTools;

impl DiseaseTools {
    pub async fn get_disease(
        pool: &sqlx::PgPool,
        params: GetDiseaseParams,
        agent_id: Option<&str>,
    ) -> Result<CallToolResult, rmcp::Error> {
        let start = Instant::now();
        let input = resolve::cap_input(&params.id);

        // 1. Resolve entity to MONDO ID
        let mondo_id = if let Some(resolve::CanonicalId::Mondo(id)) = resolve::detect_id_type(input) {
            id.to_string()
        } else {
            // FTS name search
            let matches = resolve::diseases_by_name(pool, input)
                .await
                .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))?;
            match matches.len() {
                0 => return Err(rmcp::Error::invalid_params(
                    format!("No disease matching '{input}'. Try a MONDO ID like 'MONDO:0004975'."),
                    None,
                )),
                1 => {
                    // Fetch by UUID to get mondo_id string
                    let d = queries::get_disease_by_id(pool, matches[0].id)
                        .await
                        .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))?
                        .ok_or_else(|| rmcp::Error::internal_error("resolve mismatch", None))?;
                    d.mondo_id
                }
                _ => {
                    let candidates: Vec<_> = matches.iter()
                        .map(|m| json!({"id": m.id, "name": m.name}))
                        .collect();
                    return Err(rmcp::Error::invalid_params(
                        format!("Ambiguous name '{input}'. Candidates: {:?}", candidates),
                        Some(json!({"candidates": candidates})),
                    ));
                }
            }
        };

        // 2. Fetch disease
        let disease = queries::get_disease(pool, &mondo_id)
            .await
            .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))?
            .ok_or_else(|| rmcp::Error::invalid_params(
                format!("Disease '{mondo_id}' not found"),
                None,
            ))?;

        let synonyms = queries::get_disease_synonyms(pool, disease.id)
            .await
            .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))?;

        let xrefs = queries::get_disease_xrefs(pool, disease.id)
            .await
            .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))?;

        let duration_ms = start.elapsed().as_millis() as i32;

        // 3. Audit
        audit::log_tool_call(pool, audit::AuditEntry {
            agent_id,
            tool_name: "get_disease",
            query_params: json!({"id": params.id}),
            dataset_versions: json!({"mondo": disease.mondo_release}),
            result_count: Some(1),
            duration_ms: Some(duration_ms),
        }).await;

        // 4. Return dual-format result
        let text = format!(
            "Disease: {} ({})\nDefinition: {}\nOMIM: {}\nOrphanet: {}\nSynonyms: {}",
            disease.name,
            disease.mondo_id,
            disease.definition.as_deref().unwrap_or("N/A"),
            disease.omim_id.as_deref().unwrap_or("N/A"),
            disease.orphanet_id.as_deref().unwrap_or("N/A"),
            synonyms.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(", ")
        );

        let structured = json!({
            "mondo_id": disease.mondo_id,
            "name": disease.name,
            "definition": disease.definition,
            "omim_id": disease.omim_id,
            "orphanet_id": disease.orphanet_id,
            "synonyms": synonyms.iter().map(|s| json!({"scope": s.scope, "text": s.text})).collect::<Vec<_>>(),
            "xrefs": xrefs.iter().map(|x| json!({"source_db": x.source_db, "source_id": x.source_id})).collect::<Vec<_>>(),
            "_meta": { "datasets_used": [{"name": "mondo", "release": disease.mondo_release}], "duration_ms": duration_ms }
        });

        let mut result = CallToolResult::success(vec![Content::text(text)]);
        result.structured_content = Some(structured);
        Ok(result)
    }

    // get_disease_phenotypes: similar pattern with pagination
    // get_disease_genes: stub (see Task 13)
    // get_disease_trials: stub (see Task 13)
}
```

> **Note on `get_disease_by_id`**: Add a helper to `queries.rs` that fetches by UUID (for the resolve→FTS path):
> ```rust
> pub async fn get_disease_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<DiseaseRow>> {
>     // SELECT ... FROM disease_terms WHERE id = $1 AND is_obsolete = FALSE
> }
> ```

- [ ] **Step 2: Wire tools into `BdpMcpServer` in `server.rs`**

Import `DiseaseTools` in `server.rs` and add `#[tool]` methods that delegate to `DiseaseTools::get_disease`. The `#[tool_router]` impl block gets one method per MCP tool:

```rust
use crate::tools::diseases::{DiseaseTools, GetDiseaseParams, GetDiseasePhenotypesParams};

#[tool_router]
impl BdpMcpServer {
    // ... ping from Task 7 ...

    #[tool(description = "Look up a disease by MONDO ID or name. Returns definition, synonyms, cross-references. Example: get_disease('Alzheimer disease') or get_disease('MONDO:0004975')")]
    async fn get_disease(
        &self,
        Parameters(params): Parameters<GetDiseaseParams>,
    ) -> Result<CallToolResult, rmcp::Error> {
        DiseaseTools::get_disease(&self.pool, params, None).await
    }

    #[tool(description = "Get HPO phenotype annotations for a disease (what symptoms/signs are associated). Paginated. Example: get_disease_phenotypes('MONDO:0004975')")]
    async fn get_disease_phenotypes(
        &self,
        Parameters(params): Parameters<GetDiseasePhenotypesParams>,
    ) -> Result<CallToolResult, rmcp::Error> {
        DiseaseTools::get_disease_phenotypes(&self.pool, params, None).await
    }
}
```

- [ ] **Step 3: Build and manually test via Claude Desktop**

```bash
cargo build -p bdp-mcp --release
```

In Claude Desktop: "Look up Alzheimer disease using the get_disease tool."
Expected: structured response with MONDO ID, definition, synonyms.

- [ ] **Step 4: Commit**

```bash
git add crates/bdp-mcp/src/tools/ crates/bdp-mcp/src/server.rs crates/bdp-mcp/src/db/queries.rs
git commit -m "feat(mcp): add get_disease + get_disease_phenotypes tools"
```

---

## Task 10a: Phenotype + Compound Tools

**Files:**
- Create: `crates/bdp-mcp/src/tools/common.rs`
- Create: `crates/bdp-mcp/src/tools/phenotypes.rs`
- Create: `crates/bdp-mcp/src/tools/compounds.rs`
- Modify: `crates/bdp-mcp/src/server.rs`
- Modify: `crates/bdp-mcp/src/tools/mod.rs`

Follow the exact same pattern as `diseases.rs` in Task 9:
1. Write failing test first (unit or integration)
2. Implement input struct, `XxxTools` static method, wire into `#[tool_router]`
3. Confirm tests pass

- [ ] **Step 1: Create `tools/common.rs` with pagination helpers and stub builder**

```rust
// crates/bdp-mcp/src/tools/common.rs

use rmcp::model::{CallToolResult, Content};

pub fn decode_cursor(cursor: Option<&str>) -> i64 {
    cursor
        .and_then(|c| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, c).ok())
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v["offset"].as_i64())
        .unwrap_or(0)
}

pub fn encode_cursor(offset: i64) -> String {
    let json = serde_json::json!({"offset": offset}).to_string();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, json)
}

pub fn clamp_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(50).min(200).max(1)
}

/// Build a not_yet_available stub result (is_error: false — planned capability, not a failure).
pub fn stub_result(tool_name: &str, reason: &str, tracking: &str) -> CallToolResult {
    let payload = serde_json::json!({
        "status": "not_yet_available",
        "tool": tool_name,
        "reason": reason,
        "tracking": tracking,
        "expected": "2026-Q3"
    });
    let text = format!("{tool_name}: {reason} (tracked: {tracking})");
    let mut result = CallToolResult::success(vec![Content::text(text)]);
    result.structured_content = Some(payload);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_roundtrip() {
        let encoded = encode_cursor(50);
        assert_eq!(decode_cursor(Some(&encoded)), 50);
    }

    #[test]
    fn test_clamp_limit() {
        assert_eq!(clamp_limit(None), 50);
        assert_eq!(clamp_limit(Some(300)), 200);
        assert_eq!(clamp_limit(Some(0)), 1);
    }

    #[test]
    fn test_stub_result_is_not_error() {
        let r = stub_result("test_tool", "needs pipeline", "BDP-99");
        assert!(!r.is_error.unwrap_or(false));
        let s = r.structured_content.unwrap();
        assert_eq!(s["status"], "not_yet_available");
        assert_eq!(s["tracking"], "BDP-99");
    }
}
```

- [ ] **Step 2: Run tests to confirm they pass**

```bash
cargo test -p bdp-mcp tools::common
```

Expected: 3 tests PASS.

- [ ] **Step 3: Write failing tests for phenotype tools**

Add to `tests/integration.rs`:

```rust
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_get_phenotype_by_hpo_id() {
    let pg = common::TestPostgres::start().await.unwrap();
    // seed hpo_term_metadata row
    // call bdp_mcp::tools::phenotypes::PhenotypeTools::get_phenotype
    // assert hpo_id matches
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_get_phenotype_diseases_reverse_bridge() {
    // seed disease + phenotype annotation (OMIM bridge), then
    // assert get_phenotype_diseases returns the seeded disease
    // verifies is_obsolete = FALSE filter works
}
```

- [ ] **Step 4: Implement `phenotypes.rs`**

Follow `diseases.rs` pattern. Key difference — `synonyms`/`alt_ids` are JSONB:

```rust
// In get_phenotype tool handler:
let synonyms: Vec<String> = row.synonyms_json
    .and_then(|v| serde_json::from_value(v).ok())
    .unwrap_or_default();
let alt_ids: Vec<String> = row.alt_ids_json
    .and_then(|v| serde_json::from_value(v).ok())
    .unwrap_or_default();
```

- [ ] **Step 5: Run phenotype integration tests**

```bash
cargo test -p bdp-mcp test_get_phenotype -- --ignored
```

Expected: PASS.

- [ ] **Step 6: Write failing tests for compound tools**

Add to `tests/integration.rs`:

```rust
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_get_compound_by_chebi_id() {
    // seed compound_terms, assert get_compound returns name + formula
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_get_compound_roles() {
    // seed compound_terms + compound_relationships (has_role), assert roles returned
}
```

- [ ] **Step 7: Implement `compounds.rs`**

`get_compound` — straight fetch from `compound_terms`.
`get_compound_roles` — join `compound_relationships` → `compound_terms` on `object_chebi_id`.

- [ ] **Step 8: Wire phenotype + compound tools into `server.rs`**

Add `#[tool]` methods for: `get_phenotype`, `get_phenotype_diseases`, `get_compound`, `get_compound_roles`.

- [ ] **Step 9: Build + smoke test**

```bash
cargo build -p bdp-mcp --release
```

Test via Claude Desktop: "What phenotypes are associated with HP:0000545?" and "What is CHEBI:15422?"

- [ ] **Step 10: Commit**

```bash
git add crates/bdp-mcp/src/tools/ crates/bdp-mcp/src/server.rs
git commit -m "feat(mcp): add phenotype + compound tools"
```

---

## Task 10b: Gene + Pathway Tools

**Files:**
- Create: `crates/bdp-mcp/src/tools/genes.rs`
- Create: `crates/bdp-mcp/src/tools/pathways.rs`
- Modify: `crates/bdp-mcp/src/server.rs`

- [ ] **Step 1: Write failing tests for gene tools**

```rust
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_get_gene_by_uniprot_accession() {
    // seed protein_metadata + taxonomy_metadata row (direct join via data_source_id)
    // assert get_gene returns organism and ncbi_taxon_id
    // verifies taxonomy_metadata.taxonomy_id (not taxon_id) join works
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_get_gene_by_symbol_via_entity_aliases() {
    // seed entity_aliases with alias_db='symbol', alias_id='BRCA1', canonical_id='P38398'
    // call get_gene("BRCA1") — should resolve via entity_aliases
}
```

- [ ] **Step 2: Implement `genes.rs`**

Entity resolution order:
1. UniProt accession pattern (regex) → `get_gene_by_uniprot` directly
2. Numeric → `entity_aliases` where `alias_db='entrez_gene'`
3. Otherwise → `entity_aliases` where `alias_db='symbol'` THEN FTS on `protein_metadata.gene_name`

Join for organism: `protein_metadata pm JOIN taxonomy_metadata tm ON tm.data_source_id = pm.taxonomy_id`, fetch `tm.taxonomy_id AS ncbi_taxon_id` (column is `taxonomy_id`, INTEGER — NCBI taxon ID).

- [ ] **Step 3: Run gene integration tests**

```bash
cargo test -p bdp-mcp test_get_gene -- --ignored
```

Expected: PASS.

- [ ] **Step 4: Write failing tests for pathway tools**

```rust
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_get_pathway_by_reactome_id() {
    // seed pathway_terms, assert get_pathway returns species_name field
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_get_pathway_proteins_pagination() {
    // seed 3 protein_pathway_associations, fetch with limit=2, assert next_cursor present
}
```

- [ ] **Step 5: Implement `pathways.rs`**

Follow `diseases.rs` pattern. Note: `species_name` (not `species`) is the column name in `pathway_terms`.

- [ ] **Step 6: Run pathway integration tests**

```bash
cargo test -p bdp-mcp test_get_pathway -- --ignored
```

Expected: PASS.

- [ ] **Step 7: Wire gene + pathway tools into `server.rs`**

Add `#[tool]` methods: `get_gene`, `get_gene_pathways`.

- [ ] **Step 8: Build + smoke test**

```bash
cargo build -p bdp-mcp --release
```

Test via Claude Desktop: "What pathways is P38398 involved in?"

- [ ] **Step 9: Commit**

```bash
git add crates/bdp-mcp/src/tools/ crates/bdp-mcp/src/server.rs
git commit -m "feat(mcp): add gene + pathway tools"
```

---

## Task 11: Stub Tools + Traversal

**Files:**
- Create: `crates/bdp-mcp/src/tools/literature.rs`
- Create: `crates/bdp-mcp/src/tools/traversal.rs`
- Modify: `crates/bdp-mcp/src/server.rs`

- [ ] **Step 1: Implement all stubs using `common::stub_result`**

`compounds.rs` — add stub functions for the two not-yet-available compound tools:
```rust
pub fn get_compound_targets_stub() -> CallToolResult {
    common::stub_result(
        "get_compound_targets",
        "Requires ChEMBL pipeline (BDP-80). Will return drug-target bioactivity data.",
        "BDP-80",
    )
}

pub fn get_compound_trials_stub() -> CallToolResult {
    common::stub_result(
        "get_compound_trials",
        "Requires ClinicalTrials.gov pipeline (BDP-89). Will return clinical trial evidence.",
        "BDP-89",
    )
}
```

`literature.rs`:
```rust
pub fn search_literature_stub() -> CallToolResult {
    common::stub_result(
        "search_literature",
        "Requires PubMed pipeline (BDP-84). Literature ingestion is planned for 2026-Q3.",
        "BDP-84",
    )
}

pub fn get_publication_stub() -> CallToolResult {
    common::stub_result("get_publication", "Requires PubMed pipeline (BDP-84).", "BDP-84")
}
```

`traversal.rs`:
```rust
// traverse is partially live — dispatch based on path
pub async fn traverse(...) -> Result<CallToolResult, rmcp::Error> {
    match path_key(&params.path) {
        "gene->pathway" => { /* call get_gene_pathways */ }
        "disease->phenotype" => { /* call get_disease_phenotypes */ }
        "phenotype->disease" => { /* call get_phenotype_diseases */ }
        "compound->role" => { /* call get_compound_roles */ }
        _ => Ok(common::stub_result("traverse", "Path not yet supported", "BDP-90"))
    }
}
```

- [ ] **Step 2: Wire all stubs into `server.rs`**

Add stub `#[tool]` methods for every stub in the spec table. Descriptions must tell the agent WHY it's unavailable and when it will be ready:

```rust
#[tool(description = "Get drug targets for a compound. NOT YET AVAILABLE — requires ChEMBL pipeline (BDP-80, 2026-Q3). Will return binding targets, IC50/EC50 activity values.")]
async fn get_compound_targets(&self, Parameters(_): Parameters<GetCompoundParams>) -> Result<CallToolResult, rmcp::Error> {
    Ok(common::stub_result("get_compound_targets", "Requires ChEMBL (BDP-80)", "BDP-80"))
}

#[tool(description = "Get clinical trials for a compound. NOT YET AVAILABLE — requires ClinicalTrials.gov pipeline (BDP-89, 2026-Q3).")]
async fn get_compound_trials(&self, Parameters(_): Parameters<GetCompoundParams>) -> Result<CallToolResult, rmcp::Error> {
    Ok(common::stub_result("get_compound_trials", "Requires ClinicalTrials.gov (BDP-89)", "BDP-89"))
}

#[tool(description = "Search biomedical literature by query or entity. NOT YET AVAILABLE — requires PubMed pipeline (BDP-84, 2026-Q3). Will return papers with abstracts, authors, MeSH terms.")]
async fn search_literature(
    &self,
    Parameters(_): Parameters<SearchLiteratureParams>,
) -> Result<CallToolResult, rmcp::Error> {
    Ok(common::stub_result("search_literature", "Requires PubMed pipeline (BDP-84)", "BDP-84"))
}
```

Ensure all 8 stubs from the spec table are wired: `get_gene_diseases`, `get_disease_genes`, `get_disease_trials`, `get_compound_targets`, `get_compound_trials`, `search_literature`, `get_publication`, `find_connection`.

- [ ] **Step 3: Verify `tools/list` returns all tools**

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}' | \
  DATABASE_URL=postgresql://bdp:bdp_dev_password@localhost:5432/bdp \
  ./target/release/bdp-mcp --transport stdio
```

(Pipe more JSON-RPC messages to list tools and verify count.)

- [ ] **Step 4: Commit**

```bash
git add crates/bdp-mcp/src/tools/
git commit -m "feat(mcp): add stub tools + partial traverse"
```

---

## Task 12: Streamable HTTP Transport

**Files:**
- Modify: `crates/bdp-mcp/src/main.rs`

- [ ] **Step 1: Implement HTTP branch in `main.rs`**

```rust
Transport::Http => {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    };
    use axum::{Router, routing::get};

    let session_manager = LocalSessionManager::default();
    let mcp_service = StreamableHttpService::new(
        move || Ok(BdpMcpServer::new(pool.clone())),
        session_manager,
        Default::default(),
    );

    let app = Router::new()
        .route("/mcp", mcp_service.into_router())
        .route("/health", get(|| async { "ok" }));

    let addr = format!("127.0.0.1:{}", cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(addr, "bdp-mcp HTTP listening");
    axum::serve(listener, app).await?;
}
```

- [ ] **Step 2: Build and test HTTP transport**

```bash
cargo build -p bdp-mcp --release

# Start HTTP server
DATABASE_URL=postgresql://bdp:bdp_dev_password@localhost:5432/bdp \
  ./target/release/bdp-mcp --transport http --port 3000

# Test health endpoint
curl http://127.0.0.1:3000/health
# Expected: "ok"

# Test MCP initialize via POST
curl -X POST http://127.0.0.1:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}'
```

Expected: JSON response with `serverInfo.name: "bdp-mcp"`.

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-mcp/src/main.rs
git commit -m "feat(mcp): add Streamable HTTP transport on /mcp"
```

---

## Task 13: Final Integration Tests + CI

**Files:**
- Modify: `crates/bdp-mcp/tests/integration.rs`

- [ ] **Step 1: Add end-to-end MCP protocol test**

Test that the full `initialize → tools/list → tools/call(get_disease)` flow works against a test DB:

```rust
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_mcp_get_disease_roundtrip() {
    let pg = common::TestPostgres::start().await.unwrap();
    // seed disease data
    let server = bdp_mcp::server::BdpMcpServer::new(pg.pool.clone());
    // call get_disease tool directly via server method
    let params = bdp_mcp::tools::diseases::GetDiseaseParams { id: "MONDO:0004975".into() };
    let result = bdp_mcp::tools::diseases::DiseaseTools::get_disease(
        &pg.pool, params, Some("test-agent")
    ).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
    let structured = result.structured_content.unwrap();
    assert_eq!(structured["mondo_id"], "MONDO:0004975");
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_stub_returns_not_yet_available() {
    use bdp_mcp::tools::common;
    let result = common::stub_result("search_literature", "needs PubMed", "BDP-84");
    let s = result.structured_content.unwrap();
    assert_eq!(s["status"], "not_yet_available");
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_audit_log_written_on_tool_call() {
    let pg = common::TestPostgres::start().await.unwrap();
    // seed + call get_disease + verify agent_query_log has 1 row
    let count_before = pg.count_rows("agent_query_log").await.unwrap();
    // ... call tool ...
    let count_after = pg.count_rows("agent_query_log").await.unwrap();
    assert_eq!(count_after, count_before + 1);
}
```

- [ ] **Step 2: Run full test suite**

```bash
cargo test -p bdp-mcp -- --ignored
```

Expected: all tests pass.

- [ ] **Step 3: Run clippy + fmt**

```bash
cargo clippy -p bdp-mcp -- -D warnings
cargo fmt -p bdp-mcp -- --check
```

Fix any issues.

- [ ] **Step 4: Final commit**

```bash
git add crates/bdp-mcp/
git commit -m "feat(mcp): complete bdp-mcp — all tools, stdio + HTTP, integration tests (BDP-90, BDP-91)"
```

---

## Verification Checklist (BDP-91 Acceptance Criteria)

- [ ] `cargo build -p bdp-mcp` compiles clean with no warnings
- [ ] `cargo test -p bdp-mcp` (unit tests) all pass without Docker
- [ ] `cargo test -p bdp-mcp -- --ignored` (integration tests) all pass with Docker
- [ ] stdio transport: server starts, responds to `ping` tool in Claude Desktop
- [ ] `tools/list` returns all tools (live + stubs)
- [ ] `get_disease("Alzheimer disease")` returns MONDO data from live PostgreSQL
- [ ] `get_disease_phenotypes("MONDO:0004975")` returns HPO annotations (OMIM bridge join works)
- [ ] Stub tools return `{"status": "not_yet_available"}` with `is_error: false`
- [ ] `agent_query_log` has a row after each tool call
- [ ] Streamable HTTP on `POST /mcp` responds to `initialize` request
- [ ] `GET /health` returns "ok"
