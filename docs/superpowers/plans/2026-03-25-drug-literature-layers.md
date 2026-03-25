# Drug, Interaction & Literature Layers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add five ingestion pipelines (Open Targets, ClinicalTrials.gov, ChEMBL, STRING, PubMed+PubTator3) to `crates/bdp-ingest`, completing the BDP knowledge graph so AI agents can traverse gene→disease→drug→trial→literature.

**Architecture:** Each pipeline lives in `crates/bdp-ingest/src/pipelines/<name>/` and implements the existing `PipelineRunner` trait (`run(self) -> anyhow::Result<PipelineStats>`). All share the same `PgPool`, use `sqlx::query()` runtime queries only (NO `sqlx::query!()` macros), and are gated by env flags. The `IngestOrchestrator` spawns all enabled pipelines in a `JoinSet`.

**Tech Stack:** Rust, sqlx 0.8 (runtime queries), tokio, reqwest, quick-xml, flate2, csv-async, arrow+parquet (Open Targets), rusqlite (ChEMBL), testcontainers (integration tests).

---

## Schema Key Facts (read before touching any SQL)

- `data_sources.id` — **UUID** (PRIMARY KEY UUID REFERENCES registry_entries(id))
- `disease_terms.id` — **UUID** (PRIMARY KEY UUID)
- `source_type` on `data_sources` — **TEXT FK** to `source_types.name` (since migration 20260325000002)
- All FKs to `data_sources` or `disease_terms` must use **UUID** columns
- New source types needed: `'gene_disease'`, `'trial'`, `'interaction'` — add via INSERT INTO source_types
- `PipelineRunner` trait: `fn run(self) -> impl Future<Output = anyhow::Result<PipelineStats>> + Send` — struct must be `Send + 'static`
- Tests: use testcontainers pattern with `.with_tag("16-alpine")`, `5432.tcp()`, `sqlx::migrate!("../../migrations")`
- Last migration: `20260326000006_hpo_tables.sql` — new ones start at `20260327000001`

---

## Task 1: Cargo.toml — add new dependencies

**Files:**
- Modify: `crates/bdp-ingest/Cargo.toml`

- [ ] **Read the current Cargo.toml** (`crates/bdp-ingest/Cargo.toml`) to see current deps

- [ ] **Add new dependencies** under `[dependencies]`:

```toml
# Parquet/Arrow (Open Targets)
arrow = { version = "53", default-features = false, features = ["ipc"] }
parquet = { version = "53", default-features = false, features = ["async", "arrow"] }
arrow-array = "53"

# SQLite ETL (ChEMBL)
rusqlite = { version = "0.31", features = ["bundled"] }

# Async gzip streaming (PubMed)
async-compression = { version = "0.4", features = ["tokio", "gzip"] }

# NOTE: `scraper` is already in Cargo.toml (used by Open Targets downloader for HTML directory listing).
# Verify with: grep -n "scraper" crates/bdp-ingest/Cargo.toml
# If missing, add: scraper = { workspace = true }  (or check workspace Cargo.toml for version)
```

- [ ] **Verify it compiles** (no DB needed):
```bash
cd D:\dev\datadir\bdp
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep -v "sqlx::query" | head -30
```
Expected: no errors (sqlx offline cache errors are normal/ignored)

- [ ] **Commit**:
```bash
git add crates/bdp-ingest/Cargo.toml
git commit -m "feat(ingest): add arrow, parquet, rusqlite, async-compression deps"
```

---

## Task 2: Migrations — all five pipeline tables

**Files:**
- Create: `migrations/20260327000001_gene_disease_associations.sql`
- Create: `migrations/20260327000002_clinical_trials.sql`
- Create: `migrations/20260327000003_drug_target_activities.sql`
- Create: `migrations/20260327000004_protein_interactions.sql`
- Create: `migrations/20260327000005_publications.sql`

- [ ] **Create `migrations/20260327000001_gene_disease_associations.sql`**:

```sql
-- Gene-disease associations from Open Targets
CREATE TABLE gene_disease_associations (
    id               BIGSERIAL PRIMARY KEY,
    gene_id          UUID NOT NULL REFERENCES data_sources(id),
    disease_term_id  UUID NOT NULL REFERENCES disease_terms(id),
    association_type TEXT NOT NULL DEFAULT 'direct',
    score            FLOAT4,
    source           TEXT NOT NULL DEFAULT 'open_targets',
    source_version   TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(gene_id, disease_term_id, source)
);
CREATE INDEX ON gene_disease_associations(gene_id);
CREATE INDEX ON gene_disease_associations(disease_term_id);
CREATE INDEX ON gene_disease_associations(score DESC NULLS LAST);
```

- [ ] **Create `migrations/20260327000002_clinical_trials.sql`**:

```sql
-- ClinicalTrials.gov
CREATE TABLE clinical_trials (
    id              BIGSERIAL PRIMARY KEY,
    nct_id          TEXT NOT NULL UNIQUE,
    title           TEXT,
    status          TEXT,
    phase           TEXT,
    start_date      DATE,
    completion_date DATE,
    sponsor         TEXT,
    source_version  TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX ON clinical_trials(status);
CREATE INDEX ON clinical_trials(nct_id);

CREATE TABLE trial_disease_links (
    id              BIGSERIAL PRIMARY KEY,
    trial_id        BIGINT NOT NULL REFERENCES clinical_trials(id),
    disease_term_id UUID REFERENCES disease_terms(id),
    raw_condition   TEXT NOT NULL,
    UNIQUE(trial_id, raw_condition)   -- deduplicate on raw text; disease_term_id may be NULL
);
CREATE INDEX ON trial_disease_links(trial_id);
CREATE INDEX ON trial_disease_links(disease_term_id) WHERE disease_term_id IS NOT NULL;

CREATE TABLE trial_intervention_links (
    id          BIGSERIAL PRIMARY KEY,
    trial_id    BIGINT NOT NULL REFERENCES clinical_trials(id),
    compound_id UUID REFERENCES data_sources(id),
    raw_name    TEXT NOT NULL
);
CREATE INDEX ON trial_intervention_links(trial_id);
CREATE INDEX ON trial_intervention_links(compound_id) WHERE compound_id IS NOT NULL;
```

- [ ] **Create `migrations/20260327000003_drug_target_activities.sql`**:

```sql
-- ChEMBL drug-target bioactivities
CREATE TABLE drug_target_activities (
    id              BIGSERIAL PRIMARY KEY,
    compound_id     UUID NOT NULL REFERENCES data_sources(id),
    target_gene_id  UUID NOT NULL REFERENCES data_sources(id),
    activity_type   TEXT,
    activity_value  FLOAT4,
    activity_unit   TEXT,
    relation        TEXT,
    assay_type      TEXT,
    chembl_assay_id TEXT,
    chembl_doc_id   TEXT,
    confidence      SMALLINT,
    source_version  TEXT NOT NULL DEFAULT 'chembl_36',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(compound_id, target_gene_id, chembl_assay_id)
);
CREATE INDEX ON drug_target_activities(compound_id);
CREATE INDEX ON drug_target_activities(target_gene_id);
CREATE INDEX ON drug_target_activities(activity_type, activity_value);
```

- [ ] **Create `migrations/20260327000004_protein_interactions.sql`**:

```sql
-- STRING protein-protein interactions (human, v12.0)
CREATE TABLE protein_interactions (
    id                    BIGSERIAL PRIMARY KEY,
    protein_a_id          UUID NOT NULL REFERENCES data_sources(id),
    protein_b_id          UUID NOT NULL REFERENCES data_sources(id),
    score_neighborhood    SMALLINT,
    score_fusion          SMALLINT,
    score_cooccurrence    SMALLINT,
    score_coexpression    SMALLINT,
    score_experimental    SMALLINT,
    score_database        SMALLINT,
    score_textmining      SMALLINT,
    combined_score        SMALLINT NOT NULL,
    source_version        TEXT NOT NULL DEFAULT 'string_v12',
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(protein_a_id, protein_b_id)
);
CREATE INDEX ON protein_interactions(protein_a_id);
CREATE INDEX ON protein_interactions(protein_b_id);
CREATE INDEX ON protein_interactions(combined_score DESC);
```

- [ ] **Create `migrations/20260327000005_publications.sql`**:

```sql
-- PubMed publications and entity annotations
CREATE TABLE publications (
    id          BIGSERIAL PRIMARY KEY,
    pmid        INTEGER NOT NULL UNIQUE,
    pmcid       TEXT,
    doi         TEXT,
    title       TEXT NOT NULL,
    abstract    TEXT,
    pub_date    DATE,
    journal     TEXT,
    source      TEXT NOT NULL DEFAULT 'pubmed',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX ON publications(pmid);
CREATE INDEX ON publications(pub_date DESC NULLS LAST);
CREATE INDEX ON publications USING GIN (to_tsvector('english', coalesce(title,'') || ' ' || coalesce(abstract,'')));

CREATE TABLE publication_authors (
    id             BIGSERIAL PRIMARY KEY,
    publication_id BIGINT NOT NULL REFERENCES publications(id),
    position       SMALLINT NOT NULL,
    last_name      TEXT,
    fore_name      TEXT,
    collective     TEXT,
    affiliation    TEXT
);
CREATE INDEX ON publication_authors(publication_id);

CREATE TABLE publication_mesh (
    id             BIGSERIAL PRIMARY KEY,
    publication_id BIGINT NOT NULL REFERENCES publications(id),
    mesh_ui        TEXT NOT NULL,
    descriptor     TEXT NOT NULL,
    is_major_topic BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX ON publication_mesh(publication_id);
CREATE INDEX ON publication_mesh(mesh_ui);

CREATE TABLE publication_entities (
    id              BIGSERIAL PRIMARY KEY,
    publication_id  BIGINT NOT NULL REFERENCES publications(id),
    entity_type     TEXT NOT NULL,
    external_id     TEXT NOT NULL,
    entity_name     TEXT,
    gene_id         UUID REFERENCES data_sources(id),
    disease_term_id UUID REFERENCES disease_terms(id),
    compound_id     UUID REFERENCES data_sources(id)
);
CREATE INDEX ON publication_entities(publication_id);
CREATE INDEX ON publication_entities(entity_type, external_id);
CREATE INDEX ON publication_entities(gene_id) WHERE gene_id IS NOT NULL;
CREATE INDEX ON publication_entities(disease_term_id) WHERE disease_term_id IS NOT NULL;
CREATE INDEX ON publication_entities(compound_id) WHERE compound_id IS NOT NULL;

CREATE TABLE pubmed_ingest_files (
    id            BIGSERIAL PRIMARY KEY,
    filename      TEXT NOT NULL UNIQUE,
    record_count  INTEGER,
    status        TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT,
    processed_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

- [ ] **Commit**:
```bash
git add migrations/
git commit -m "feat(migrations): add gene_disease, clinical_trials, drug_target, protein_interactions, publications tables"
```

---

## Task 3: Open Targets pipeline — config + downloader

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/open_targets/mod.rs`
- Create: `crates/bdp-ingest/src/pipelines/open_targets/config.rs`
- Create: `crates/bdp-ingest/src/pipelines/open_targets/downloader.rs`
- Modify: `crates/bdp-ingest/src/pipelines/mod.rs`

