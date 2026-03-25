# HPO Phenotype Ontology Pipeline Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Human Phenotype Ontology (HPO) ingestion pipeline — DB migration for `phenotype_terms`, `phenotype_hierarchy`, `gene_disease_associations`, and `disease_phenotype_associations`, plus parsing and storage via `bdp-ingest`.

**Architecture:** HPO ships two files:
1. `hp.obo` — phenotype terms (OBO format, parsed via shared `OboParser`)
2. `phenotype.hpoa` — disease-phenotype annotations (TSV, parsed separately)

Both are stored. Follows `registry_entries → data_sources → versions → domain_tables` chain. Runtime `sqlx::query()` only.

**Data sources:**
- `https://purl.obolibrary.org/obo/hp.obo` (~18K terms)
- `https://purl.obolibrary.org/obo/hp/hpoa/phenotype.hpoa` (~270K disease-phenotype rows)

**Tech Stack:** Rust, tokio, sqlx (runtime), reqwest, bdp-ingest common utilities

---

## File Map

**New migration:**
- `migrations/20260326000003_hpo_tables.sql`

**New Rust files in `crates/bdp-ingest/src/pipelines/hpo/`:**
- `mod.rs` — constants, re-exports
- `models.rs` — `PhenotypeTerm`, `PhenotypeRelationship`, `DiseasePhenotypeAnnotation`
- `parser.rs` — OBO adapter (`RawOboTerm → PhenotypeTerm`) + HPOA TSV parser
- `storage.rs` — `HpoStorage`: writes all HPO tables
- `runner.rs` — `HpoPipelineRunner`

**Modified:**
- `crates/bdp-ingest/src/pipelines/mod.rs` — add `pub mod hpo;`

---

## Task 1: DB migration

**Files:**
- Create: `migrations/20260326000003_hpo_tables.sql`

- [ ] **Step 1: Write migration**

```sql
-- migrations/20260326000003_hpo_tables.sql

INSERT INTO source_types (name, label, description)
VALUES ('phenotype', 'Phenotype', 'Human Phenotype Ontology (HPO) terms and annotations')
ON CONFLICT (name) DO NOTHING;

-- Phenotype terms (HPO)
CREATE TABLE phenotype_terms (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id  UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    hpo_id          TEXT NOT NULL,           -- 'HP:0000001'
    hpo_accession   BIGINT NOT NULL,          -- 1
    name            TEXT NOT NULL,
    definition      TEXT,
    comment         TEXT,
    is_obsolete     BOOLEAN NOT NULL DEFAULT FALSE,
    -- Category denormalized for fast filtering
    category        TEXT,                    -- 'Abnormality of the nervous system', etc.
    hpo_release     TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_phenotype_per_release UNIQUE (hpo_id, hpo_release)
);

CREATE INDEX idx_phenotype_hpo_id     ON phenotype_terms(hpo_id);
CREATE INDEX idx_phenotype_accession  ON phenotype_terms(hpo_accession);
CREATE INDEX idx_phenotype_data_src   ON phenotype_terms(data_source_id);
CREATE INDEX idx_phenotype_obsolete   ON phenotype_terms(is_obsolete) WHERE is_obsolete = FALSE;
CREATE INDEX idx_phenotype_name_fts   ON phenotype_terms
    USING GIN (to_tsvector('english', name));

-- HPO hierarchy (parent-child relationships)
CREATE TABLE phenotype_hierarchy (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_hpo_id   TEXT NOT NULL,
    child_hpo_id    TEXT NOT NULL,
    relationship_type TEXT NOT NULL DEFAULT 'is_a',
    hpo_release     TEXT NOT NULL,
    CONSTRAINT unique_phenotype_hier UNIQUE (parent_hpo_id, child_hpo_id, hpo_release)
);

CREATE INDEX idx_phenotype_hier_parent ON phenotype_hierarchy(parent_hpo_id);
CREATE INDEX idx_phenotype_hier_child  ON phenotype_hierarchy(child_hpo_id);

-- Disease-phenotype annotations (from phenotype.hpoa)
CREATE TABLE disease_phenotype_associations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- disease side (OMIM/ORPHA ID, resolved when MONDO is loaded)
    disease_db      TEXT NOT NULL,           -- 'OMIM', 'ORPHA'
    disease_id      TEXT NOT NULL,           -- '114500', '68335'
    disease_name    TEXT,
    -- phenotype side
    hpo_id          TEXT NOT NULL,
    -- annotation metadata
    qualifier       TEXT,                    -- 'NOT' if excluded phenotype
    frequency_hpo   TEXT,                   -- 'HP:0040280' (Obligate) → 'HP:0040285' (Excluded)
    onset_hpo       TEXT,                   -- HPO onset term
    evidence_code   TEXT NOT NULL,           -- 'IEA', 'PCS', 'TAS'
    source          TEXT,                    -- 'OMIM:123456', 'PMID:12345'
    aspect          TEXT,                    -- 'P'=phenotype, 'I'=inheritance, 'C'=onset, 'M'=modifier
    hpoa_release    TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_disease_phenotype UNIQUE (disease_db, disease_id, hpo_id, evidence_code, hpoa_release)
);

CREATE INDEX idx_dpa_disease    ON disease_phenotype_associations(disease_db, disease_id);
CREATE INDEX idx_dpa_phenotype  ON disease_phenotype_associations(hpo_id);
CREATE INDEX idx_dpa_qualifier  ON disease_phenotype_associations(qualifier) WHERE qualifier IS NOT NULL;
CREATE INDEX idx_dpa_aspect     ON disease_phenotype_associations(aspect);
```

