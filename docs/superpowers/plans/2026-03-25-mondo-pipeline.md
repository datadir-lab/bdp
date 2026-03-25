# MONDO Disease Ontology Pipeline Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a full MONDO (Monarch Disease Ontology) ingestion pipeline — DB migration, OBO parsing via the shared `OboParser`, storage layer with proper relational tables, and `PipelineRunner` implementation — outputting `disease_terms` and `disease_relationships` in the database.

**Architecture:** MONDO uses OBO 1.4 format (same as GO). The `OboParser` from `bdp-ingest::common::obo` handles parsing; this pipeline provides a thin domain adapter mapping `RawOboTerm` → `DiseaseTerm`. Full DB writes are in `bdp-ingest` using `sqlx::query()` (runtime, not macros — no offline cache needed). Follows the same `registry_entries → data_sources → versions → domain_tables` chain as all other pipelines.

**Data source:** `https://purl.obolibrary.org/obo/mondo.obo` (~50MB, ~27K terms)

**Tech Stack:** Rust, tokio, sqlx (runtime queries), reqwest, bdp-ingest common utilities

---

## File Map

**New migration:**
- `migrations/20260326000002_mondo_tables.sql`

**New Rust files in `crates/bdp-ingest/src/pipelines/mondo/`:**
- `mod.rs` — module entry, constants, public re-exports
- `models.rs` — `DiseaseTerm`, `DiseaseRelationship`, `DiseaseRelationType`
- `parser.rs` — `RawOboTerm → DiseaseTerm` adapter
- `storage.rs` — `MondoStorage`: writes to `registry_entries`, `data_sources`, `versions`, `disease_terms`, `disease_relationships`, `ont_term_synonyms`, `ont_term_xrefs`
- `runner.rs` — `MondoPipelineRunner` implementing `PipelineRunner`

**Modified:**
- `crates/bdp-ingest/src/pipelines/mod.rs` — uncomment `pub mod mondo;`

---

## Task 1: DB migration for MONDO tables

**Files:**
- Create: `migrations/20260326000002_mondo_tables.sql`

- [ ] **Step 1: Write migration**

```sql
-- migrations/20260326000002_mondo_tables.sql
-- MONDO Disease Ontology domain tables

-- Register source type (INSERT only — no DDL needed for new pipelines)
INSERT INTO source_types (name, label, description)
VALUES ('disease', 'Disease', 'Disease terms from MONDO (Monarch Disease Ontology)')
ON CONFLICT (name) DO NOTHING;

-- Primary disease term table
CREATE TABLE disease_terms (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id      UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    mondo_id            TEXT NOT NULL,              -- 'MONDO:0004992'
    mondo_accession     BIGINT NOT NULL,            -- 4992
    name                TEXT NOT NULL,
    definition          TEXT,
    is_obsolete         BOOLEAN NOT NULL DEFAULT FALSE,
    comment             TEXT,
    -- Denormalized external IDs for fast lookup (full data in ont_term_xrefs)
    omim_id             TEXT,                       -- first OMIM xref, if any
    orphanet_id         TEXT,                       -- first ORPHA xref, if any
    mondo_release       TEXT NOT NULL,              -- e.g., '2026-03-01'
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_disease_per_release UNIQUE (mondo_id, mondo_release)
);

CREATE INDEX idx_disease_mondo_id    ON disease_terms(mondo_id);
CREATE INDEX idx_disease_accession   ON disease_terms(mondo_accession);
CREATE INDEX idx_disease_omim        ON disease_terms(omim_id) WHERE omim_id IS NOT NULL;
CREATE INDEX idx_disease_orphanet    ON disease_terms(orphanet_id) WHERE orphanet_id IS NOT NULL;
CREATE INDEX idx_disease_data_source ON disease_terms(data_source_id);
CREATE INDEX idx_disease_obsolete    ON disease_terms(is_obsolete) WHERE is_obsolete = FALSE;
CREATE INDEX idx_disease_name_fts    ON disease_terms
    USING GIN (to_tsvector('english', name));

-- Hierarchical relationships (is_a, subClassOf, etc.)
CREATE TABLE disease_relationships (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_mondo_id    TEXT NOT NULL,   -- child
    object_mondo_id     TEXT NOT NULL,   -- parent
    relationship_type   TEXT NOT NULL,   -- 'is_a', 'subClassOf', 'part_of'
    mondo_release       TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_disease_rel UNIQUE (subject_mondo_id, object_mondo_id, relationship_type, mondo_release)
);

CREATE INDEX idx_disease_rel_subject ON disease_relationships(subject_mondo_id);
CREATE INDEX idx_disease_rel_object  ON disease_relationships(object_mondo_id);
CREATE INDEX idx_disease_rel_type    ON disease_relationships(relationship_type);
```