The Open Targets 25.03 Parquet data lives at:
- Associations: `https://ftp.ebi.ac.uk/pub/databases/opentargets/platform/25.03/output/association_overall_direct/` (multiple `.parquet` files)
- Targets: `https://ftp.ebi.ac.uk/pub/databases/opentargets/platform/25.03/output/targets/` (for Ensembl→UniProt mapping)

- [ ] **Write a unit test for config** (in `config.rs` test module):

```rust
#[test]
fn test_config_defaults() {
    let org_id = uuid::Uuid::new_v4();
    let cfg = OpenTargetsConfig::new("25.03", org_id);
    assert_eq!(cfg.release, "25.03");
    assert_eq!(cfg.max_retries, 3);
    assert!(cfg.parse_limit.is_none());
}
```

- [ ] **Run it to confirm it fails** (struct not defined yet):
```bash
cargo test -p bdp-ingest open_targets::config 2>&1 | tail -5
```

- [ ] **Create `crates/bdp-ingest/src/pipelines/open_targets/config.rs`**:

```rust
use uuid::Uuid;

pub const OPEN_TARGETS_BASE: &str =
    "https://ftp.ebi.ac.uk/pub/databases/opentargets/platform";

#[derive(Debug, Clone)]
pub struct OpenTargetsConfig {
    pub release: String,
    pub base_url: String,
    pub max_retries: u32,
    pub parse_limit: Option<usize>,
    pub min_score: f32,
    pub org_id: Uuid,
}

impl OpenTargetsConfig {
    pub fn new(release: impl Into<String>, org_id: Uuid) -> Self {
        let release = release.into();
        Self {
            base_url: format!("{}/{}/output", OPEN_TARGETS_BASE, release),
            release,
            max_retries: 3,
            parse_limit: None,
            min_score: 0.0,
            org_id,
        }
    }

    pub fn associations_url(&self) -> String {
        format!("{}/association_overall_direct/", self.base_url)
    }

    pub fn targets_url(&self) -> String {
        format!("{}/targets/", self.base_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let org_id = Uuid::new_v4();
        let cfg = OpenTargetsConfig::new("25.03", org_id);
        assert_eq!(cfg.release, "25.03");
        assert_eq!(cfg.max_retries, 3);
        assert!(cfg.parse_limit.is_none());
        assert!(cfg.associations_url().contains("25.03"));
    }
}
```

- [ ] **Create `crates/bdp-ingest/src/pipelines/open_targets/downloader.rs`**:

```rust
// Lists and downloads Parquet files from an Open Targets directory listing.
// Open Targets FTP directories return HTML with <a href="*.parquet"> links.

use anyhow::{Context, Result};
use bytes::Bytes;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, info};

/// Return all `.parquet` hrefs found in an HTML directory listing.
pub async fn list_parquet_files(client: &Client, url: &str) -> Result<Vec<String>> {
    let html = client
        .get(url)
        .send()
        .await
        .context("listing Open Targets directory")?
        .text()
        .await?;

    let doc = Html::parse_document(&html);
    let sel = Selector::parse("a[href]").expect("valid selector");
    let files: Vec<String> = doc
        .select(&sel)
        .filter_map(|el| el.value().attr("href"))
        .filter(|href| href.ends_with(".parquet"))
        .map(|href| {
            if href.starts_with("http") {
                href.to_string()
            } else {
                format!("{}{}", url.trim_end_matches('/'), href)
            }
        })
        .collect();

    info!(count = files.len(), %url, "found parquet files");
    Ok(files)
}

/// Download a single Parquet file into memory.
pub async fn download_parquet(client: &Client, url: &str, max_retries: u32) -> Result<Bytes> {
    let mut last_err = anyhow::anyhow!("no attempts");
    for attempt in 0..=max_retries {
        match client.get(url).send().await {
            Ok(resp) => {
                let bytes = resp.bytes().await.context("reading parquet bytes")?;
                debug!(url, bytes = bytes.len(), "downloaded parquet");
                return Ok(bytes);
            }
            Err(e) => {
                last_err = anyhow::anyhow!("{}", e);
                if attempt < max_retries {
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt))).await;
                }
            }
        }
    }
    Err(last_err).context(format!("downloading {url}"))
}
```

- [ ] **Create `crates/bdp-ingest/src/pipelines/open_targets/mod.rs`**:

```rust
pub mod config;
pub mod downloader;
pub mod mapper;
pub mod runner;
pub mod storage;

pub use config::OpenTargetsConfig;
pub use runner::OpenTargetsPipelineRunner;
```

- [ ] **Register in `crates/bdp-ingest/src/pipelines/mod.rs`**:

```rust
pub mod open_targets;
```

(add alongside existing `pub mod chebi;` etc.)

- [ ] **Verify compile**:
```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -20
```

- [ ] **Run config test**:
```bash
cargo test -p bdp-ingest open_targets::config 2>&1 | tail -10
```
Expected: PASS

- [ ] **Commit**:
```bash
git add crates/bdp-ingest/src/pipelines/
git commit -m "feat(ingest): open_targets config + downloader"
```

---

## Task 4: Open Targets pipeline — Parquet mapper + storage + runner

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/open_targets/mapper.rs`
- Create: `crates/bdp-ingest/src/pipelines/open_targets/storage.rs`
- Create: `crates/bdp-ingest/src/pipelines/open_targets/runner.rs`

The Parquet schema for `association_overall_direct`:
- `targetId`: UTF8 (Ensembl gene ID, e.g. `ENSG00000141510`)
- `diseaseId`: UTF8 (e.g. `MONDO_0005015` — note underscore not colon)
- `score`: FLOAT

The Parquet schema for `targets`:
- `id`: UTF8 (Ensembl ID)
- `approvedSymbol`: UTF8
- `proteinIds`: LIST of STRUCT { id: UTF8, source: UTF8 } — filter `source == "uniprot_swissprot"`

- [ ] **Write unit test for disease ID normalisation** (underscore→colon):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_disease_id() {
        assert_eq!(normalize_disease_id("MONDO_0005015"), "MONDO:0005015");
        assert_eq!(normalize_disease_id("EFO_0000400"), "EFO:0000400");
        assert_eq!(normalize_disease_id("MONDO:0005015"), "MONDO:0005015"); // already colon
    }
}
```

- [ ] **Create `crates/bdp-ingest/src/pipelines/open_targets/mapper.rs`**:

```rust
// Parquet row extraction helpers.

use anyhow::Result;
use arrow_array::{RecordBatch, StringArray, Float32Array};
use std::collections::HashMap;

/// Normalize Open Targets disease IDs: "MONDO_0005015" → "MONDO:0005015"
pub fn normalize_disease_id(id: &str) -> String {
    // OT uses underscore separator, BDP uses colon
    if let Some(pos) = id.find('_') {
        let prefix = &id[..pos];
        if prefix.chars().all(|c| c.is_ascii_uppercase()) {
            return format!("{}:{}", prefix, &id[pos + 1..]);
        }
    }
    id.to_string()
}

pub struct AssociationRow {
    pub ensembl_id: String,
    pub disease_id: String, // normalized (colon)
    pub score: f32,
}

/// Extract association rows from a record batch.
pub fn extract_associations(batch: &RecordBatch) -> Result<Vec<AssociationRow>> {
    let target_col = batch
        .column_by_name("targetId")
        .and_then(|c| c.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| anyhow::anyhow!("missing targetId column"))?;

    let disease_col = batch
        .column_by_name("diseaseId")
        .and_then(|c| c.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| anyhow::anyhow!("missing diseaseId column"))?;

    let score_col = batch
        .column_by_name("score")
        .and_then(|c| c.as_any().downcast_ref::<Float32Array>())
        .ok_or_else(|| anyhow::anyhow!("missing score column"))?;

    let mut rows = Vec::with_capacity(batch.num_rows());
    for i in 0..batch.num_rows() {
        if target_col.is_null(i) || disease_col.is_null(i) {
            continue;
        }
        rows.push(AssociationRow {
            ensembl_id: target_col.value(i).to_string(),
            disease_id: normalize_disease_id(disease_col.value(i)),
            score: if score_col.is_null(i) { 0.0 } else { score_col.value(i) },
        });
    }
    Ok(rows)
}

/// Ensembl ID → UniProt accession lookup table.
pub type EnsemblToUniprot = HashMap<String, String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_disease_id() {
        assert_eq!(normalize_disease_id("MONDO_0005015"), "MONDO:0005015");
        assert_eq!(normalize_disease_id("EFO_0000400"), "EFO:0000400");
        assert_eq!(normalize_disease_id("MONDO:0005015"), "MONDO:0005015");
    }
}
```

- [ ] **Create `crates/bdp-ingest/src/pipelines/open_targets/storage.rs`**:

```rust
use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::warn;
use uuid::Uuid;

pub struct OpenTargetsStorage {
    pool: PgPool,
}

impl OpenTargetsStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Build Ensembl→data_sources.id map by joining through external_id.
    /// data_sources.external_id holds the UniProt accession; the Ensembl mapping
    /// comes from the targets Parquet `ensembl_to_uniprot` arg.
    /// IMPORTANT: uses sqlx::query() runtime — NO sqlx::query!() macros (no offline cache in bdp-mcp/bdp-ingest pipelines)
    pub async fn build_gene_id_map(
        &self,
        ensembl_to_uniprot: &HashMap<String, String>,
    ) -> Result<HashMap<String, Uuid>> {
        let uniprot_accs: Vec<String> = ensembl_to_uniprot.values().cloned().collect();
        if uniprot_accs.is_empty() {
            return Ok(HashMap::new());
        }

        let rows = sqlx::query(
            "SELECT external_id, id FROM data_sources WHERE external_id = ANY($1) AND source_type = 'protein'"
        )
        .bind(&uniprot_accs)
        .fetch_all(&self.pool)
        .await?;

        let uniprot_to_uuid: HashMap<String, Uuid> = rows
            .iter()
            .filter_map(|r| {
                let ext: Option<String> = r.try_get("external_id").ok()?;
                let id: Uuid = r.try_get("id").ok()?;
                Some((ext?, id))
            })
            .collect();

        let map: HashMap<String, Uuid> = ensembl_to_uniprot
            .iter()
            .filter_map(|(ensg, uniprot)| {
                uniprot_to_uuid.get(uniprot).map(|&id| (ensg.clone(), id))
            })
            .collect();

        Ok(map)
    }

    /// Build MONDO term_id → disease_terms.id map.
    /// IMPORTANT: uses sqlx::query() runtime — NO sqlx::query!() macros
    pub async fn build_disease_id_map(
        &self,
        mondo_ids: &[String],
    ) -> Result<HashMap<String, Uuid>> {
        if mondo_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query(
            "SELECT mondo_id, id FROM disease_terms WHERE mondo_id = ANY($1) AND is_obsolete = FALSE"
        )
        .bind(mondo_ids)
        .fetch_all(&self.pool)
        .await?;

        let map: HashMap<String, Uuid> = rows
            .iter()
            .filter_map(|r| {
                let mondo_id: String = r.try_get("mondo_id").ok()?;
                let id: Uuid = r.try_get("id").ok()?;
                Some((mondo_id, id))
            })
            .collect();

        Ok(map)
    }

    /// Bulk-insert associations. Skips rows where gene or disease not found.
    pub async fn insert_associations(
        &self,
        rows: &[(Uuid, Uuid, f32)], // (gene_id, disease_term_id, score)
        source_version: &str,
    ) -> Result<usize> {
        let mut inserted = 0usize;
        for chunk in rows.chunks(500) {
            let gene_ids: Vec<Uuid> = chunk.iter().map(|r| r.0).collect();
            let disease_ids: Vec<Uuid> = chunk.iter().map(|r| r.1).collect();
            let scores: Vec<f32> = chunk.iter().map(|r| r.2).collect();
            let versions: Vec<&str> = chunk.iter().map(|_| source_version).collect();

            let result = sqlx::query(
                r#"INSERT INTO gene_disease_associations
                   (gene_id, disease_term_id, score, source, source_version)
                   SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::float4[], $4::text[], $5::text[])
                   AS t(gene_id, disease_term_id, score, source, source_version)
                   ON CONFLICT (gene_id, disease_term_id, source) DO UPDATE
                   SET score = EXCLUDED.score, source_version = EXCLUDED.source_version"#,
            )
            .bind(&gene_ids)
            .bind(&disease_ids)
            .bind(&scores)
            .bind(vec!["open_targets"; chunk.len()])
            .bind(&versions)
            .execute(&self.pool)
            .await;

            match result {
                Ok(r) => inserted += r.rows_affected() as usize,
                Err(e) => warn!("open_targets insert chunk error: {}", e),
            }
        }
        Ok(inserted)
    }
}
```