- [ ] **Step 2: Apply migration (if DB available)**

```bash
cargo xtask db migrate 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
git add migrations/20260326000003_hpo_tables.sql
git commit -m "feat(db): add HPO phenotype_terms, phenotype_hierarchy, disease_phenotype_associations"
```

---

## Task 2: Domain models

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/hpo/mod.rs`
- Create: `crates/bdp-ingest/src/pipelines/hpo/models.rs`

- [ ] **Step 1: Create mod.rs**

```rust
pub mod models;
pub mod parser;
pub mod runner;
pub mod storage;

pub use runner::{HpoConfig, HpoPipelineRunner};

pub const HPO_OBO_URL: &str = "https://purl.obolibrary.org/obo/hp.obo";
pub const HPO_ANNOTATIONS_URL: &str = "https://purl.obolibrary.org/obo/hp/hpoa/phenotype.hpoa";
```

- [ ] **Step 2: Create models.rs**

```rust
// crates/bdp-ingest/src/pipelines/hpo/models.rs

#[derive(Debug, Clone)]
pub struct PhenotypeTerm {
    pub hpo_id: String,           // "HP:0000001"
    pub hpo_accession: i64,       // 1
    pub name: String,
    pub definition: Option<String>,
    pub comment: Option<String>,
    pub is_obsolete: bool,
    pub category: Option<String>,
    pub hpo_release: String,
}

#[derive(Debug, Clone)]
pub struct PhenotypeRelationship {
    pub parent_hpo_id: String,
    pub child_hpo_id: String,
    pub relationship_type: String,  // 'is_a'
    pub hpo_release: String,
}

#[derive(Debug, Clone)]
pub struct DiseasePhenotypeAnnotation {
    pub disease_db: String,         // 'OMIM', 'ORPHA'
    pub disease_id: String,         // '114500'
    pub disease_name: Option<String>,
    pub hpo_id: String,
    pub qualifier: Option<String>,  // 'NOT'
    pub frequency_hpo: Option<String>,
    pub onset_hpo: Option<String>,
    pub evidence_code: String,
    pub source: Option<String>,
    pub aspect: Option<String>,
    pub hpoa_release: String,
}