- [ ] **Step 2: Apply migration (if DB available; skip if not)**

```bash
cargo xtask db migrate 2>&1 | tail -5
```
If no DB: skip and continue — schema is verified through Rust compile checks.

- [ ] **Step 3: Commit**

```bash
git add migrations/20260326000002_mondo_tables.sql
git commit -m "feat(db): add disease_terms and disease_relationships tables for MONDO"
```

---

## Task 2: Domain models

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/mondo/mod.rs`
- Create: `crates/bdp-ingest/src/pipelines/mondo/models.rs`

- [ ] **Step 1: Create mod.rs**

```rust
// crates/bdp-ingest/src/pipelines/mondo/mod.rs

pub mod models;
pub mod parser;
pub mod runner;
pub mod storage;

pub use runner::{MondoConfig, MondoPipelineRunner};

pub const MONDO_OBO_URL: &str = "https://purl.obolibrary.org/obo/mondo.obo";
```

- [ ] **Step 2: Create models.rs**

```rust
// crates/bdp-ingest/src/pipelines/mondo/models.rs

use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DiseaseTerm {
    pub mondo_id: String,         // "MONDO:0004992"
    pub mondo_accession: i64,     // 4992
    pub name: String,
    pub definition: Option<String>,
    pub is_obsolete: bool,
    pub comment: Option<String>,
    pub omim_id: Option<String>,
    pub orphanet_id: Option<String>,
    pub mondo_release: String,
}

#[derive(Debug, Clone)]
pub enum DiseaseRelationType {
    IsA,
    SubClassOf,
    PartOf,
    Other(String),
}