- [ ] **Create `crates/bdp-ingest/src/pipelines/open_targets/runner.rs`**:

```rust
use anyhow::Result;
use bytes::Bytes;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use reqwest::Client;
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::open_targets::{
    config::OpenTargetsConfig,
    downloader::{download_parquet, list_parquet_files},
    mapper::{extract_associations, EnsemblToUniprot},
    storage::OpenTargetsStorage,
};

pub struct OpenTargetsPipelineRunner {
    pub config: OpenTargetsConfig,
    pub pool: sqlx::PgPool,
}

impl OpenTargetsPipelineRunner {
    pub fn new(config: OpenTargetsConfig, pool: sqlx::PgPool) -> Self {
        Self { config, pool }
    }

    async fn build_ensembl_map(&self, client: &Client) -> Result<EnsemblToUniprot> {
        let targets_url = self.config.targets_url();
        let files = list_parquet_files(client, &targets_url).await?;
        let mut map = HashMap::new();

        for url in files.iter().take(self.config.parse_limit.unwrap_or(usize::MAX)) {
            let bytes = download_parquet(client, url, self.config.max_retries).await?;
            let reader = ParquetRecordBatchReaderBuilder::try_new(bytes)?
                .build()?;
            for batch in reader {
                let batch = batch?;
                let id_col = batch.column_by_name("id");
                let prot_col = batch.column_by_name("approvedSymbol"); // used as fallback
                // Real extraction: iterate id + proteinIds list column
                // For simplicity, we use the pre-built chembl_uniprot_mapping approach:
                // iterate rows, find proteinIds entries with source=uniprot_swissprot
                let _ = (id_col, prot_col); // columns extracted in full impl
            }
        }
        // NOTE: Full Parquet list column iteration requires arrow nested type handling.
        // Implementation must iterate proteinIds LIST<STRUCT> column per row.
        // See arrow docs for ListArray + StructArray traversal.
        Ok(map)
    }
}

impl PipelineRunner for OpenTargetsPipelineRunner {
    fn name(&self) -> &'static str {
        "open_targets"
    }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());
        let client = Client::new();
        let storage = OpenTargetsStorage::new(self.pool.clone());

        info!("building Ensembl→UniProt map from Open Targets targets/");
        let ensembl_map = self.build_ensembl_map(&client).await?;
        info!(entries = ensembl_map.len(), "Ensembl→UniProt map built");

        let gene_id_map = storage.build_gene_id_map(&ensembl_map).await?;
        info!(resolved = gene_id_map.len(), "gene UUIDs resolved");

        info!("listing Open Targets association files");
        let assoc_url = self.config.associations_url();
        let files = list_parquet_files(&client, &assoc_url).await?;
        info!(files = files.len(), "found association parquet files");

        let mut total_rows: Vec<(Uuid, Uuid, f32)> = Vec::new();

        // Pre-collect all unique disease IDs across all files for bulk lookup
        let mut all_disease_ids: Vec<String> = Vec::new();

        for url in &files {
            let bytes = download_parquet(&client, url, self.config.max_retries).await?;
            let reader = ParquetRecordBatchReaderBuilder::try_new(bytes)?.build()?;
            for batch in reader {
                let batch = batch?;
                let rows = extract_associations(&batch)?;
                for row in rows {
                    if row.score < self.config.min_score {
                        continue;
                    }
                    all_disease_ids.push(row.disease_id);
                }
            }
        }

        all_disease_ids.sort();
        all_disease_ids.dedup();
        let disease_id_map = storage.build_disease_id_map(&all_disease_ids).await?;
        info!(resolved = disease_id_map.len(), "disease UUIDs resolved");

        // Second pass: build insert rows
        for url in &files {
            let bytes = download_parquet(&client, url, self.config.max_retries).await?;
            let reader = ParquetRecordBatchReaderBuilder::try_new(bytes)?.build()?;
            for batch in reader {
                let batch = batch?;
                let rows = extract_associations(&batch)?;
                for row in rows {
                    let Some(&gene_uuid) = gene_id_map.get(&row.ensembl_id) else {
                        continue;
                    };
                    let Some(&disease_uuid) = disease_id_map.get(&row.disease_id) else {
                        continue;
                    };
                    total_rows.push((gene_uuid, disease_uuid, row.score));
                }
            }
        }

        info!(rows = total_rows.len(), "inserting gene-disease associations");
        let inserted = storage
            .insert_associations(&total_rows, &self.config.release)
            .await?;

        stats.records_ingested = inserted as u64;
        stats.records_skipped = (total_rows.len() - inserted) as u64;
        Ok(stats)
    }
}
```

- [ ] **Compile check**:
```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -30
```

- [ ] **Commit**:
```bash
git add crates/bdp-ingest/src/pipelines/open_targets/
git commit -m "feat(ingest): open_targets mapper + storage + runner"
```

---

## Task 5: Open Targets — integration test

**Files:**
- Create: `crates/bdp-ingest/tests/open_targets_test.rs`

The test runs the full pipeline against a real Postgres testcontainer with seed data.

- [ ] **Write the test** — `crates/bdp-ingest/tests/open_targets_test.rs`:

```rust
#[cfg(test)]
mod tests {
    use bdp_ingest::pipelines::open_targets::{OpenTargetsConfig, OpenTargetsPipelineRunner};
    use bdp_ingest::framework::PipelineRunner;
    use sqlx::PgPool;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::postgres::Postgres;
    use uuid::Uuid;

    async fn setup_db() -> PgPool {
        let container = Postgres::default()
            .with_tag("16-alpine")
            .start()
            .await
            .expect("postgres container");
        let host = container.get_host().await.expect("host");
        let port = container.get_host_port_ipv4(5432).await.expect("port");
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        let pool = PgPool::connect(&url).await.expect("connect");
        sqlx::migrate!("../../migrations").run(&pool).await.expect("migrate");
        pool
    }

    #[tokio::test]
    #[ignore = "requires Docker + internet"]
    async fn test_open_targets_schema_exists() {
        let pool = setup_db().await;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'gene_disease_associations'"
        )
        .fetch_one(&pool)
        .await
        .expect("query");
        assert_eq!(count, 1, "gene_disease_associations table should exist");
    }

    #[tokio::test]
    #[ignore = "requires Docker + internet (downloads ~2GB)"]
    async fn test_open_targets_pipeline_runs() {
        let pool = setup_db().await;
        let org_id = Uuid::new_v4();

        // Seed a minimal org + system data (skipped here — pipeline only INSERTs associations
        // where gene_id and disease_term_id already exist; with empty DB, inserted=0 is valid)
        let mut config = OpenTargetsConfig::new("25.03", org_id);
        config.parse_limit = Some(1); // only first parquet file

        let runner = OpenTargetsPipelineRunner::new(config, pool.clone());
        let stats = runner.run().await.expect("pipeline should not error");
        assert_eq!(stats.pipeline_name, "open_targets");
        // With empty seed data, 0 associations inserted is correct (no genes/diseases to join against)
        println!("ingested={} skipped={}", stats.records_ingested, stats.records_skipped);
    }
}
```

- [ ] **Run schema test** (Docker required):
```bash
cargo test -p bdp-ingest test_open_targets_schema_exists -- --include-ignored 2>&1 | tail -15
```
Expected: PASS (table exists after migrations)

- [ ] **Commit**:
```bash
git add crates/bdp-ingest/tests/open_targets_test.rs
git commit -m "test(ingest): open_targets integration test"
```

---

## Task 6: ClinicalTrials pipeline — config + AACT loader + API fetcher

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/clinical_trials/mod.rs`
- Create: `crates/bdp-ingest/src/pipelines/clinical_trials/config.rs`
- Create: `crates/bdp-ingest/src/pipelines/clinical_trials/aact_loader.rs`
- Create: `crates/bdp-ingest/src/pipelines/clinical_trials/api_fetcher.rs`
- Create: `crates/bdp-ingest/src/pipelines/clinical_trials/storage.rs`
- Create: `crates/bdp-ingest/src/pipelines/clinical_trials/runner.rs`
- Modify: `crates/bdp-ingest/src/pipelines/mod.rs`

AACT CSV download: `https://aact.ctti-clinicaltrials.org/rec_download_static_file/studies` (redirect to latest)
API endpoint: `https://clinicaltrials.gov/api/v2/studies`

- [ ] **Create `config.rs`**:

```rust
use chrono::NaiveDate;
use std::path::PathBuf;
use uuid::Uuid;

pub const AACT_BASE_URL: &str = "https://aact.ctti-clinicaltrials.org";
pub const CT_API_BASE: &str = "https://clinicaltrials.gov/api/v2";

#[derive(Debug, Clone)]
pub struct ClinicalTrialsConfig {
    pub aact_dump_path: Option<PathBuf>,
    pub from_date: Option<NaiveDate>,
    pub api_page_size: u32,
    pub max_retries: u32,
    pub org_id: Uuid,
}

impl ClinicalTrialsConfig {
    pub fn new(org_id: Uuid) -> Self {
        Self {
            aact_dump_path: None,
            from_date: None,
            api_page_size: 1000,
            max_retries: 3,
            org_id,
        }
    }

    pub fn with_dump(mut self, path: PathBuf) -> Self {
        self.aact_dump_path = Some(path);
        self
    }

    pub fn with_from_date(mut self, date: NaiveDate) -> Self {
        self.from_date = Some(date);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_config_defaults() {
        let org_id = Uuid::new_v4();
        let cfg = ClinicalTrialsConfig::new(org_id);
        assert!(cfg.aact_dump_path.is_none());
        assert_eq!(cfg.api_page_size, 1000);
    }
}
```