#[derive(Debug, Default)]
pub struct ParsedHpo {
    pub terms: Vec<PhenotypeTerm>,
    pub relationships: Vec<PhenotypeRelationship>,
    pub annotations: Vec<DiseasePhenotypeAnnotation>,
}
```

- [ ] **Step 3: Compile check + commit**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -10
git add crates/bdp-ingest/src/pipelines/hpo/
git commit -m "feat(bdp-ingest): add HPO domain models"
```

---

## Task 3: Parser (OBO terms + HPOA annotations)

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/hpo/parser.rs`

- [ ] **Step 1: Create parser.rs**

```rust
// crates/bdp-ingest/src/pipelines/hpo/parser.rs

use crate::common::obo::{OboParser, OboParseError};
use crate::pipelines::hpo::models::*;
use anyhow::Result;

/// Parse hp.obo content into phenotype terms and hierarchy relationships.
pub fn parse_obo(content: &str, release: &str, limit: Option<usize>) -> Result<(Vec<PhenotypeTerm>, Vec<PhenotypeRelationship>), OboParseError> {
    let raw = OboParser::parse(content, limit)?;
    let mut terms = Vec::new();
    let mut rels = Vec::new();

    for raw_term in raw {
        if !raw_term.id.starts_with("HP:") {
            continue;
        }

        let accession: i64 = raw_term.id
            .trim_start_matches("HP:")
            .parse()
            .unwrap_or(0);

        terms.push(PhenotypeTerm {
            hpo_id: raw_term.id.clone(),
            hpo_accession: accession,
            name: raw_term.name.clone(),
            definition: raw_term.definition.clone(),
            comment: raw_term.comment.clone(),
            is_obsolete: raw_term.is_obsolete,
            category: None,  // set during hierarchy traversal if needed
            hpo_release: release.to_string(),
        });

        for parent_id in &raw_term.is_a {
            if !parent_id.starts_with("HP:") { continue; }
            rels.push(PhenotypeRelationship {
                parent_hpo_id: parent_id.clone(),
                child_hpo_id: raw_term.id.clone(),
                relationship_type: "is_a".to_string(),
                hpo_release: release.to_string(),
            });
        }
    }

    Ok((terms, rels))
}

/// Parse phenotype.hpoa TSV annotation file.
///
/// Format (tab-separated, lines starting with '#' are comments):
/// database_id  disease_name  qualifier  hpo_id  reference  evidence  onset  frequency  sex  modifier  aspect  biocuration
pub fn parse_hpoa(content: &str, release: &str) -> Result<Vec<DiseasePhenotypeAnnotation>> {
    let mut annotations = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 8 {
            continue;
        }

        // database_id is like "OMIM:114500" or "ORPHA:68335"
        let (disease_db, disease_id) = if let Some(colon) = cols[0].find(':') {
            (cols[0][..colon].to_string(), cols[0][colon+1..].to_string())
        } else {
            continue;
        };

        let qualifier = if cols[2].is_empty() { None } else { Some(cols[2].to_string()) };
        let hpo_id = cols[3].to_string();
        if !hpo_id.starts_with("HP:") { continue; }

        let evidence_code = cols[5].to_string();
        let onset_hpo = if cols[6].is_empty() { None } else { Some(cols[6].to_string()) };
        let frequency_hpo = if cols[7].is_empty() { None } else {
            // frequency field may be a percentage string or HP: term
            if cols[7].starts_with("HP:") { Some(cols[7].to_string()) } else { None }
        };

        let source = cols[4].split(';').next().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let aspect = cols.get(10).map(|s| s.to_string()).filter(|s| !s.is_empty());

        annotations.push(DiseasePhenotypeAnnotation {
            disease_db,
            disease_id,
            disease_name: if cols[1].is_empty() { None } else { Some(cols[1].to_string()) },
            hpo_id,
            qualifier,
            frequency_hpo,
            onset_hpo,
            evidence_code,
            source,
            aspect,
            hpoa_release: release.to_string(),
        });
    }

    Ok(annotations)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_OBO: &str = r#"
format-version: 1.2
ontology: hp

[Term]
id: HP:0000001
name: All
comment: Root of all HPO terms.

