# ChEBI Chemical Compounds Pipeline Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the ChEBI (Chemical Entities of Biological Interest) ingestion pipeline — DB migration for `compound_terms` and `compound_relationships`, OBO parsing via shared `OboParser`, and full storage via `bdp-ingest`.

**Architecture:** ChEBI uses OBO format. Key extra fields not in GO/MONDO/HPO: InChIKey, SMILES, molecular formula, monoisotopic mass — stored in `compound_terms` columns (extracted from `property_values` in `RawOboTerm`). Follows `registry_entries → data_sources → versions → domain_tables` chain. Runtime `sqlx::query()` only.

**Data source:** `https://ftp.ebi.ac.uk/pub/databases/chebi/ontology/chebi.obo` (~280MB, ~180K terms)

**Note on size:** ChEBI is large (~180K terms). Use chunked inserts (500 per batch). The download is ~280MB — use streaming if possible, but text download with 600s timeout is acceptable since it's gzipped at transfer.

**Tech Stack:** Rust, tokio, sqlx (runtime), reqwest, bdp-ingest common utilities

---

## File Map

**New migration:**
- `migrations/20260326000004_chebi_tables.sql`

**New Rust files in `crates/bdp-ingest/src/pipelines/chebi/`:**
- `mod.rs` — constants, re-exports
- `models.rs` — `CompoundTerm`, `CompoundRelationship`
- `parser.rs` — `RawOboTerm → CompoundTerm` adapter (extracts InChIKey, SMILES from property_values)
- `storage.rs` — `ChebiStorage`
- `runner.rs` — `ChebiPipelineRunner`

**Modified:**
- `crates/bdp-ingest/src/pipelines/mod.rs` — add `pub mod chebi;`

---

## Task 1: DB migration

**Files:**
- Create: `migrations/20260326000004_chebi_tables.sql`

- [ ] **Step 1: Write migration**

```sql
-- migrations/20260326000004_chebi_tables.sql

INSERT INTO source_types (name, label, description)
VALUES ('compound', 'Compound', 'Chemical compounds from ChEBI ontology')
ON CONFLICT (name) DO NOTHING;

CREATE TABLE compound_terms (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id  UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    chebi_id        TEXT NOT NULL,          -- 'CHEBI:33709'
    chebi_accession BIGINT NOT NULL,        -- 33709
    name            TEXT NOT NULL,
    definition      TEXT,
    comment         TEXT,
    is_obsolete     BOOLEAN NOT NULL DEFAULT FALSE,
    -- Chemical identifiers (extracted from OBO property_values)
    inchikey        TEXT,                   -- 'UHOVQNZJYSORNB-UHFFFAOYSA-N'
    smiles          TEXT,                   -- canonical SMILES
    inchi           TEXT,                   -- InChI string
    formula         TEXT,                   -- 'C6H12O6'
    mass_mono       DOUBLE PRECISION,       -- monoisotopic mass
    charge          INTEGER,
    chebi_release   TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_compound_per_release UNIQUE (chebi_id, chebi_release)
);

CREATE INDEX idx_compound_chebi_id    ON compound_terms(chebi_id);
CREATE INDEX idx_compound_accession   ON compound_terms(chebi_accession);
CREATE INDEX idx_compound_inchikey    ON compound_terms(inchikey) WHERE inchikey IS NOT NULL;
CREATE INDEX idx_compound_data_src    ON compound_terms(data_source_id);
CREATE INDEX idx_compound_obsolete    ON compound_terms(is_obsolete) WHERE is_obsolete = FALSE;
CREATE INDEX idx_compound_name_fts    ON compound_terms
    USING GIN (to_tsvector('english', name));

-- Hierarchical and structural relationships
CREATE TABLE compound_relationships (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_chebi_id    TEXT NOT NULL,
    object_chebi_id     TEXT NOT NULL,
    relationship_type   TEXT NOT NULL,   -- 'is_a', 'has_role', 'is_conjugate_acid_of', etc.
    chebi_release       TEXT NOT NULL,
    CONSTRAINT unique_compound_rel UNIQUE (subject_chebi_id, object_chebi_id, relationship_type, chebi_release)
);

CREATE INDEX idx_compound_rel_subject ON compound_relationships(subject_chebi_id);
CREATE INDEX idx_compound_rel_object  ON compound_relationships(object_chebi_id);
CREATE INDEX idx_compound_rel_type    ON compound_relationships(relationship_type);
```