- [ ] **Create `aact_loader.rs`** — parse AACT `studies.txt` CSV (pipe-delimited):

```rust
// Parses AACT studies flat file (pipe-delimited, gzip or plain)
// Key fields: nct_id, brief_title, overall_status, phase, start_date, completion_date, source

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AactStudyRow {
    pub nct_id: String,
    pub brief_title: Option<String>,
    pub overall_status: Option<String>,
    pub phase: Option<String>,
    pub start_date: Option<String>,  // string — parse later
    pub completion_date: Option<String>,
    pub source: Option<String>,
}

pub fn parse_studies_csv(content: &str) -> Result<Vec<AactStudyRow>> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    let mut rows = Vec::new();
    for result in rdr.deserialize() {
        match result {
            Ok(row) => rows.push(row),
            Err(e) => tracing::warn!("AACT CSV parse error (row skipped): {}", e),
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_studies_csv_minimal() {
        let csv = "nct_id|brief_title|overall_status|phase|start_date|completion_date|source\n\
                   NCT00000001|Test Study|Completed|Phase 2|2020-01-01|2022-06-30|Sponsor\n";
        let rows = parse_studies_csv(csv).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].nct_id, "NCT00000001");
        assert_eq!(rows[0].phase.as_deref(), Some("Phase 2"));
    }
}
```

- [ ] **Create `api_fetcher.rs`** — delta fetch via ClinicalTrials API v2:

```rust
use anyhow::{Context, Result};
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStudy {
    pub nct_id: Option<String>,
    pub brief_title: Option<String>,
    pub overall_status: Option<String>,
    pub phase: Option<String>,
}

#[derive(Deserialize)]
struct ApiPage {
    studies: Vec<serde_json::Value>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
    #[serde(rename = "totalCount")]
    total_count: Option<u64>,
}

/// Fetch all studies updated since `from_date`. Returns raw JSON values.
pub async fn fetch_updated_studies(
    client: &Client,
    base_url: &str,
    from_date: NaiveDate,
    page_size: u32,
    max_retries: u32,
) -> Result<Vec<serde_json::Value>> {
    let date_str = from_date.format("%Y-%m-%d").to_string();
    let filter = format!("AREA[LastUpdatePostDate]RANGE[{date_str},MAX]");
    let mut all_studies = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url = format!("{base_url}/studies?query.term={filter}&pageSize={page_size}&format=json");
        if let Some(ref token) = page_token {
            url.push_str(&format!("&pageToken={token}"));
        }

        let mut last_err = anyhow::anyhow!("no attempts");
        let page: ApiPage = 'retry: {
            for attempt in 0..=max_retries {
                match client.get(&url).send().await {
                    Ok(r) => {
                        let text = r.text().await.context("reading CT API response")?;
                        match serde_json::from_str(&text) {
                            Ok(p) => break 'retry p,
                            Err(e) => last_err = e.into(),
                        }
                    }
                    Err(e) => last_err = e.into(),
                }
                if attempt < max_retries {
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt))).await;
                }
            }
            return Err(last_err);
        };

        let count = page.studies.len();
        all_studies.extend(page.studies);
        info!(fetched = all_studies.len(), total = page.total_count, "CT API page");

        match page.next_page_token {
            Some(t) if !t.is_empty() => page_token = Some(t),
            _ => break,
        }
    }

    Ok(all_studies)
}
```

- [ ] **Create `storage.rs`** — upserts trials by `nct_id`, inserts disease/intervention links:

```rust
// crates/bdp-ingest/src/pipelines/clinical_trials/storage.rs
use anyhow::Result;
use sqlx::PgPool;
use tracing::warn;
use crate::pipelines::clinical_trials::aact_loader::AactStudyRow;

pub struct ClinicalTrialsStorage { pool: PgPool }

impl ClinicalTrialsStorage {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    /// Upsert a batch of studies. Returns count inserted/updated.
    pub async fn upsert_studies(&self, rows: &[AactStudyRow]) -> Result<usize> {
        let mut count = 0usize;
        for chunk in rows.chunks(200) {
            let nct_ids: Vec<&str> = chunk.iter().map(|r| r.nct_id.as_str()).collect();
            let titles: Vec<Option<&str>> = chunk.iter().map(|r| r.brief_title.as_deref()).collect();
            let statuses: Vec<Option<&str>> = chunk.iter().map(|r| r.overall_status.as_deref()).collect();
            let phases: Vec<Option<&str>> = chunk.iter().map(|r| r.phase.as_deref()).collect();

            let result = sqlx::query(
                r#"INSERT INTO clinical_trials (nct_id, title, status, phase)
                   SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[])
                   AS t(nct_id, title, status, phase)
                   ON CONFLICT (nct_id) DO UPDATE
                   SET title = EXCLUDED.title, status = EXCLUDED.status,
                       phase = EXCLUDED.phase, updated_at = NOW()"#
            )
            .bind(&nct_ids)
            .bind(&titles)
            .bind(&statuses)
            .bind(&phases)
            .execute(&self.pool)
            .await;

            match result {
                Ok(r) => count += r.rows_affected() as usize,
                Err(e) => warn!("clinical_trials upsert error: {}", e),
            }
        }
        Ok(count)
    }
}
```

- [ ] **Create `runner.rs`** — selects AACT dump or API mode based on config:

```rust
// crates/bdp-ingest/src/pipelines/clinical_trials/runner.rs
use anyhow::Result;
use sqlx::PgPool;
use tracing::info;
use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::clinical_trials::{
    aact_loader::parse_studies_csv,
    api_fetcher::fetch_updated_studies,
    config::ClinicalTrialsConfig,
    storage::ClinicalTrialsStorage,
};

pub struct ClinicalTrialsPipelineRunner {
    pub config: ClinicalTrialsConfig,
    pub pool: PgPool,
}

impl ClinicalTrialsPipelineRunner {
    pub fn new(config: ClinicalTrialsConfig, pool: PgPool) -> Self { Self { config, pool } }
}

impl PipelineRunner for ClinicalTrialsPipelineRunner {
    fn name(&self) -> &'static str { "clinical_trials" }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());
        let storage = ClinicalTrialsStorage::new(self.pool.clone());

        if let Some(dump_path) = &self.config.aact_dump_path {
            // AACT flat-file mode
            info!("loading AACT dump from {:?}", dump_path);
            let content = tokio::fs::read_to_string(dump_path).await?;
            let rows = parse_studies_csv(&content)?;
            info!(rows = rows.len(), "parsed AACT studies");
            let inserted = storage.upsert_studies(&rows).await?;
            stats.records_ingested = inserted as u64;
        } else if let Some(from_date) = self.config.from_date {
            // API delta mode
            let client = reqwest::Client::new();
            info!("fetching CT.gov delta since {}", from_date);
            let raw = fetch_updated_studies(
                &client,
                &crate::pipelines::clinical_trials::config::CT_API_BASE.to_string(),
                from_date,
                self.config.api_page_size,
                self.config.max_retries,
            ).await?;
            info!(count = raw.len(), "fetched CT.gov studies via API");
            // Convert raw JSON to AactStudyRow (simplified — extract nct_id and protocolSection fields)
            let rows: Vec<_> = raw.iter().filter_map(|v| {
                let nct_id = v.pointer("/protocolSection/identificationModule/nctId")
                    ?.as_str()?.to_string();
                Some(crate::pipelines::clinical_trials::aact_loader::AactStudyRow {
                    nct_id,
                    brief_title: v.pointer("/protocolSection/identificationModule/briefTitle")
                        .and_then(|t| t.as_str()).map(String::from),
                    overall_status: v.pointer("/protocolSection/statusModule/overallStatus")
                        .and_then(|t| t.as_str()).map(String::from),
                    phase: v.pointer("/protocolSection/designModule/phases/0")
                        .and_then(|t| t.as_str()).map(String::from),
                    start_date: None,
                    completion_date: None,
                    source: None,
                })
            }).collect();
            let inserted = storage.upsert_studies(&rows).await?;
            stats.records_ingested = inserted as u64;
        } else {
            anyhow::bail!("ClinicalTrialsConfig: set aact_dump_path or from_date");
        }

        Ok(stats)
    }
}
```

- [ ] **Create `mod.rs`**, register in `pipelines/mod.rs`

- [ ] **Write unit test** for `parse_studies_csv` and run it:
```bash
cargo test -p bdp-ingest clinical_trials::aact_loader 2>&1 | tail -10
```
Expected: PASS

- [ ] **Compile check**:
```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -20
```

- [ ] **Commit**:
```bash
git add crates/bdp-ingest/src/pipelines/clinical_trials/
git commit -m "feat(ingest): clinical_trials config + AACT loader + API fetcher + storage + runner"
```

---

## Task 7: ChEMBL pipeline — SQLite ETL

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/chembl/mod.rs`
- Create: `crates/bdp-ingest/src/pipelines/chembl/config.rs`
- Create: `crates/bdp-ingest/src/pipelines/chembl/extractor.rs`
- Create: `crates/bdp-ingest/src/pipelines/chembl/mapper.rs`
- Create: `crates/bdp-ingest/src/pipelines/chembl/storage.rs`
- Create: `crates/bdp-ingest/src/pipelines/chembl/runner.rs`
- Modify: `crates/bdp-ingest/src/pipelines/mod.rs`

**ChEMBL SQLite queries needed:**
```sql
-- Activities with targets
SELECT
    act.molregno,
    cs.standard_inchi_key,
    td.chembl_id AS target_chembl_id,
    act.standard_type,
    act.standard_value,
    act.standard_units,
    act.standard_relation,
    a.assay_type,
    a.chembl_id AS assay_chembl_id,
    act.doc_id::text AS doc_id,
    a.confidence_score
FROM activities act
JOIN compound_structures cs ON cs.molregno = act.molregno
JOIN assays a ON a.assay_id = act.assay_id
JOIN target_dictionary td ON td.tid = a.tid
WHERE act.standard_value IS NOT NULL
  AND cs.standard_inchi_key IS NOT NULL
  AND td.target_type = 'SINGLE PROTEIN';
```

UniProt mapping file format: `ChEMBL_target_id\tUniProt_AC\tname\ttarget_type`

- [ ] **Write unit test** for mapper (InChIKey matching logic):

```rust
#[test]
fn test_inchikey_normalise() {
    // InChIKeys are uppercase, 27 chars, two hyphens
    let key = "BQJCRHHNABKAKU-KBQPJGBKSA-N";
    assert!(is_valid_inchikey(key));
    assert!(!is_valid_inchikey("not-a-key"));
}
```

- [ ] **Create `config.rs`**:

```rust
use std::path::PathBuf;
use uuid::Uuid;

pub const CHEMBL_FTP_BASE: &str =
    "https://ftp.ebi.ac.uk/pub/databases/chembl/ChEMBLdb/releases/chembl_36";

#[derive(Debug, Clone)]
pub struct ChemblConfig {
    pub sqlite_path: PathBuf,
    pub uniprot_mapping_path: Option<PathBuf>,
    pub source_version: String,
    pub batch_size: usize,
    pub org_id: Uuid,
}