[Term]
id: HP:0000118
name: Phenotypic abnormality
is_a: HP:0000001 ! All
"#;

    #[test]
    fn test_parse_obo_terms() {
        let (terms, rels) = parse_obo(SAMPLE_OBO, "2026-03-01", None).unwrap();
        assert_eq!(terms.len(), 2);
        assert_eq!(rels.len(), 1);
        let root = terms.iter().find(|t| t.hpo_id == "HP:0000001").unwrap();
        assert_eq!(root.name, "All");
        assert_eq!(rels[0].parent_hpo_id, "HP:0000001");
        assert_eq!(rels[0].child_hpo_id, "HP:0000118");
    }

    #[test]
    fn test_parse_hpoa_line() {
        let sample = "OMIM:114500\tcancer\t\tHP:0002664\tOMIM:114500\tTAS\tHP:0030674\tHP:0040281\t\t\tP\tHPO:probinson[2022-01-01]";
        let annotations = parse_hpoa(sample, "2026-03-01").unwrap();
        assert_eq!(annotations.len(), 1);
        let ann = &annotations[0];
        assert_eq!(ann.disease_db, "OMIM");
        assert_eq!(ann.disease_id, "114500");
        assert_eq!(ann.hpo_id, "HP:0002664");
        assert_eq!(ann.evidence_code, "TAS");
        assert!(ann.qualifier.is_none());
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p bdp-ingest pipelines::hpo::parser 2>&1 | tail -20
```
Expected: both tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-ingest/src/pipelines/hpo/parser.rs
git commit -m "feat(bdp-ingest): add HPO OBO parser and HPOA annotation parser"
```

---

## Task 4: Storage layer

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/hpo/storage.rs`

- [ ] **Step 1: Create storage.rs**

```rust
// crates/bdp-ingest/src/pipelines/hpo/storage.rs

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction};
use tracing::info;
use uuid::Uuid;

use crate::common::batch::BatchConfig;
use crate::pipelines::hpo::models::*;

pub struct HpoStorage {
    pool: PgPool,
    batch: BatchConfig,
}