- [ ] **Step 2: Apply + commit**

```bash
cargo xtask db migrate 2>&1 | tail -5
git add migrations/20260326000004_chebi_tables.sql
git commit -m "feat(db): add ChEBI compound_terms and compound_relationships tables"
```

---

## Task 2: Models

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/chebi/mod.rs`
- Create: `crates/bdp-ingest/src/pipelines/chebi/models.rs`

- [ ] **Step 1: Create files**

```rust
// mod.rs
pub mod models;
pub mod parser;
pub mod runner;
pub mod storage;
pub use runner::{ChebiConfig, ChebiPipelineRunner};
pub const CHEBI_OBO_URL: &str = "https://ftp.ebi.ac.uk/pub/databases/chebi/ontology/chebi.obo";
```

```rust
// models.rs
#[derive(Debug, Clone, Default)]
pub struct CompoundTerm {
    pub chebi_id: String,           // "CHEBI:33709"
    pub chebi_accession: i64,
    pub name: String,
    pub definition: Option<String>,
    pub comment: Option<String>,
    pub is_obsolete: bool,
    pub inchikey: Option<String>,
    pub smiles: Option<String>,
    pub inchi: Option<String>,
    pub formula: Option<String>,
    pub mass_mono: Option<f64>,
    pub charge: Option<i32>,
    pub chebi_release: String,
}

#[derive(Debug, Clone)]
pub struct CompoundRelationship {
    pub subject_chebi_id: String,
    pub object_chebi_id: String,
    pub relationship_type: String,
    pub chebi_release: String,
}

#[derive(Debug, Default)]
pub struct ParsedChebi {
    pub terms: Vec<CompoundTerm>,
    pub relationships: Vec<CompoundRelationship>,
}
```

- [ ] **Step 2: Compile check + commit**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -10
git add crates/bdp-ingest/src/pipelines/chebi/
git commit -m "feat(bdp-ingest): add ChEBI domain models"
```

---

## Task 3: Parser (with property_value extraction)

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/chebi/parser.rs`

ChEBI OBO uses `property_value:` lines to store chemical identifiers:
```
property_value: http://purl.obolibrary.org/obo/chebi/inchikey "UHOVQNZJYSORNB-UHFFFAOYSA-N" xsd:string
property_value: http://purl.obolibrary.org/obo/chebi/smiles "c1ccccc1" xsd:string
property_value: http://purl.obolibrary.org/obo/chebi/mass "78.04695" xsd:double
property_value: http://purl.obolibrary.org/obo/chebi/formula "C6H6" xsd:string
property_value: http://purl.obolibrary.org/obo/chebi/charge "0" xsd:integer
property_value: http://purl.obolibrary.org/obo/chebi/inchi "InChI=1S/..." xsd:string
```

The `OboParser` already collects `property_values` as `Vec<(String, String)>`.

- [ ] **Step 1: Create parser.rs**

```rust
// crates/bdp-ingest/src/pipelines/chebi/parser.rs

use crate::common::obo::{OboParser, OboParseError};
use crate::pipelines::chebi::models::*;