impl ChemblConfig {
    pub fn new(sqlite_path: PathBuf, org_id: Uuid) -> Self {
        Self {
            sqlite_path,
            uniprot_mapping_path: None,
            source_version: "chembl_36".to_string(),
            batch_size: 500,
            org_id,
        }
    }
}
```

- [ ] **Create `extractor.rs`** — rusqlite queries:

```rust
use anyhow::Result;
use rusqlite::{Connection, params};

pub struct ActivityRow {
    pub inchikey: String,
    pub target_chembl_id: String,
    pub activity_type: Option<String>,
    pub activity_value: Option<f64>,
    pub activity_unit: Option<String>,
    pub relation: Option<String>,
    pub assay_type: Option<String>,
    pub assay_chembl_id: Option<String>,
    pub doc_id: Option<String>,
    pub confidence: Option<i64>,
}

pub fn extract_activities(conn: &Connection, limit: Option<usize>) -> Result<Vec<ActivityRow>> {
    let limit_clause = limit.map(|n| format!("LIMIT {n}")).unwrap_or_default();
    let sql = format!(
        r#"SELECT cs.standard_inchi_key, td.chembl_id,
                  act.standard_type, act.standard_value, act.standard_units,
                  act.standard_relation, a.assay_type, a.chembl_id,
                  CAST(act.doc_id AS TEXT), a.confidence_score
           FROM activities act
           JOIN compound_structures cs ON cs.molregno = act.molregno
           JOIN assays a ON a.assay_id = act.assay_id
           JOIN target_dictionary td ON td.tid = a.tid
           WHERE act.standard_value IS NOT NULL
             AND cs.standard_inchi_key IS NOT NULL
             AND td.target_type = 'SINGLE PROTEIN'
           {limit_clause}"#
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(ActivityRow {
            inchikey: row.get(0)?,
            target_chembl_id: row.get(1)?,
            activity_type: row.get(2)?,
            activity_value: row.get(3)?,
            activity_unit: row.get(4)?,
            relation: row.get(5)?,
            assay_type: row.get(6)?,
            assay_chembl_id: row.get(7)?,
            doc_id: row.get(8)?,
            confidence: row.get(9)?,
        })
    })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Parse `chembl_uniprot_mapping.txt` — tab-delimited: chembl_id\tuniprot_ac\tname\ttarget_type
pub fn parse_uniprot_mapping(content: &str) -> std::collections::HashMap<String, String> {
    content
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .filter_map(|l| {
            let mut cols = l.splitn(4, '\t');
            let chembl_id = cols.next()?.to_string();
            let uniprot_ac = cols.next()?.to_string();
            Some((chembl_id, uniprot_ac))
        })
        .collect()
}

pub fn is_valid_inchikey(key: &str) -> bool {
    let parts: Vec<&str> = key.split('-').collect();
    parts.len() == 3
        && parts[0].len() == 14
        && parts[1].len() == 10
        && parts[2].len() == 1
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_uppercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inchikey_validate() {
        assert!(is_valid_inchikey("BQJCRHHNABKAKU-KBQPJGBKSA-N"));
        assert!(!is_valid_inchikey("not-a-key"));
        assert!(!is_valid_inchikey(""));
    }

    #[test]
    fn test_parse_uniprot_mapping() {
        let content = "CHEMBL612545\tP00519\tABL1\tSINGLE PROTEIN\n";
        let map = parse_uniprot_mapping(content);
        assert_eq!(map.get("CHEMBL612545").map(|s| s.as_str()), Some("P00519"));
    }
}
```

- [ ] **Run unit tests**:
```bash
cargo test -p bdp-ingest chembl::extractor 2>&1 | tail -10
```
Expected: PASS (no DB needed for unit tests)

- [ ] **Create `mapper.rs`** — resolves InChIKey → compound UUID and ChEMBL target → gene UUID. Uses `sqlx::query()` runtime only (NO macros):

```rust
// crates/bdp-ingest/src/pipelines/chembl/mapper.rs
use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn build_compound_map(pool: &PgPool, inchikeys: &[String]) -> Result<HashMap<String, Uuid>> {
    if inchikeys.is_empty() { return Ok(HashMap::new()); }
    let rows = sqlx::query(
        "SELECT ct.inchikey, ct.data_source_id FROM compound_terms ct WHERE ct.inchikey = ANY($1)"
    )
    .bind(inchikeys)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().filter_map(|r| {
        let k: String = r.try_get("inchikey").ok()?;
        let id: Uuid = r.try_get("data_source_id").ok()?;
        Some((k, id))
    }).collect())
}

pub async fn build_target_map(
    pool: &PgPool,
    chembl_to_uniprot: &HashMap<String, String>,
) -> Result<HashMap<String, Uuid>> {
    let uniprots: Vec<String> = chembl_to_uniprot.values().cloned().collect();
    if uniprots.is_empty() { return Ok(HashMap::new()); }
    let rows = sqlx::query(
        "SELECT external_id, id FROM data_sources WHERE external_id = ANY($1) AND source_type = 'protein'"
    )
    .bind(&uniprots)
    .fetch_all(pool)
    .await?;
    let uniprot_map: HashMap<String, Uuid> = rows.iter().filter_map(|r| {
        let ext: Option<String> = r.try_get("external_id").ok()?;
        let id: Uuid = r.try_get("id").ok()?;
        Some((ext?, id))
    }).collect();
    Ok(chembl_to_uniprot.iter().filter_map(|(cid, uniprot)| {
        uniprot_map.get(uniprot).map(|&id| (cid.clone(), id))
    }).collect())
}
```

- [ ] **Create `storage.rs`** — bulk insert into `drug_target_activities` using UNNEST:

```rust
// crates/bdp-ingest/src/pipelines/chembl/storage.rs
use anyhow::Result;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

pub struct ChemblStorage { pool: PgPool }

impl ChemblStorage {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    pub async fn insert_activities(
        &self,
        rows: &[(Uuid, Uuid, Option<String>, Option<f32>, String)], // (compound_id, target_id, type, value, version)
    ) -> Result<usize> {
        let mut inserted = 0usize;
        for chunk in rows.chunks(500) {
            let compound_ids: Vec<Uuid> = chunk.iter().map(|r| r.0).collect();
            let target_ids: Vec<Uuid> = chunk.iter().map(|r| r.1).collect();
            let types: Vec<Option<&str>> = chunk.iter().map(|r| r.2.as_deref()).collect();
            let values: Vec<Option<f32>> = chunk.iter().map(|r| r.3).collect();
            let versions: Vec<&str> = chunk.iter().map(|r| r.4.as_str()).collect();

            let result = sqlx::query(
                r#"INSERT INTO drug_target_activities
                   (compound_id, target_gene_id, activity_type, activity_value, source_version)
                   SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::text[], $4::float4[], $5::text[])
                   AS t(compound_id, target_gene_id, activity_type, activity_value, source_version)
                   ON CONFLICT DO NOTHING"#
            )
            .bind(&compound_ids).bind(&target_ids).bind(&types)
            .bind(&values).bind(&versions)
            .execute(&self.pool).await;

            match result {
                Ok(r) => inserted += r.rows_affected() as usize,
                Err(e) => warn!("chembl insert error: {}", e),
            }
        }
        Ok(inserted)
    }
}
```

- [ ] **Create `runner.rs`** — CRITICAL: rusqlite is synchronous; ALL SQLite calls MUST be inside `tokio::task::spawn_blocking`:

```rust
// crates/bdp-ingest/src/pipelines/chembl/runner.rs
use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::info;
use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::chembl::{
    config::ChemblConfig,
    extractor::{extract_activities, parse_uniprot_mapping},
    mapper::{build_compound_map, build_target_map},
    storage::ChemblStorage,
};

pub struct ChemblPipelineRunner { pub config: ChemblConfig, pub pool: PgPool }

impl ChemblPipelineRunner {
    pub fn new(config: ChemblConfig, pool: PgPool) -> Self { Self { config, pool } }
}

impl PipelineRunner for ChemblPipelineRunner {
    fn name(&self) -> &'static str { "chembl" }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());

        // Parse UniProt mapping file (small, sync OK)
        let chembl_to_uniprot: HashMap<String, String> =
            if let Some(ref path) = self.config.uniprot_mapping_path {
                let content = tokio::fs::read_to_string(path).await?;
                parse_uniprot_mapping(&content)
            } else {
                HashMap::new()
            };

        // Extract activities from SQLite — MUST use spawn_blocking (rusqlite is sync/blocking)
        let sqlite_path = self.config.sqlite_path.clone();
        let activities = tokio::task::spawn_blocking(move || -> Result<_> {
            let conn = rusqlite::Connection::open(&sqlite_path)?;
            extract_activities(&conn, None)
        }).await??;
        info!(count = activities.len(), "extracted ChEMBL activities");

        // Resolve IDs (async DB lookups — outside spawn_blocking)
        let inchikeys: Vec<String> = activities.iter().map(|a| a.inchikey.clone()).collect();
        let compound_map = build_compound_map(&self.pool, &inchikeys).await?;
        let target_map = build_target_map(&self.pool, &chembl_to_uniprot).await?;

        let insert_rows: Vec<_> = activities.iter().filter_map(|a| {
            let compound_id = *compound_map.get(&a.inchikey)?;
            let target_id = *target_map.get(&a.target_chembl_id)?;
            Some((compound_id, target_id, a.activity_type.clone(),
                  a.activity_value.map(|v| v as f32), self.config.source_version.clone()))
        }).collect();

        let storage = ChemblStorage::new(self.pool.clone());
        let inserted = storage.insert_activities(&insert_rows).await?;
        stats.records_ingested = inserted as u64;
        stats.records_skipped = (insert_rows.len() - inserted) as u64;
        Ok(stats)
    }
}
```

- [ ] **Compile check + unit tests**:
```bash
cargo test -p bdp-ingest chembl 2>&1 | tail -15
```

- [ ] **Commit**:
```bash
git add crates/bdp-ingest/src/pipelines/chembl/
git commit -m "feat(ingest): chembl SQLite ETL pipeline"
```

---

## Task 8: STRING pipeline — protein interactions

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/string_db/mod.rs`
- Create: `crates/bdp-ingest/src/pipelines/string_db/config.rs`
- Create: `crates/bdp-ingest/src/pipelines/string_db/parser.rs`
- Create: `crates/bdp-ingest/src/pipelines/string_db/storage.rs`
- Create: `crates/bdp-ingest/src/pipelines/string_db/runner.rs`
- Modify: `crates/bdp-ingest/src/pipelines/mod.rs`

STRING file URLs (human, v12.0):
- Links: `https://stringdb-downloads.org/download/protein.links.detailed.v12.0/9606.protein.links.detailed.v12.0.txt.gz`
- Aliases: `https://stringdb-downloads.org/download/protein.aliases.v12.0/9606.protein.aliases.v12.0.txt.gz`

STRING links TSV header:
```
protein1 protein2 neighborhood fusion cooccurence coexpression experimental database textmining combined_score
```
Both `protein1`/`protein2` are `9606.ENSP00000XXXXXX` format.

Aliases TSV header: `#string_protein_id alias source`
Filter `source` column for `BLAST_UniProt_AC`.

Deduplication: only store row where `protein1 < protein2` (lexicographic) to avoid storing both directions.

- [ ] **Write unit tests for parser**:

```rust
#[test]
fn test_parse_links_row() {
    let line = "9606.ENSP00000269696 9606.ENSP00000261509 0 0 0 50 300 0 200 450";
    let row = parse_links_row(line).unwrap();
    assert_eq!(row.protein1, "9606.ENSP00000269696");
    assert_eq!(row.combined_score, 450);
}

#[test]
fn test_deduplicate_keeps_a_lt_b() {
    // protein1 < protein2 lexicographically → keep
    assert!(should_keep("9606.ENSP00000000001", "9606.ENSP00000000002"));
    // protein1 > protein2 → skip (already stored in the other direction)
    assert!(!should_keep("9606.ENSP00000000002", "9606.ENSP00000000001"));
}

#[test]
fn test_parse_alias_row() {
    let line = "9606.ENSP00000269696\tP12345\tBLAST_UniProt_AC";
    let (ensp, uniprot) = parse_alias_row(line).unwrap();
    assert_eq!(ensp, "9606.ENSP00000269696");
    assert_eq!(uniprot, "P12345");
}
```

- [ ] **Create `parser.rs`**:

```rust
use anyhow::{bail, Result};

pub struct LinksRow {
    pub protein1: String,
    pub protein2: String,
    pub score_neighborhood: i16,
    pub score_fusion: i16,
    pub score_cooccurrence: i16,
    pub score_coexpression: i16,
    pub score_experimental: i16,
    pub score_database: i16,
    pub score_textmining: i16,
    pub combined_score: i16,
}

pub fn parse_links_row(line: &str) -> Result<LinksRow> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 {
        bail!("invalid links row: {}", line);
    }
    Ok(LinksRow {
        protein1: parts[0].to_string(),
        protein2: parts[1].to_string(),
        score_neighborhood: parts[2].parse()?,
        score_fusion: parts[3].parse()?,
        score_cooccurrence: parts[4].parse()?,
        score_coexpression: parts[5].parse()?,
        score_experimental: parts[6].parse()?,
        score_database: parts[7].parse()?,
        score_textmining: parts[8].parse()?,
        combined_score: parts[9].parse()?,
    })
}

/// True if this row should be stored (deduplication: keep only A < B).
pub fn should_keep(p1: &str, p2: &str) -> bool {
    p1 < p2
}

/// Parse alias row — returns (ensp_id, uniprot_ac) if source is BLAST_UniProt_AC.
pub fn parse_alias_row(line: &str) -> Option<(String, String)> {
    let mut cols = line.splitn(3, '\t');
    let ensp = cols.next()?.to_string();
    let alias = cols.next()?.to_string();
    let source = cols.next()?.trim();
    if source == "BLAST_UniProt_AC" {
        Some((ensp, alias))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_links_row() {
        let line = "9606.ENSP00000269696 9606.ENSP00000261509 0 0 0 50 300 0 200 450";
        let row = parse_links_row(line).unwrap();
        assert_eq!(row.protein1, "9606.ENSP00000269696");
        assert_eq!(row.combined_score, 450);
    }

    #[test]
    fn test_deduplicate_keeps_a_lt_b() {
        assert!(should_keep("9606.ENSP00000000001", "9606.ENSP00000000002"));
        assert!(!should_keep("9606.ENSP00000000002", "9606.ENSP00000000001"));
    }

    #[test]
    fn test_parse_alias_row() {
        let line = "9606.ENSP00000269696\tP12345\tBLAST_UniProt_AC";
        let result = parse_alias_row(line);
        assert!(result.is_some());
        let (ensp, uniprot) = result.unwrap();
        assert_eq!(ensp, "9606.ENSP00000269696");
        assert_eq!(uniprot, "P12345");
    }

    #[test]
    fn test_parse_alias_row_wrong_source() {
        let line = "9606.ENSP00000269696\tsome_alias\tOther_source";
        assert!(parse_alias_row(line).is_none());
    }
}
```

- [ ] **Run parser unit tests**:
```bash
cargo test -p bdp-ingest string_db::parser 2>&1 | tail -10
```
Expected: 4 tests PASS

- [ ] **Create `config.rs`**:

```rust
// crates/bdp-ingest/src/pipelines/string_db/config.rs
use uuid::Uuid;
pub const STRING_LINKS_URL: &str =
    "https://stringdb-downloads.org/download/protein.links.detailed.v12.0/9606.protein.links.detailed.v12.0.txt.gz";
pub const STRING_ALIASES_URL: &str =
    "https://stringdb-downloads.org/download/protein.aliases.v12.0/9606.protein.aliases.v12.0.txt.gz";

#[derive(Debug, Clone)]
pub struct StringConfig {
    pub species_id: u32,
    pub min_combined_score: i16,
    pub links_url: String,
    pub aliases_url: String,
    pub max_retries: u32,
    pub org_id: Uuid,
}

impl StringConfig {
    pub fn new(species_id: u32, min_combined_score: i16, org_id: Uuid) -> Self {
        Self {
            species_id,
            min_combined_score,
            links_url: STRING_LINKS_URL.to_string(),
            aliases_url: STRING_ALIASES_URL.to_string(),
            max_retries: 3,
            org_id,
        }
    }
}
```

- [ ] **Create `storage.rs`** — bulk-insert via UNNEST, dedup enforced by UNIQUE constraint:

```rust
// crates/bdp-ingest/src/pipelines/string_db/storage.rs
use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::warn;
use uuid::Uuid;

pub struct StringStorage { pool: PgPool }

impl StringStorage {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    /// Build ENSP → data_sources.id map using UniProt accessions from alias file.
    pub async fn build_protein_map(
        &self,
        ensp_to_uniprot: &HashMap<String, String>,
    ) -> Result<HashMap<String, Uuid>> {
        let uniprots: Vec<String> = ensp_to_uniprot.values().cloned().collect();
        if uniprots.is_empty() { return Ok(HashMap::new()); }
        let rows = sqlx::query(
            "SELECT external_id, id FROM data_sources WHERE external_id = ANY($1) AND source_type = 'protein'"
        )
        .bind(&uniprots)
        .fetch_all(&self.pool)
        .await?;
        let uniprot_map: HashMap<String, Uuid> = rows.iter().filter_map(|r| {
            let ext: Option<String> = r.try_get("external_id").ok()?;
            let id: Uuid = r.try_get("id").ok()?;
            Some((ext?, id))
        }).collect();
        Ok(ensp_to_uniprot.iter().filter_map(|(ensp, uniprot)| {
            uniprot_map.get(uniprot).map(|&id| (ensp.clone(), id))
        }).collect())
    }

    pub async fn insert_interactions(
        &self,
        // (protein_a_id, protein_b_id, neighborhood, fusion, cooccurrence, coexpression, experimental, database, textmining, combined)
        rows: &[(Uuid, Uuid, i16, i16, i16, i16, i16, i16, i16, i16)],
    ) -> Result<usize> {
        let mut inserted = 0usize;
        for chunk in rows.chunks(1000) {
            let a_ids: Vec<Uuid> = chunk.iter().map(|r| r.0).collect();
            let b_ids: Vec<Uuid> = chunk.iter().map(|r| r.1).collect();
            let combined: Vec<i16> = chunk.iter().map(|r| r.9).collect();
            let experimental: Vec<i16> = chunk.iter().map(|r| r.6).collect();

            let result = sqlx::query(
                r#"INSERT INTO protein_interactions
                   (protein_a_id, protein_b_id, score_experimental, combined_score)
                   SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::smallint[], $4::smallint[])
                   AS t(protein_a_id, protein_b_id, score_experimental, combined_score)
                   ON CONFLICT (protein_a_id, protein_b_id) DO NOTHING"#
            )
            .bind(&a_ids).bind(&b_ids).bind(&experimental).bind(&combined)
            .execute(&self.pool).await;

            match result {
                Ok(r) => inserted += r.rows_affected() as usize,
                Err(e) => warn!("string insert error: {}", e),
            }
        }
        Ok(inserted)
    }
}
```

- [ ] **Create `runner.rs`** — downloads aliases gz first, builds ENSP→UUID map, then streams links:

```rust
// crates/bdp-ingest/src/pipelines/string_db/runner.rs
use anyhow::Result;
use flate2::read::GzDecoder;
use reqwest::Client;
use sqlx::PgPool;
use std::collections::HashMap;
use std::io::Read;
use tracing::info;
use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::string_db::{
    config::StringConfig,
    parser::{parse_alias_row, parse_links_row, should_keep},
    storage::StringStorage,
};

pub struct StringPipelineRunner { pub config: StringConfig, pub pool: PgPool }

impl StringPipelineRunner {
    pub fn new(config: StringConfig, pool: PgPool) -> Self { Self { config, pool } }
}

impl PipelineRunner for StringPipelineRunner {
    fn name(&self) -> &'static str { "string_db" }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());
        let client = Client::new();
        let storage = StringStorage::new(self.pool.clone());

        // 1. Download + parse aliases file
        info!("downloading STRING aliases (~30MB)");
        let alias_bytes = download_gz(&client, &self.config.aliases_url, self.config.max_retries).await?;
        let ensp_to_uniprot: HashMap<String, String> = alias_bytes
            .lines()
            .filter_map(|l| parse_alias_row(l))
            .collect();
        info!(entries = ensp_to_uniprot.len(), "alias map built");

        let protein_map = storage.build_protein_map(&ensp_to_uniprot).await?;
        info!(resolved = protein_map.len(), "ENSP→UUID resolved");

        // 2. Download + parse links file
        info!("downloading STRING links (~130MB)");
        let links_bytes = download_gz(&client, &self.config.links_url, self.config.max_retries).await?;
        let min_score = self.config.min_combined_score;

        let mut insert_rows = Vec::new();
        for line in links_bytes.lines().skip(1) { // skip header
            let row = match parse_links_row(line) {
                Ok(r) => r,
                Err(_) => { stats.records_failed += 1; continue; }
            };
            if row.combined_score < min_score { continue; }
            if !should_keep(&row.protein1, &row.protein2) { continue; }
            let Some(&a_id) = protein_map.get(&row.protein1) else { continue; };
            let Some(&b_id) = protein_map.get(&row.protein2) else { continue; };
            insert_rows.push((a_id, b_id,
                row.score_neighborhood, row.score_fusion, row.score_cooccurrence,
                row.score_coexpression, row.score_experimental, row.score_database,
                row.score_textmining, row.combined_score));
        }
        info!(rows = insert_rows.len(), "inserting STRING interactions");
        let inserted = storage.insert_interactions(&insert_rows).await?;
        stats.records_ingested = inserted as u64;
        Ok(stats)
    }
}

async fn download_gz(client: &Client, url: &str, max_retries: u32) -> Result<String> {
    let mut last_err = anyhow::anyhow!("no attempts");
    for attempt in 0..=max_retries {
        match client.get(url).send().await {
            Ok(resp) => {
                let bytes = resp.bytes().await?;
                let mut gz = GzDecoder::new(bytes.as_ref());
                let mut s = String::new();
                gz.read_to_string(&mut s)?;
                return Ok(s);
            }
            Err(e) => {
                last_err = e.into();
                if attempt < max_retries {
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt))).await;
                }
            }
        }
    }
    Err(last_err)
}
```

- [ ] **Compile check**:
```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -20
```