impl HpoStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool, batch: BatchConfig::default() }
    }

    pub async fn ingest_release(
        &self,
        org_id: Uuid,
        hpo_release: &str,
        terms: &[PhenotypeTerm],
        relationships: &[PhenotypeRelationship],
        annotations: &[DiseasePhenotypeAnnotation],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.context("begin tx")?;

        let data_source_id = self.upsert_registry(&mut tx, org_id).await?;
        self.upsert_version(&mut tx, data_source_id, hpo_release).await?;

        info!(count = terms.len(), "storing HPO terms");
        self.store_terms(&mut tx, data_source_id, terms).await?;

        info!(count = relationships.len(), "storing HPO hierarchy");
        self.store_relationships(&mut tx, relationships).await?;

        info!(count = annotations.len(), "storing disease-phenotype annotations");
        self.store_annotations(&mut tx, annotations).await?;

        tx.commit().await.context("commit tx")?;
        info!(hpo_release, "HPO ingest complete");
        Ok(())
    }

    async fn upsert_registry(&self, tx: &mut Transaction<'_, Postgres>, org_id: Uuid) -> Result<Uuid> {
        let registry_id: Uuid = sqlx::query_scalar(
            "INSERT INTO registry_entries (organization_id, slug, name, entry_type)
             VALUES ($1, 'hpo', 'Human Phenotype Ontology', 'data_source')
             ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name RETURNING id"
        ).bind(org_id).fetch_one(&mut **tx).await?;

        sqlx::query(
            "INSERT INTO data_sources (id, source_type, external_id)
             VALUES ($1, 'phenotype', 'hpo')
             ON CONFLICT (id) DO NOTHING"
        ).bind(registry_id).execute(&mut **tx).await?;

        Ok(registry_id)
    }

    async fn upsert_version(&self, tx: &mut Transaction<'_, Postgres>, data_source_id: Uuid, release: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO versions (entry_id, version, release_date)
             VALUES ($1, $2, CURRENT_DATE)
             ON CONFLICT (entry_id, version) DO UPDATE SET release_date = EXCLUDED.release_date"
        ).bind(data_source_id).bind(release).execute(&mut **tx).await?;
        Ok(())
    }

    async fn store_terms(&self, tx: &mut Transaction<'_, Postgres>, data_source_id: Uuid, terms: &[PhenotypeTerm]) -> Result<()> {
        for chunk in terms.chunks(self.batch.chunk_size) {
            for term in chunk {
                sqlx::query(
                    "INSERT INTO phenotype_terms
                     (data_source_id, hpo_id, hpo_accession, name, definition, comment, is_obsolete, hpo_release)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                     ON CONFLICT (hpo_id, hpo_release)
                     DO UPDATE SET name = EXCLUDED.name, definition = EXCLUDED.definition,
                                   is_obsolete = EXCLUDED.is_obsolete"
                )
                .bind(data_source_id)
                .bind(&term.hpo_id)
                .bind(term.hpo_accession)
                .bind(&term.name)
                .bind(&term.definition)
                .bind(&term.comment)
                .bind(term.is_obsolete)
                .bind(&term.hpo_release)
                .execute(&mut **tx).await
                .context("insert phenotype term")?;
            }
        }
        Ok(())
    }

    async fn store_relationships(&self, tx: &mut Transaction<'_, Postgres>, rels: &[PhenotypeRelationship]) -> Result<()> {
        for chunk in rels.chunks(self.batch.chunk_size) {
            for rel in chunk {
                sqlx::query(
                    "INSERT INTO phenotype_hierarchy (parent_hpo_id, child_hpo_id, relationship_type, hpo_release)
                     VALUES ($1, $2, $3, $4)
                     ON CONFLICT (parent_hpo_id, child_hpo_id, hpo_release) DO NOTHING"
                )
                .bind(&rel.parent_hpo_id)
                .bind(&rel.child_hpo_id)
                .bind(&rel.relationship_type)
                .bind(&rel.hpo_release)
                .execute(&mut **tx).await?;
            }
        }
        Ok(())
    }

    async fn store_annotations(&self, tx: &mut Transaction<'_, Postgres>, anns: &[DiseasePhenotypeAnnotation]) -> Result<()> {
        for chunk in anns.chunks(self.batch.chunk_size) {
            for ann in chunk {
                sqlx::query(
                    "INSERT INTO disease_phenotype_associations
                     (disease_db, disease_id, disease_name, hpo_id, qualifier, frequency_hpo,
                      onset_hpo, evidence_code, source, aspect, hpoa_release)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                     ON CONFLICT (disease_db, disease_id, hpo_id, evidence_code, hpoa_release)
                     DO NOTHING"
                )
                .bind(&ann.disease_db).bind(&ann.disease_id).bind(&ann.disease_name)
                .bind(&ann.hpo_id).bind(&ann.qualifier).bind(&ann.frequency_hpo)
                .bind(&ann.onset_hpo).bind(&ann.evidence_code).bind(&ann.source)
                .bind(&ann.aspect).bind(&ann.hpoa_release)
                .execute(&mut **tx).await?;
            }
        }
        Ok(())
    }
}
```

- [ ] **Step 2: Compile check + commit**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -10
git add crates/bdp-ingest/src/pipelines/hpo/storage.rs
git commit -m "feat(bdp-ingest): add HPO storage layer"
```

---

## Task 5: Runner + module registration

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/hpo/runner.rs`
- Modify: `crates/bdp-ingest/src/pipelines/mod.rs`

- [ ] **Step 1: Create runner.rs**

```rust
// crates/bdp-ingest/src/pipelines/hpo/runner.rs

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::common::http::download_text;
use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::hpo::{parser, storage::HpoStorage, HPO_OBO_URL, HPO_ANNOTATIONS_URL};

#[derive(Debug, Clone)]
pub struct HpoConfig {
    pub obo_url: String,
    pub annotations_url: String,
    pub max_retries: u32,
    pub release: String,
    pub org_id: Uuid,
}