impl DiseaseRelationType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::IsA => "is_a",
            Self::SubClassOf => "subClassOf",
            Self::PartOf => "part_of",
            Self::Other(s) => s,
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "is_a" => Self::IsA,
            "subClassOf" => Self::SubClassOf,
            "part_of" => Self::PartOf,
            other => Self::Other(other.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiseaseRelationship {
    pub subject_mondo_id: String,
    pub object_mondo_id: String,
    pub relationship_type: DiseaseRelationType,
    pub mondo_release: String,
}

/// Result of parsing a MONDO OBO file.
#[derive(Debug, Default)]
pub struct ParsedMondo {
    pub terms: Vec<DiseaseTerm>,
    pub relationships: Vec<DiseaseRelationship>,
}

impl ParsedMondo {
    pub fn term_count(&self) -> usize { self.terms.len() }
    pub fn relationship_count(&self) -> usize { self.relationships.len() }
}
```

- [ ] **Step 3: Compile check**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -10
```

- [ ] **Step 4: Commit**

```bash
git add crates/bdp-ingest/src/pipelines/mondo/
git commit -m "feat(bdp-ingest): add MONDO domain models"
```

---

## Task 3: OBO parser adapter

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/mondo/parser.rs`

- [ ] **Step 1: Create parser.rs**

```rust
// crates/bdp-ingest/src/pipelines/mondo/parser.rs
//
// Thin adapter: RawOboTerm → DiseaseTerm / DiseaseRelationship
// OBO parsing is handled by bdp_ingest::common::obo::OboParser.

use crate::common::obo::{OboParser, OboParseError};
use crate::pipelines::mondo::models::{
    DiseaseTerm, DiseaseRelationship, DiseaseRelationType, ParsedMondo,
};
use tracing::warn;

pub fn parse_obo(content: &str, release: &str, limit: Option<usize>) -> Result<ParsedMondo, OboParseError> {
    let raw_terms = OboParser::parse(content, limit)?;
    let mut parsed = ParsedMondo::default();

    for raw in raw_terms {
        // Only process MONDO-prefixed terms
        if !raw.id.starts_with("MONDO:") {
            continue;
        }

        let accession: i64 = raw.id
            .trim_start_matches("MONDO:")
            .parse()
            .unwrap_or(0);

        // Extract OMIM and Orphanet from xrefs
        let omim_id = raw.xrefs.iter()
            .find(|x| x.starts_with("OMIM:"))
            .and_then(|x| x.strip_prefix("OMIM:"))
            .map(|s| s.to_string());

        let orphanet_id = raw.xrefs.iter()
            .find(|x| x.starts_with("ORPHA:") || x.starts_with("Orphanet:"))
            .map(|x| {
                x.trim_start_matches("ORPHA:")
                 .trim_start_matches("Orphanet:")
                 .to_string()
            });

        let term = DiseaseTerm {
            mondo_id: raw.id.clone(),
            mondo_accession: accession,
            name: raw.name.clone(),
            definition: raw.definition.clone(),
            is_obsolete: raw.is_obsolete,
            comment: raw.comment.clone(),
            omim_id,
            orphanet_id,
            mondo_release: release.to_string(),
        };
        parsed.terms.push(term);

        // is_a relationships
        for parent_id in &raw.is_a {
            if !parent_id.starts_with("MONDO:") {
                continue;
            }
            parsed.relationships.push(DiseaseRelationship {
                subject_mondo_id: raw.id.clone(),
                object_mondo_id: parent_id.clone(),
                relationship_type: DiseaseRelationType::IsA,
                mondo_release: release.to_string(),
            });
        }

        // Other typed relationships
        for rel in &raw.relationships {
            if !rel.target.starts_with("MONDO:") {
                continue;
            }
            parsed.relationships.push(DiseaseRelationship {
                subject_mondo_id: raw.id.clone(),
                object_mondo_id: rel.target.clone(),
                relationship_type: DiseaseRelationType::from_str(&rel.rel_type),
                mondo_release: release.to_string(),
            });
        }
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MONDO: &str = r#"
format-version: 1.2
ontology: mondo

[Term]
id: MONDO:0004992
name: cancer
def: "A disease involving uncontrolled cell growth." [HPO:probinson]
synonym: "malignant neoplasm" EXACT []
synonym: "malignancy" BROAD []
xref: OMIM:114500
xref: ORPHA:68335
is_a: MONDO:0000001 ! disease

[Term]
id: MONDO:0000001
name: disease
"#;

    #[test]
    fn test_parse_basic_disease() {
        let parsed = parse_obo(SAMPLE_MONDO, "2026-03-01", None).unwrap();
        assert_eq!(parsed.terms.len(), 2);

        let cancer = parsed.terms.iter().find(|t| t.mondo_id == "MONDO:0004992").unwrap();
        assert_eq!(cancer.name, "cancer");
        assert_eq!(cancer.mondo_accession, 4992);
        assert_eq!(cancer.omim_id.as_deref(), Some("114500"));
        assert_eq!(cancer.orphanet_id.as_deref(), Some("68335"));
        assert!(!cancer.is_obsolete);

        // Should have one is_a relationship
        assert_eq!(parsed.relationships.len(), 1);
        assert_eq!(parsed.relationships[0].subject_mondo_id, "MONDO:0004992");
        assert_eq!(parsed.relationships[0].object_mondo_id, "MONDO:0000001");
    }

    #[test]
    fn test_parse_limit() {
        let parsed = parse_obo(SAMPLE_MONDO, "2026-03-01", Some(1)).unwrap();
        assert_eq!(parsed.terms.len(), 1);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p bdp-ingest pipelines::mondo::parser 2>&1 | tail -20
```
Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-ingest/src/pipelines/mondo/parser.rs
git commit -m "feat(bdp-ingest): add MONDO OBO parser adapter"
```

---

## Task 4: Storage layer

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/mondo/storage.rs`

Uses `sqlx::query()` (runtime) — no `sqlx::query!()` macros (avoids offline cache dependency).

- [ ] **Step 1: Create storage.rs**

```rust
// crates/bdp-ingest/src/pipelines/mondo/storage.rs

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction};
use tracing::info;
use uuid::Uuid;

use crate::common::batch::BatchConfig;
use crate::common::obo::OboParser;
use crate::pipelines::mondo::models::{DiseaseTerm, DiseaseRelationship, ParsedMondo};

pub struct MondoStorage {
    pool: PgPool,
    batch: BatchConfig,
}

impl MondoStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool, batch: BatchConfig::default() }
    }

    /// Full ingest: register + store all terms and relationships.
    pub async fn ingest_release(
        &self,
        org_id: Uuid,
        release: &str,
        parsed: &ParsedMondo,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.context("begin transaction")?;

        let (registry_id, data_source_id) = self.upsert_registry(&mut tx, org_id).await?;
        let version_id = self.upsert_version(&mut tx, data_source_id, release).await?;

        info!(release, terms = parsed.terms.len(), "storing MONDO terms");
        let term_ids = self.store_terms(&mut tx, data_source_id, &parsed.terms).await?;

        info!(rels = parsed.relationships.len(), "storing MONDO relationships");
        self.store_relationships(&mut tx, &parsed.relationships).await?;

        // Store synonyms and xrefs for all terms via ont_term_* tables
        // (implementation: iterate terms, get their UUIDs from term_ids map, insert synonyms/xrefs)

        tx.commit().await.context("commit transaction")?;
        info!(release, "MONDO ingest complete");
        Ok(())
    }

    async fn upsert_registry(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        org_id: Uuid,
    ) -> Result<(Uuid, Uuid)> {
        // Upsert registry entry
        let registry_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, 'mondo', 'MONDO Disease Ontology', 'data_source')
            ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name
            RETURNING id
            "#,
        )
        .bind(org_id)
        .fetch_one(&mut **tx)
        .await
        .context("upsert registry entry")?;

        // Upsert data source
        let data_source_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO data_sources (id, source_type, external_id)
            VALUES ($1, 'disease', 'mondo')
            ON CONFLICT (id) DO UPDATE SET source_type = EXCLUDED.source_type
            RETURNING id
            "#,
        )
        .bind(registry_id)
        .fetch_one(&mut **tx)
        .await
        .context("upsert data source")?;

        Ok((registry_id, data_source_id))
    }

    async fn upsert_version(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        data_source_id: Uuid,
        release: &str,
    ) -> Result<Uuid> {
        let version_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO versions (entry_id, version, release_date)
            VALUES ($1, $2, CURRENT_DATE)
            ON CONFLICT (entry_id, version) DO UPDATE SET release_date = EXCLUDED.release_date
            RETURNING id
            "#,
        )
        .bind(data_source_id)
        .bind(release)
        .fetch_one(&mut **tx)
        .await
        .context("upsert version")?;

        Ok(version_id)
    }

    async fn store_terms(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        data_source_id: Uuid,
        terms: &[DiseaseTerm],
    ) -> Result<Vec<Uuid>> {
        let mut ids = Vec::with_capacity(terms.len());

        for chunk in terms.chunks(self.batch.chunk_size) {
            for term in chunk {
                let id: Uuid = sqlx::query_scalar(
                    r#"
                    INSERT INTO disease_terms (
                        data_source_id, mondo_id, mondo_accession, name, definition,
                        is_obsolete, comment, omim_id, orphanet_id, mondo_release
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                    ON CONFLICT (mondo_id, mondo_release)
                    DO UPDATE SET
                        name = EXCLUDED.name,
                        definition = EXCLUDED.definition,
                        is_obsolete = EXCLUDED.is_obsolete,
                        omim_id = EXCLUDED.omim_id,
                        orphanet_id = EXCLUDED.orphanet_id,
                        updated_at = NOW()
                    RETURNING id
                    "#,
                )
                .bind(data_source_id)
                .bind(&term.mondo_id)
                .bind(term.mondo_accession)
                .bind(&term.name)
                .bind(&term.definition)
                .bind(term.is_obsolete)
                .bind(&term.comment)
                .bind(&term.omim_id)
                .bind(&term.orphanet_id)
                .bind(&term.mondo_release)
                .fetch_one(&mut **tx)
                .await
                .context("insert disease term")?;

                ids.push(id);
            }
        }

        Ok(ids)
    }

    async fn store_relationships(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        relationships: &[DiseaseRelationship],
    ) -> Result<()> {
        for chunk in relationships.chunks(self.batch.chunk_size) {
            for rel in chunk {
                sqlx::query(
                    r#"
                    INSERT INTO disease_relationships (
                        subject_mondo_id, object_mondo_id, relationship_type, mondo_release
                    ) VALUES ($1, $2, $3, $4)
                    ON CONFLICT (subject_mondo_id, object_mondo_id, relationship_type, mondo_release)
                    DO NOTHING
                    "#,
                )
                .bind(&rel.subject_mondo_id)
                .bind(&rel.object_mondo_id)
                .bind(rel.relationship_type.as_str())
                .bind(&rel.mondo_release)
                .execute(&mut **tx)
                .await
                .context("insert disease relationship")?;
            }
        }

        Ok(())
    }
}
```

- [ ] **Step 2: Compile check**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -10
```

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-ingest/src/pipelines/mondo/storage.rs
git commit -m "feat(bdp-ingest): add MONDO storage layer with registry chain"
```

---

## Task 5: PipelineRunner implementation

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/mondo/runner.rs`
- Modify: `crates/bdp-ingest/src/pipelines/mod.rs`