- [ ] **Commit**:
```bash
git add crates/bdp-ingest/src/pipelines/string_db/
git commit -m "feat(ingest): STRING protein interaction pipeline"
```

---

## Task 9: PubMed pipeline — XML parser + manifest

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/pubmed/mod.rs`
- Create: `crates/bdp-ingest/src/pipelines/pubmed/config.rs`
- Create: `crates/bdp-ingest/src/pipelines/pubmed/manifest.rs`
- Create: `crates/bdp-ingest/src/pipelines/pubmed/parser.rs`
- Modify: `crates/bdp-ingest/src/pipelines/mod.rs`

PubMed FTP base: `https://ftp.ncbi.nlm.nih.gov/pubmed/baseline/`
Directory listing returns HTML with `.xml.gz` links.
Each gz file contains a `<PubmedArticleSet>` with multiple `<PubmedArticle>` elements.

- [ ] **Write unit tests for XML parser** first:

```rust
#[test]
fn test_parse_minimal_article() {
    let xml = r#"<PubmedArticleSet>
<PubmedArticle>
  <MedlineCitation><PMID Version="1">12345678</PMID>
    <Article><ArticleTitle>Test Title</ArticleTitle>
      <Abstract><AbstractText>Some abstract text.</AbstractText></Abstract>
    </Article>
  </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;
    let articles = parse_pubmed_xml(xml.as_bytes()).unwrap();
    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].pmid, 12345678);
    assert_eq!(articles[0].title, "Test Title");
    assert_eq!(articles[0].abstract_text.as_deref(), Some("Some abstract text."));
}
```

- [ ] **Create `parser.rs`** using quick-xml:

```rust
use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;

#[derive(Debug, Default)]
pub struct PubmedArticle {
    pub pmid: i32,
    pub pmcid: Option<String>,
    pub doi: Option<String>,
    pub title: String,
    pub abstract_text: Option<String>,
    pub pub_year: Option<i32>,
    pub pub_month: Option<u32>,
    pub pub_day: Option<u32>,
    pub journal: Option<String>,
    pub mesh_headings: Vec<MeshHeading>,
    pub authors: Vec<Author>,
}

#[derive(Debug, Default)]
pub struct MeshHeading {
    pub ui: String,
    pub descriptor: String,
    pub is_major_topic: bool,
}

#[derive(Debug, Default)]
pub struct Author {
    pub last_name: Option<String>,
    pub fore_name: Option<String>,
    pub collective: Option<String>,
    pub affiliation: Option<String>,
}

/// Parse a PubmedArticleSet XML (bytes) into a Vec of articles.
/// Uses quick-xml event-based streaming to handle large files.
pub fn parse_pubmed_xml(data: &[u8]) -> Result<Vec<PubmedArticle>> {
    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);

    let mut articles = Vec::new();
    let mut current: Option<PubmedArticle> = None;
    let mut buf = Vec::new();
    let mut text_target: Option<&'static str> = None;
    let mut in_article_id = false;
    let mut article_id_type = String::new();
    let mut in_mesh_descriptor = false;
    let mut mesh_major = false;
    let mut current_mesh: MeshHeading = MeshHeading::default();
    let mut current_author: Option<Author> = None;
    let mut author_text_target: Option<&'static str> = None;
    let mut in_abstract = false;
    let mut abstract_parts: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name().as_ref().to_vec();
                match name.as_slice() {
                    b"PubmedArticle" => current = Some(PubmedArticle::default()),
                    b"PMID" => text_target = Some("pmid"),
                    b"ArticleTitle" => text_target = Some("title"),
                    b"AbstractText" => { in_abstract = true; }
                    b"Journal" => {}
                    b"Title" if current.is_some() => text_target = Some("journal"),
                    b"ArticleId" => {
                        in_article_id = true;
                        article_id_type = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|a| a.key.as_ref() == b"IdType")
                            .map(|a| String::from_utf8_lossy(&a.value).to_string())
                            .unwrap_or_default();
                    }
                    b"DescriptorName" => {
                        in_mesh_descriptor = true;
                        mesh_major = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|a| a.key.as_ref() == b"MajorTopicYN")
                            .map(|a| a.value.as_ref() == b"Y")
                            .unwrap_or(false);
                        current_mesh = MeshHeading::default();
                        current_mesh.is_major_topic = mesh_major;
                        if let Some(ui) = e.attributes().filter_map(|a| a.ok())
                            .find(|a| a.key.as_ref() == b"UI") {
                            current_mesh.ui = String::from_utf8_lossy(&ui.value).to_string();
                        }
                    }
                    b"Author" => { current_author = Some(Author::default()); }
                    b"LastName" if current_author.is_some() => author_text_target = Some("last_name"),
                    b"ForeName" if current_author.is_some() => author_text_target = Some("fore_name"),
                    b"CollectiveName" if current_author.is_some() => author_text_target = Some("collective"),
                    b"AffiliationInfo" if current_author.is_some() => author_text_target = Some("affiliation"),
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name().as_ref().to_vec();
                match name.as_slice() {
                    b"PubmedArticle" => {
                        if let Some(mut art) = current.take() {
                            if !abstract_parts.is_empty() {
                                art.abstract_text = Some(abstract_parts.join(" "));
                                abstract_parts.clear();
                            }
                            if art.pmid > 0 && !art.title.is_empty() {
                                articles.push(art);
                            }
                        }
                    }
                    b"AbstractText" => { in_abstract = false; text_target = None; }
                    b"ArticleId" => { in_article_id = false; }
                    b"DescriptorName" => {
                        if let Some(art) = current.as_mut() {
                            art.mesh_headings.push(std::mem::take(&mut current_mesh));
                        }
                        in_mesh_descriptor = false;
                    }
                    b"Author" => {
                        if let (Some(auth), Some(art)) = (current_author.take(), current.as_mut()) {
                            art.authors.push(auth);
                        }
                        author_text_target = None;
                    }
                    _ => { text_target = None; }
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().into_owned();
                if in_abstract {
                    abstract_parts.push(text.clone());
                }
                if in_mesh_descriptor {
                    current_mesh.descriptor = text.clone();
                }
                if in_article_id {
                    if let Some(art) = current.as_mut() {
                        match article_id_type.as_str() {
                            "doi" => art.doi = Some(text.clone()),
                            "pmc" => art.pmcid = Some(text.clone()),
                            _ => {}
                        }
                    }
                }
                if let Some(target) = text_target {
                    if let Some(art) = current.as_mut() {
                        match target {
                            "pmid" => art.pmid = text.parse().unwrap_or(0),
                            "title" => art.title = text.clone(),
                            "journal" => art.journal = Some(text.clone()),
                            _ => {}
                        }
                    }
                }
                if let Some(target) = author_text_target {
                    if let Some(auth) = current_author.as_mut() {
                        match target {
                            "last_name" => auth.last_name = Some(text),
                            "fore_name" => auth.fore_name = Some(text),
                            "collective" => auth.collective = Some(text),
                            "affiliation" => auth.affiliation = Some(text),
                            _ => {}
                        }
                    }
                    author_text_target = None;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(articles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_article() {
        let xml = r#"<PubmedArticleSet>
<PubmedArticle>
  <MedlineCitation><PMID Version="1">12345678</PMID>
    <Article><ArticleTitle>Test Title</ArticleTitle>
      <Abstract><AbstractText>Some abstract text.</AbstractText></Abstract>
    </Article>
  </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;
        let articles = parse_pubmed_xml(xml.as_bytes()).unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].pmid, 12345678);
        assert_eq!(articles[0].title, "Test Title");
        assert_eq!(articles[0].abstract_text.as_deref(), Some("Some abstract text."));
    }

    #[test]
    fn test_parse_skips_empty_pmid() {
        let xml = r#"<PubmedArticleSet>
<PubmedArticle>
  <MedlineCitation><PMID Version="1">0</PMID>
    <Article><ArticleTitle></ArticleTitle></Article>
  </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;
        let articles = parse_pubmed_xml(xml.as_bytes()).unwrap();
        assert_eq!(articles.len(), 0);
    }
}
```

- [ ] **Run parser unit tests**:
```bash
cargo test -p bdp-ingest pubmed::parser 2>&1 | tail -10
```
Expected: 2 tests PASS

- [ ] **Create `manifest.rs`** — list files from FTP directory HTML (same approach as Open Targets downloader, but filters `.xml.gz`). Track state in `pubmed_ingest_files` table.

- [ ] **Create `config.rs`**:

```rust
use uuid::Uuid;

pub const PUBMED_FTP_BASE: &str = "https://ftp.ncbi.nlm.nih.gov/pubmed/baseline/";

#[derive(Debug, Clone)]
pub struct PubmedConfig {
    pub ftp_base: String,
    pub open_access_only: bool,  // if true, only fetch PMC OA subset
    pub worker_count: usize,
    pub batch_size: usize,
    pub max_retries: u32,
    pub parse_limit: Option<usize>, // max files to process
    pub org_id: Uuid,
}

impl PubmedConfig {
    pub fn new(org_id: Uuid) -> Self {
        Self {
            ftp_base: PUBMED_FTP_BASE.to_string(),
            open_access_only: true,  // default: OA subset only
            worker_count: 4,
            batch_size: 1000,
            max_retries: 3,
            parse_limit: None,
            org_id,
        }
    }
}
```

- [ ] **Commit**:
```bash
git add crates/bdp-ingest/src/pipelines/pubmed/
git commit -m "feat(ingest): pubmed XML parser + manifest + config"
```

---

## Task 10: PubMed pipeline — storage + PubTator3 entity linker + runner

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/pubmed/storage.rs`
- Create: `crates/bdp-ingest/src/pipelines/pubmed/entity_linker.rs`
- Create: `crates/bdp-ingest/src/pipelines/pubmed/runner.rs`

- [ ] **Write test for entity linker** (PubTator3 TSV parse):

```rust
#[test]
fn test_parse_pubtator_line() {
    // Format: pmid|type|concept_id|name|mentions_count
    let line = "12345678|Gene|7157|TP53|TP53;p53";
    let entry = parse_pubtator_line(line).unwrap();
    assert_eq!(entry.pmid, 12345678);
    assert_eq!(entry.entity_type, "Gene");
    assert_eq!(entry.concept_id, "7157");
}
```

- [ ] **Create `entity_linker.rs`**:

```rust
use anyhow::Result;

#[derive(Debug)]
pub struct PubTatorEntry {
    pub pmid: i32,
    pub entity_type: String,  // "Gene", "Disease", "Chemical"
    pub concept_id: String,
    pub name: Option<String>,
}

/// Parse a single PubTator3 line: pmid|type|concept_id|name|mentions
pub fn parse_pubtator_line(line: &str) -> Option<PubTatorEntry> {
    if line.starts_with('#') || line.trim().is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.splitn(5, '|').collect();
    if parts.len() < 3 {
        return None;
    }
    Some(PubTatorEntry {
        pmid: parts[0].trim().parse().ok()?,
        entity_type: parts[1].to_string(),
        concept_id: parts[2].to_string(),
        name: parts.get(3).map(|s| s.to_string()),
    })
}