pub fn parse_obo(content: &str, release: &str, limit: Option<usize>) -> Result<ParsedChebi, OboParseError> {
    let raw = OboParser::parse(content, limit)?;
    let mut parsed = ParsedChebi::default();

    for raw_term in raw {
        if !raw_term.id.starts_with("CHEBI:") {
            continue;
        }

        let accession: i64 = raw_term.id
            .trim_start_matches("CHEBI:")
            .parse()
            .unwrap_or(0);

        // Extract chemical properties from property_values
        let mut term = CompoundTerm {
            chebi_id: raw_term.id.clone(),
            chebi_accession: accession,
            name: raw_term.name.clone(),
            definition: raw_term.definition.clone(),
            comment: raw_term.comment.clone(),
            is_obsolete: raw_term.is_obsolete,
            chebi_release: release.to_string(),
            ..Default::default()
        };

        for (key, value) in &raw_term.property_values {
            // ChEBI uses full URIs or short names
            let short_key = key.rsplit('/').next().unwrap_or(key);
            match short_key {
                "inchikey" => term.inchikey = Some(value.clone()),
                "smiles" => term.smiles = Some(value.clone()),
                "inchi" => term.inchi = Some(value.clone()),
                "formula" => term.formula = Some(value.clone()),
                "mass" | "monoisotopicmass" => {
                    term.mass_mono = value.parse().ok();
                }
                "charge" => {
                    term.charge = value.parse().ok();
                }
                _ => {}
            }
        }

        parsed.terms.push(term);

        // is_a relationships
        for parent_id in &raw_term.is_a {
            if !parent_id.starts_with("CHEBI:") { continue; }
            parsed.relationships.push(CompoundRelationship {
                subject_chebi_id: raw_term.id.clone(),
                object_chebi_id: parent_id.clone(),
                relationship_type: "is_a".to_string(),
                chebi_release: release.to_string(),
            });
        }

        // Other relationships (has_role, is_conjugate_acid_of, etc.)
        for rel in &raw_term.relationships {
            if !rel.target.starts_with("CHEBI:") { continue; }
            parsed.relationships.push(CompoundRelationship {
                subject_chebi_id: raw_term.id.clone(),
                object_chebi_id: rel.target.clone(),
                relationship_type: rel.rel_type.clone(),
                chebi_release: release.to_string(),
            });
        }
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
format-version: 1.2
ontology: chebi

[Term]
id: CHEBI:33709
name: amino acid
def: "Organic acid with amino group." [CHEBI]
is_a: CHEBI:25703 ! organic acid
property_value: http://purl.obolibrary.org/obo/chebi/formula "C2H5NO2" xsd:string
property_value: http://purl.obolibrary.org/obo/chebi/mass "75.032" xsd:double
property_value: http://purl.obolibrary.org/obo/chebi/inchikey "DHMQDGOQFOQNFH-UHFFFAOYSA-N" xsd:string

[Term]
id: CHEBI:25703
name: organic acid
"#;

    #[test]
    fn test_parse_compound() {
        let parsed = parse_obo(SAMPLE, "2026-03-01", None).unwrap();
        assert_eq!(parsed.terms.len(), 2);

        let aa = parsed.terms.iter().find(|t| t.chebi_id == "CHEBI:33709").unwrap();
        assert_eq!(aa.name, "amino acid");
        assert_eq!(aa.formula.as_deref(), Some("C2H5NO2"));
        assert!(aa.mass_mono.is_some());
        assert_eq!(aa.inchikey.as_deref(), Some("DHMQDGOQFOQNFH-UHFFFAOYSA-N"));

        assert_eq!(parsed.relationships.len(), 1);
        assert_eq!(parsed.relationships[0].subject_chebi_id, "CHEBI:33709");
        assert_eq!(parsed.relationships[0].object_chebi_id, "CHEBI:25703");
        assert_eq!(parsed.relationships[0].relationship_type, "is_a");
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p bdp-ingest pipelines::chebi::parser 2>&1 | tail -15
```
Expected: pass.

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-ingest/src/pipelines/chebi/parser.rs
git commit -m "feat(bdp-ingest): add ChEBI OBO parser with chemical property extraction"
```

---

## Task 4: Storage layer

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/chebi/storage.rs`

- [ ] **Step 1: Create storage.rs**

```rust
// crates/bdp-ingest/src/pipelines/chebi/storage.rs

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction};
use tracing::info;
use uuid::Uuid;

use crate::common::batch::BatchConfig;
use crate::pipelines::chebi::models::*;

pub struct ChebiStorage {
    pool: PgPool,
    batch: BatchConfig,
}

impl ChebiStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool, batch: BatchConfig::new(200) }  // smaller chunks for large ChEBI
    }

    pub async fn ingest_release(&self, org_id: Uuid, release: &str, parsed: &ParsedChebi) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let data_source_id = self.upsert_registry(&mut tx, org_id).await?;
        self.upsert_version(&mut tx, data_source_id, release).await?;

        info!(count = parsed.terms.len(), "storing ChEBI terms");
        self.store_terms(&mut tx, data_source_id, &parsed.terms).await?;

        info!(count = parsed.relationships.len(), "storing ChEBI relationships");
        self.store_relationships(&mut tx, &parsed.relationships).await?;

        tx.commit().await?;
        info!(release, "ChEBI ingest complete");
        Ok(())
    }

    async fn upsert_registry(&self, tx: &mut Transaction<'_, Postgres>, org_id: Uuid) -> Result<Uuid> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO registry_entries (organization_id, slug, name, entry_type)
             VALUES ($1, 'chebi', 'ChEBI Chemical Entities of Biological Interest', 'data_source')
             ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name RETURNING id"
        ).bind(org_id).fetch_one(&mut **tx).await?;

        sqlx::query("INSERT INTO data_sources (id, source_type, external_id) VALUES ($1, 'compound', 'chebi') ON CONFLICT (id) DO NOTHING")
            .bind(id).execute(&mut **tx).await?;
        Ok(id)
    }

    async fn upsert_version(&self, tx: &mut Transaction<'_, Postgres>, ds_id: Uuid, release: &str) -> Result<()> {
        sqlx::query("INSERT INTO versions (entry_id, version, release_date) VALUES ($1, $2, CURRENT_DATE) ON CONFLICT (entry_id, version) DO NOTHING")
            .bind(ds_id).bind(release).execute(&mut **tx).await?;
        Ok(())
    }

    async fn store_terms(&self, tx: &mut Transaction<'_, Postgres>, ds_id: Uuid, terms: &[CompoundTerm]) -> Result<()> {
        for chunk in terms.chunks(self.batch.chunk_size) {
            for t in chunk {
                sqlx::query(
                    "INSERT INTO compound_terms
                     (data_source_id, chebi_id, chebi_accession, name, definition, comment,
                      is_obsolete, inchikey, smiles, inchi, formula, mass_mono, charge, chebi_release)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
                     ON CONFLICT (chebi_id, chebi_release)
                     DO UPDATE SET name=EXCLUDED.name, inchikey=EXCLUDED.inchikey,
                                   smiles=EXCLUDED.smiles, formula=EXCLUDED.formula,
                                   mass_mono=EXCLUDED.mass_mono, is_obsolete=EXCLUDED.is_obsolete"
                )
                .bind(ds_id).bind(&t.chebi_id).bind(t.chebi_accession).bind(&t.name)
                .bind(&t.definition).bind(&t.comment).bind(t.is_obsolete)
                .bind(&t.inchikey).bind(&t.smiles).bind(&t.inchi)
                .bind(&t.formula).bind(t.mass_mono).bind(t.charge).bind(&t.chebi_release)
                .execute(&mut **tx).await.context("insert compound term")?;
            }
        }
        Ok(())
    }

    async fn store_relationships(&self, tx: &mut Transaction<'_, Postgres>, rels: &[CompoundRelationship]) -> Result<()> {
        for chunk in rels.chunks(self.batch.chunk_size) {
            for r in chunk {
                sqlx::query(
                    "INSERT INTO compound_relationships (subject_chebi_id, object_chebi_id, relationship_type, chebi_release)
                     VALUES ($1,$2,$3,$4) ON CONFLICT (subject_chebi_id, object_chebi_id, relationship_type, chebi_release) DO NOTHING"
                )
                .bind(&r.subject_chebi_id).bind(&r.object_chebi_id).bind(&r.relationship_type).bind(&r.chebi_release)
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
git add crates/bdp-ingest/src/pipelines/chebi/storage.rs
git commit -m "feat(bdp-ingest): add ChEBI storage layer"
```

---

## Task 5: Runner + registration

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/chebi/runner.rs`
- Modify: `crates/bdp-ingest/src/pipelines/mod.rs`

- [ ] **Step 1: Create runner.rs**

```rust
// crates/bdp-ingest/src/pipelines/chebi/runner.rs

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::common::http::download_text;
use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::chebi::{parser, storage::ChebiStorage, CHEBI_OBO_URL};

#[derive(Debug, Clone)]
pub struct ChebiConfig {
    pub obo_url: String,
    pub max_retries: u32,
    pub release: String,
    pub org_id: Uuid,
    pub parse_limit: Option<usize>,
}

impl ChebiConfig {
    pub fn new(release: impl Into<String>, org_id: Uuid) -> Self {
        Self {
            obo_url: CHEBI_OBO_URL.to_string(),
            max_retries: 3,
            release: release.into(),
            org_id,
            parse_limit: None,
        }
    }
}

pub struct ChebiPipelineRunner {
    config: ChebiConfig,
    pool: PgPool,
}

impl ChebiPipelineRunner {
    pub fn new(config: ChebiConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }
}

impl PipelineRunner for ChebiPipelineRunner {
    fn name(&self) -> &'static str { "chebi" }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());

        info!("downloading ChEBI OBO (~280MB)");
        let content = download_text(&self.config.obo_url, self.config.max_retries).await?;

        info!("parsing ChEBI OBO ({} bytes)", content.len());
        let parsed = parser::parse_obo(&content, &self.config.release, self.config.parse_limit)?;

        stats.records_ingested = parsed.terms.len() as u64;
        stats.records_skipped = parsed.terms.iter().filter(|t| t.is_obsolete).count() as u64;

        info!(terms = parsed.terms.len(), rels = parsed.relationships.len(), "ChEBI parsed");

        let storage = ChebiStorage::new(self.pool);
        storage.ingest_release(self.config.org_id, &self.config.release, &parsed).await?;

        Ok(stats)
    }
}
```

- [ ] **Step 2: Add to pipelines/mod.rs**

```rust
pub mod chebi;
```

- [ ] **Step 3: Integration test**

Add to `crates/bdp-ingest/tests/obo_integration.rs`:

```rust
#[tokio::test]
#[ignore = "downloads ~280MB from EBI FTP"]
async fn test_parse_chebi_sample() {
    use bdp_ingest::pipelines::chebi::parser;

    // Parse only first 1000 terms to keep test fast
    let url = "https://ftp.ebi.ac.uk/pub/databases/chebi/ontology/chebi.obo";
    let content = bdp_ingest::common::http::download_text(url, 3).await.unwrap();
    let parsed = parser::parse_obo(&content, "test", Some(1000)).unwrap();

    assert_eq!(parsed.terms.len(), 1000, "expected 1000 ChEBI terms");

    // Check that InChIKey extraction works on real data
    let with_inchikey: Vec<_> = parsed.terms.iter().filter(|t| t.inchikey.is_some()).collect();
    assert!(!with_inchikey.is_empty(), "expected some terms with InChIKey");

    println!(
        "ChEBI sample: {} terms, {} with InChIKey, {} rels",
        parsed.terms.len(),
        with_inchikey.len(),
        parsed.relationships.len()
    );
}
```

Run:
```bash
cargo test -p bdp-ingest --test obo_integration test_parse_chebi_sample -- --ignored --nocapture 2>&1 | tail -10
```
Expected: 1000 terms parsed, some with InChIKey.

- [ ] **Step 4: All unit tests pass**

```bash
cargo test -p bdp-ingest --lib 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add crates/bdp-ingest/src/pipelines/chebi/runner.rs \
        crates/bdp-ingest/src/pipelines/mod.rs \
        crates/bdp-ingest/tests/obo_integration.rs
git commit -m "feat(bdp-ingest): complete ChEBI pipeline — compounds with InChIKey/SMILES"
```

---

## Task 6: Final verification

- [ ] **Step 1: Full compile**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest -p bdp-server 2>&1 | grep "^error" | head -10
```

- [ ] **Step 2: All tests**

```bash
cargo test -p bdp-ingest --lib 2>&1 | tail -10
```

- [ ] **Step 3: Log**

```bash
git log --oneline -12
```