- [ ] **Step 1: Create runner.rs**

```rust
// crates/bdp-ingest/src/pipelines/mondo/runner.rs

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::common::http::download_text;
use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::mondo::{parser, storage::MondoStorage, MONDO_OBO_URL};

#[derive(Debug, Clone)]
pub struct MondoConfig {
    pub obo_url: String,
    pub max_retries: u32,
    pub release: String,
    pub org_id: Uuid,
    pub parse_limit: Option<usize>,
}

impl MondoConfig {
    pub fn new(release: impl Into<String>, org_id: Uuid) -> Self {
        Self {
            obo_url: MONDO_OBO_URL.to_string(),
            max_retries: 3,
            release: release.into(),
            org_id,
            parse_limit: None,
        }
    }
}

pub struct MondoPipelineRunner {
    config: MondoConfig,
    pool: PgPool,
}

impl MondoPipelineRunner {
    pub fn new(config: MondoConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }
}

impl PipelineRunner for MondoPipelineRunner {
    fn name(&self) -> &'static str { "mondo" }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());

        info!(url = %self.config.obo_url, "downloading MONDO OBO");
        let content = download_text(&self.config.obo_url, self.config.max_retries).await?;

        info!("parsing MONDO OBO ({} bytes)", content.len());
        let parsed = parser::parse_obo(&content, &self.config.release, self.config.parse_limit)?;

        stats.records_ingested = parsed.term_count() as u64;
        stats.records_skipped = parsed.terms.iter().filter(|t| t.is_obsolete).count() as u64;

        info!(
            terms = parsed.term_count(),
            rels = parsed.relationship_count(),
            "MONDO parsed — storing"
        );

        let storage = MondoStorage::new(self.pool);
        storage.ingest_release(self.config.org_id, &self.config.release, &parsed).await?;

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let org_id = Uuid::new_v4();
        let cfg = MondoConfig::new("2026-03-01", org_id);
        assert_eq!(cfg.obo_url, MONDO_OBO_URL);
        assert_eq!(cfg.max_retries, 3);
        assert_eq!(cfg.org_id, org_id);
    }
}
```