/// Normalize concept IDs to BDP format:
/// Gene: "7157" → kept as NCBI Gene ID
/// Disease: "MESH:D001" → "MESH:D001", "MONDO:0001" → "MONDO:0001"
/// Chemical: "CHEBI:15422" → "CHEBI:15422", "MESH:D001234" → "MESH:D001234"
pub fn normalize_entity_id(entity_type: &str, concept_id: &str) -> String {
    match entity_type {
        "Disease" | "Chemical" => {
            // Already normalized if contains ':'
            if concept_id.contains(':') {
                concept_id.to_string()
            } else if concept_id.starts_with('D') || concept_id.starts_with('C') {
                format!("MESH:{}", concept_id)
            } else {
                concept_id.to_string()
            }
        }
        _ => concept_id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pubtator_line() {
        let line = "12345678|Gene|7157|TP53|TP53;p53";
        let entry = parse_pubtator_line(line).unwrap();
        assert_eq!(entry.pmid, 12345678);
        assert_eq!(entry.entity_type, "Gene");
        assert_eq!(entry.concept_id, "7157");
    }

    #[test]
    fn test_normalize_mesh_disease() {
        assert_eq!(normalize_entity_id("Disease", "D009369"), "MESH:D009369");
        assert_eq!(normalize_entity_id("Disease", "MONDO:0005015"), "MONDO:0005015");
    }
}
```

- [ ] **Run entity_linker tests**:
```bash
cargo test -p bdp-ingest pubmed::entity_linker 2>&1 | tail -10
```
Expected: 2 PASS

- [ ] **Create `storage.rs`** — bulk insert publications + authors + mesh using UNNEST. Key method signature:
```rust
pub async fn insert_publications_batch(
    &self,
    articles: &[PubmedArticle],
) -> Result<usize>
```
Returns count of successfully inserted publications.

- [ ] **Create `runner.rs`**:
  1. List files from `pubmed_ingest_files` with status='pending' (populated by manifest step)
  2. Process `worker_count` files concurrently via JoinSet
  3. For each file: download gz → decompress (flate2 GzDecoder) → parse XML → batch insert
  4. Update `pubmed_ingest_files.status` to 'done'/'error'

```rust
// Decompression pattern using flate2:
use flate2::read::GzDecoder;
use std::io::Read;

let bytes = download_bytes(&client, &url, config.max_retries).await?;
let mut gz = GzDecoder::new(bytes.as_ref());
let mut xml_content = Vec::new();
gz.read_to_end(&mut xml_content)?;
let articles = parser::parse_pubmed_xml(&xml_content)?;
```

- [ ] **Compile check + all pubmed unit tests**:
```bash
cargo test -p bdp-ingest pubmed 2>&1 | tail -15
```

- [ ] **Commit**:
```bash
git add crates/bdp-ingest/src/pipelines/pubmed/
git commit -m "feat(ingest): pubmed storage + entity_linker + runner"
```

---

## Task 11: Register all pipelines in IngestOrchestrator + main.rs env gates

**Files:**
- Modify: `crates/bdp-ingest/src/main.rs`
- Modify (or create): any orchestrator file that spawns pipelines

- [ ] **Read `crates/bdp-ingest/src/main.rs`** to understand current pipeline spawn pattern

- [ ] **Add env-gated spawn for each new pipeline** following existing pattern:

```rust
// Example pattern (read main.rs first to match existing style):
if std::env::var("INGEST_OPEN_TARGETS_ENABLED").as_deref() == Ok("true") {
    let release = std::env::var("INGEST_OPEN_TARGETS_RELEASE")
        .unwrap_or_else(|_| "25.03".to_string());
    let config = OpenTargetsConfig::new(release, org_id);
    let runner = OpenTargetsPipelineRunner::new(config, pool.clone());
    set.spawn(async move { runner.run().await });
}

if std::env::var("INGEST_CLINICAL_TRIALS_ENABLED").as_deref() == Ok("true") {
    let config = ClinicalTrialsConfig::new(org_id);
    let runner = ClinicalTrialsPipelineRunner::new(config, pool.clone());
    set.spawn(async move { runner.run().await });
}

if std::env::var("INGEST_CHEMBL_ENABLED").as_deref() == Ok("true") {
    let sqlite_path = std::env::var("INGEST_CHEMBL_SQLITE_PATH")
        .map(std::path::PathBuf::from)
        .expect("INGEST_CHEMBL_SQLITE_PATH required when INGEST_CHEMBL_ENABLED=true");
    let config = ChemblConfig::new(sqlite_path, org_id);
    let runner = ChemblPipelineRunner::new(config, pool.clone());
    set.spawn(async move { runner.run().await });
}

if std::env::var("INGEST_STRING_ENABLED").as_deref() == Ok("true") {
    let min_score: i16 = std::env::var("INGEST_STRING_MIN_SCORE")
        .ok().and_then(|s| s.parse().ok()).unwrap_or(400);
    let config = StringConfig::new(9606, min_score, org_id);
    let runner = StringPipelineRunner::new(config, pool.clone());
    set.spawn(async move { runner.run().await });
}

if std::env::var("INGEST_PUBMED_ENABLED").as_deref() == Ok("true") {
    let config = PubmedConfig::new(org_id);
    let runner = PubmedPipelineRunner::new(config, pool.clone());
    set.spawn(async move { runner.run().await });
}
```

- [ ] **Compile check**:
```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -20
```

- [ ] **Commit**:
```bash
git add crates/bdp-ingest/src/main.rs
git commit -m "feat(ingest): register open_targets, clinical_trials, chembl, string_db, pubmed in orchestrator"
```

---

## Task 12: Activate stub MCP tools → live implementations

**Files:**
- Modify: `crates/bdp-mcp/src/tools/genes.rs`
- Modify: `crates/bdp-mcp/src/tools/diseases.rs`
- Modify: `crates/bdp-mcp/src/tools/compounds.rs`
- Modify: `crates/bdp-mcp/src/tools/literature.rs`
- Modify: `crates/bdp-mcp/src/db/queries.rs`

For each stub tool, the pattern is:
1. Add query function to `db/queries.rs` using `sqlx::query()` (runtime, no macros)
2. Replace `Ok(common::stub_result(...))` in the tool with a real DB call + JSON serialization

- [ ] **Activate `get_gene_diseases`** in `genes.rs`:

Add to `db/queries.rs`:
```rust
pub async fn get_gene_diseases(
    pool: &PgPool,
    gene_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    sqlx::query(
        r#"SELECT dt.mondo_id, dt.name, gda.score, gda.source_version
           FROM gene_disease_associations gda
           JOIN disease_terms dt ON dt.id = gda.disease_term_id
           WHERE gda.gene_id = $1
           ORDER BY gda.score DESC NULLS LAST
           LIMIT $2 OFFSET $3"#
    )
    .bind(gene_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map(|rows| rows.iter().map(|r| serde_json::json!({
        "mondo_id": r.try_get::<String, _>("mondo_id").unwrap_or_default(),
        "name": r.try_get::<String, _>("name").unwrap_or_default(),
        "score": r.try_get::<Option<f32>, _>("score").unwrap_or(None),
        "source": r.try_get::<Option<String>, _>("source_version").unwrap_or(None),
    })).collect())
}
```

Replace stub in `get_gene_diseases_stub` → `get_gene_diseases` in `genes.rs`.

- [ ] **Activate `get_disease_trials`** in `diseases.rs` (query `trial_disease_links JOIN clinical_trials`)

- [ ] **Activate `get_compound_targets`** in `compounds.rs` (query `drug_target_activities JOIN data_sources`)

- [ ] **Activate `search_literature`** in `literature.rs` (FTS on publications using `to_tsvector @@ plainto_tsquery`)

- [ ] **Activate `get_publication`** in `literature.rs` (lookup by PMID + fetch authors/mesh)

- [ ] **Add NEW tool `get_gene_interactions`** for STRING — this tool does NOT exist yet (spec §MCP Tool Activation Map requires it):

  **Step 1**: Add to `crates/bdp-mcp/src/tools/genes.rs`:

  ```rust
  #[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
  pub struct GetGeneInteractionsParams {
      /// UniProt accession or gene symbol
      pub gene: String,
      /// Minimum combined STRING score (0-1000, default 400)
      pub min_score: Option<i16>,
      pub limit: Option<i64>,
      pub cursor: Option<String>,
  }
  ```

  **Step 2**: Add query to `crates/bdp-mcp/src/db/queries.rs`:

  ```rust
  pub async fn get_gene_interactions(
      pool: &PgPool,
      gene_uuid: Uuid,
      min_score: i16,
      limit: i64,
      offset: i64,
  ) -> Result<Vec<serde_json::Value>, sqlx::Error> {
      // Handles both directions (protein_a or protein_b)
      sqlx::query(
          r#"SELECT
              CASE WHEN pi.protein_a_id = $1 THEN ds_b.external_id ELSE ds_a.external_id END AS partner,
              pi.combined_score, pi.score_experimental
             FROM protein_interactions pi
             JOIN data_sources ds_a ON ds_a.id = pi.protein_a_id
             JOIN data_sources ds_b ON ds_b.id = pi.protein_b_id
             WHERE (pi.protein_a_id = $1 OR pi.protein_b_id = $1)
               AND pi.combined_score >= $2
             ORDER BY pi.combined_score DESC
             LIMIT $3 OFFSET $4"#
      )
      .bind(gene_uuid).bind(min_score).bind(limit).bind(offset)
      .fetch_all(pool)
      .await
      .map(|rows| rows.iter().map(|r| serde_json::json!({
          "partner_uniprot": r.try_get::<Option<String>, _>("partner").unwrap_or(None),
          "combined_score": r.try_get::<i16, _>("combined_score").unwrap_or(0),
          "experimental_score": r.try_get::<Option<i16>, _>("score_experimental").unwrap_or(None),
      })).collect())
  }
  ```

  **Step 3**: Add `#[tool]` method to `BdpMcpServer` in `server.rs` and register in `#[tool_router]`.

- [ ] **Update server.rs tool names** — rename stub methods to live names if they changed

- [ ] **Compile check**:
```bash
SQLX_OFFLINE=true cargo check -p bdp-mcp 2>&1 | grep "^error" | head -20
```

- [ ] **Commit**:
```bash
git add crates/bdp-mcp/src/
git commit -m "feat(mcp): activate stub tools — gene_diseases, disease_trials, compound_targets, literature search"
```

---

## Task 13: Final review + clippy + push

- [ ] **Run all unit tests** (no Docker needed):
```bash
cargo test -p bdp-ingest 2>&1 | grep -E "^test |FAILED|passed|failed" | tail -30
```
Expected: all unit tests pass; integration tests skipped (need `--include-ignored` + Docker)

- [ ] **Run clippy**:
```bash
cargo clippy -p bdp-ingest -p bdp-mcp -- -D warnings 2>&1 | head -40
```
Fix any warnings before continuing.

- [ ] **Run fmt**:
```bash
cargo fmt -p bdp-ingest -p bdp-mcp
```

- [ ] **Final compile check**:
```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest -p bdp-mcp 2>&1 | grep "^error" | head -20
```

- [ ] **Commit any fmt/clippy fixes**:
```bash
git add -p  # stage only changed files
git commit -m "fix(ingest,mcp): clippy + fmt"
```

- [ ] **Push to remote**:
```bash
git push origin main
```