impl HpoConfig {
    pub fn new(release: impl Into<String>, org_id: Uuid) -> Self {
        Self {
            obo_url: HPO_OBO_URL.to_string(),
            annotations_url: HPO_ANNOTATIONS_URL.to_string(),
            max_retries: 3,
            release: release.into(),
            org_id,
        }
    }
}

pub struct HpoPipelineRunner {
    config: HpoConfig,
    pool: PgPool,
}

impl HpoPipelineRunner {
    pub fn new(config: HpoConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }
}

impl PipelineRunner for HpoPipelineRunner {
    fn name(&self) -> &'static str { "hpo" }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());

        // 1. Download and parse OBO
        info!("downloading hp.obo");
        let obo_content = download_text(&self.config.obo_url, self.config.max_retries).await?;
        let (terms, relationships) = parser::parse_obo(&obo_content, &self.config.release, None)?;

        // 2. Download and parse HPOA annotations
        info!("downloading phenotype.hpoa");
        let hpoa_content = download_text(&self.config.annotations_url, self.config.max_retries).await?;
        let annotations = parser::parse_hpoa(&hpoa_content, &self.config.release)?;

        stats.records_ingested = (terms.len() + annotations.len()) as u64;
        stats.records_skipped = terms.iter().filter(|t| t.is_obsolete).count() as u64;

        info!(terms = terms.len(), rels = relationships.len(), annotations = annotations.len(), "HPO parsed");

        // 3. Store everything
        let storage = HpoStorage::new(self.pool);
        storage.ingest_release(self.config.org_id, &self.config.release, &terms, &relationships, &annotations).await?;

        Ok(stats)
    }
}
```

- [ ] **Step 2: Add to pipelines/mod.rs**

```rust
pub mod hpo;
```

- [ ] **Step 3: Run all tests**

```bash
cargo test -p bdp-ingest --lib 2>&1 | tail -20
```

- [ ] **Step 4: Integration test (run with --ignored)**

```bash
cargo test -p bdp-ingest --test obo_integration -- --ignored --nocapture 2>&1 | tail -20
```

Add an HPO integration test to `crates/bdp-ingest/tests/obo_integration.rs`:

```rust
#[tokio::test]
#[ignore = "downloads from internet"]
async fn test_parse_full_hpo() {
    use bdp_ingest::pipelines::hpo::parser;

    let url = "https://purl.obolibrary.org/obo/hp.obo";
    let content = bdp_ingest::common::http::download_text(url, 3).await.unwrap();
    let (terms, rels) = parser::parse_obo(&content, "test", None).unwrap();

    // HPO has ~18K terms
    assert!(terms.len() > 15_000, "expected >15K HPO terms, got {}", terms.len());
    assert!(rels.len() > 15_000, "expected >15K HPO relationships, got {}", rels.len());

    // Root term
    let root = terms.iter().find(|t| t.hpo_id == "HP:0000001");
    assert!(root.is_some(), "HP:0000001 (root) not found");

    println!("HPO: {} terms, {} relationships", terms.len(), rels.len());
}
```

Run the test:
```bash
cargo test -p bdp-ingest --test obo_integration test_parse_full_hpo -- --ignored --nocapture 2>&1 | tail -10
```
Expected: pass with term count > 15K.

- [ ] **Step 5: Commit**

```bash
git add crates/bdp-ingest/src/pipelines/hpo/runner.rs \
        crates/bdp-ingest/src/pipelines/mod.rs \
        crates/bdp-ingest/tests/obo_integration.rs
git commit -m "feat(bdp-ingest): complete HPO pipeline — terms, hierarchy, disease-phenotype annotations"
```

---

## Task 6: Final verification

- [ ] **Step 1: Full compile**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest -p bdp-server 2>&1 | grep "^error" | head -10
```

- [ ] **Step 2: All unit tests**

```bash
cargo test -p bdp-ingest --lib 2>&1 | tail -10
```

- [ ] **Step 3: Log**

```bash
git log --oneline -10
```