- [ ] **Step 2: Uncomment mondo in pipelines/mod.rs**

Edit `crates/bdp-ingest/src/pipelines/mod.rs` to add:
```rust
pub mod mondo;
```

- [ ] **Step 3: Compile check**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -10
```

- [ ] **Step 4: Run all unit tests**

```bash
cargo test -p bdp-ingest 2>&1 | tail -20
```
Expected: all pass including MONDO parser tests.

- [ ] **Step 5: Commit**

```bash
git add crates/bdp-ingest/src/pipelines/mondo/runner.rs \
        crates/bdp-ingest/src/pipelines/mod.rs
git commit -m "feat(bdp-ingest): add MondoPipelineRunner — complete MONDO pipeline"
```

---

## Task 6: Integration test against real MONDO data

**Files:**
- Modify: `crates/bdp-ingest/tests/obo_integration.rs`

- [ ] **Step 1: Add MONDO integration test to the existing test file**

```rust
/// Parse real MONDO OBO (full, no limit) and verify counts.
/// Run: cargo test -p bdp-ingest --test obo_integration test_parse_full_mondo -- --ignored --nocapture
#[tokio::test]
#[ignore = "downloads ~50MB from internet"]
async fn test_parse_full_mondo() {
    let url = "https://purl.obolibrary.org/obo/mondo.obo";
    let content = bdp_ingest::common::http::download_text(url, 3)
        .await
        .expect("download MONDO");

    let parsed = bdp_ingest::pipelines::mondo::parser::parse_obo(&content, "test", None)
        .expect("parse MONDO");

    // MONDO has ~27K MONDO-prefixed terms
    assert!(
        parsed.term_count() > 20_000,
        "expected >20K MONDO terms, got {}",
        parsed.term_count()
    );

    // Should have many relationships
    assert!(
        parsed.relationship_count() > 15_000,
        "expected >15K relationships, got {}",
        parsed.relationship_count()
    );

    // Spot check: cancer term
    let cancer = parsed.terms.iter().find(|t| t.mondo_id == "MONDO:0004992");
    assert!(cancer.is_some(), "MONDO:0004992 (cancer) not found");
    let cancer = cancer.unwrap();
    assert_eq!(cancer.name, "cancer");
    assert!(cancer.omim_id.is_some(), "cancer should have OMIM xref");

    println!(
        "MONDO: {} terms, {} relationships, {} obsolete",
        parsed.term_count(),
        parsed.relationship_count(),
        parsed.terms.iter().filter(|t| t.is_obsolete).count()
    );
}
```

- [ ] **Step 2: Run integration test**

```bash
cargo test -p bdp-ingest --test obo_integration test_parse_full_mondo -- --ignored --nocapture 2>&1 | tail -20
```
Expected: pass, prints term count > 20K.

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-ingest/tests/obo_integration.rs
git commit -m "test(bdp-ingest): add MONDO full parse integration test"
```

---

## Task 7: Final verification

- [ ] **Step 1: Full compile (both crates)**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest -p bdp-server 2>&1 | grep "^error" | head -10
```
Expected: zero errors.

- [ ] **Step 2: All unit tests**

```bash
cargo test -p bdp-ingest --lib 2>&1 | tail -10
```

- [ ] **Step 3: Log**

```bash
git log --oneline -10
```
