# Reactome Pathway Pipeline Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Reactome biological pathways ingestion pipeline — DB migration for `pathway_terms`, `pathway_hierarchy`, and `protein_pathway_associations`, plus TSV-based parsing and full storage via `bdp-ingest`.

**Architecture:** Reactome does NOT use OBO format. It provides two TSV files:
1. `ReactomePathways.txt` — pathway ID, name, species (tab-separated, 3 columns)
2. `UniProt2Reactome.txt` — UniProt accession, pathway ID, URL, pathway name, evidence, species (6 columns)

The `protein_pathway_associations` table is the key **typed edge** between proteins and pathways. This is NOT a generic `graph_edges` row — it's a domain-typed association table as specified in the architecture spec. Follows the `registry_entries → data_sources → versions → domain_tables` chain. Runtime `sqlx::query()` only.

**Data sources:**
- `https://reactome.org/download/current/ReactomePathways.txt` (~27K pathways)
- `https://reactome.org/download/current/UniProt2Reactome.txt` (~1M protein→pathway mappings, human only: ~500K)

**Tech Stack:** Rust, tokio, sqlx (runtime), reqwest, bdp-ingest common utilities

---

## File Map

**New migration:**
- `migrations/20260326000001_reactome_tables.sql`

**New Rust files in `crates/bdp-ingest/src/pipelines/reactome/`:**
- `mod.rs` — constants, re-exports
- `models.rs` — `Pathway`, `ProteinPathwayLink`
- `parser.rs` — TSV parsers for both files
- `storage.rs` — `ReactomeStorage`
- `runner.rs` — `ReactomePipelineRunner`

**Modified:**
- `crates/bdp-ingest/src/pipelines/mod.rs` — add `pub mod reactome;`

---

## Task 1: DB migration

**Files:**
- Create: `migrations/20260326000001_reactome_tables.sql`

- [ ] **Step 1: Write migration**

```sql
-- migrations/20260326000001_reactome_tables.sql

-- 'pathway' source type may already exist from seed — INSERT OR IGNORE
INSERT INTO source_types (name, label, description)
VALUES ('pathway', 'Pathway', 'Biological pathways from Reactome')
ON CONFLICT (name) DO NOTHING;

-- Biological pathway terms
CREATE TABLE pathway_terms (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id  UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    reactome_id     TEXT NOT NULL,       -- 'R-HSA-9612973'
    name            TEXT NOT NULL,
    species_name    TEXT NOT NULL,       -- 'Homo sapiens'
    species_taxid   BIGINT,             -- 9606 (populated from NCBI taxonomy if available)
    is_top_level    BOOLEAN NOT NULL DEFAULT FALSE,
    reactome_release TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_pathway_per_release UNIQUE (reactome_id, reactome_release)
);

CREATE INDEX idx_pathway_reactome_id ON pathway_terms(reactome_id);
CREATE INDEX idx_pathway_species     ON pathway_terms(species_name);
CREATE INDEX idx_pathway_taxid       ON pathway_terms(species_taxid) WHERE species_taxid IS NOT NULL;
CREATE INDEX idx_pathway_top_level   ON pathway_terms(is_top_level) WHERE is_top_level;
CREATE INDEX idx_pathway_data_src    ON pathway_terms(data_source_id);
CREATE INDEX idx_pathway_name_fts    ON pathway_terms
    USING GIN (to_tsvector('english', name));

-- Pathway hierarchy (parent-child) — populated from UniProt2ReactomeAll.txt or inferred
-- Reactome doesn't provide explicit hierarchy file; top-level detection from species-specific file
-- (Pathway hierarchy will be populated in a future enhancement via Reactome's SBML export)

-- TYPED EDGE: protein participates_in pathway (Biolink: biolink:participates_in)
CREATE TABLE protein_pathway_associations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Protein side: UniProt accession (resolved to protein_metadata.id when proteins are loaded)
    uniprot_acc     TEXT NOT NULL,       -- 'P04637' (denormalized — join to protein_metadata at query time)
    -- Pathway side
    pathway_id      UUID NOT NULL REFERENCES pathway_terms(id) ON DELETE CASCADE,
    reactome_id     TEXT NOT NULL,       -- denormalized for fast lookup
    -- Association details
    evidence_type   TEXT,               -- 'IEA', 'inferred_from_experiment', etc.
    species_name    TEXT NOT NULL,
    source_db       TEXT NOT NULL DEFAULT 'reactome',
    reactome_release TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_protein_pathway UNIQUE (uniprot_acc, pathway_id, reactome_release)
);

CREATE INDEX idx_ppa_uniprot    ON protein_pathway_associations(uniprot_acc);
CREATE INDEX idx_ppa_pathway    ON protein_pathway_associations(pathway_id);
CREATE INDEX idx_ppa_species    ON protein_pathway_associations(species_name);
CREATE INDEX idx_ppa_reactome   ON protein_pathway_associations(reactome_id);
```

- [ ] **Step 2: Apply + commit**

```bash
cargo xtask db migrate 2>&1 | tail -5
git add migrations/20260326000001_reactome_tables.sql
git commit -m "feat(db): add Reactome pathway_terms and protein_pathway_associations tables"
```

---

## Task 2: Domain models

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/reactome/mod.rs`
- Create: `crates/bdp-ingest/src/pipelines/reactome/models.rs`

- [ ] **Step 1: Create files**

```rust
// mod.rs
pub mod models;
pub mod parser;
pub mod runner;
pub mod storage;
pub use runner::{ReactomeConfig, ReactomePipelineRunner};

pub const REACTOME_PATHWAYS_URL: &str =
    "https://reactome.org/download/current/ReactomePathways.txt";
pub const REACTOME_UNIPROT_URL: &str =
    "https://reactome.org/download/current/UniProt2Reactome.txt";
// Human-only mapping (smaller, faster for initial testing):
pub const REACTOME_UNIPROT_HUMAN_URL: &str =
    "https://reactome.org/download/current/UniProt2Reactome_All_Levels.txt";
```

```rust
// models.rs
#[derive(Debug, Clone)]
pub struct Pathway {
    pub reactome_id: String,    // 'R-HSA-9612973'
    pub name: String,
    pub species_name: String,   // 'Homo sapiens'
    pub reactome_release: String,
}

#[derive(Debug, Clone)]
pub struct ProteinPathwayLink {
    pub uniprot_acc: String,    // 'P04637'
    pub reactome_id: String,    // 'R-HSA-9612973'
    pub pathway_name: String,
    pub evidence_type: Option<String>,
    pub species_name: String,
    pub reactome_release: String,
}

#[derive(Debug, Default)]
pub struct ParsedReactome {
    pub pathways: Vec<Pathway>,
    pub protein_pathway_links: Vec<ProteinPathwayLink>,
}
```

- [ ] **Step 2: Compile check + commit**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -10
git add crates/bdp-ingest/src/pipelines/reactome/
git commit -m "feat(bdp-ingest): add Reactome domain models"
```

---

## Task 3: TSV parsers

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/reactome/parser.rs`

- [ ] **Step 1: Create parser.rs**

```rust
// crates/bdp-ingest/src/pipelines/reactome/parser.rs
//
// Reactome uses TSV files, not OBO — no shared parser.

use crate::pipelines::reactome::models::*;
use anyhow::Result;
use tracing::warn;

/// Parse ReactomePathways.txt
///
/// Format (tab-separated, no header):
///   reactome_id \t name \t species
pub fn parse_pathways(content: &str, release: &str) -> Result<Vec<Pathway>> {
    let mut pathways = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() { continue; }

        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 3 {
            warn!(line = line_num, "ReactomePathways: skipping malformed line");
            continue;
        }

        pathways.push(Pathway {
            reactome_id: cols[0].trim().to_string(),
            name: cols[1].trim().to_string(),
            species_name: cols[2].trim().to_string(),
            reactome_release: release.to_string(),
        });
    }

    Ok(pathways)
}

/// Parse UniProt2Reactome.txt (or UniProt2Reactome_All_Levels.txt)
///
/// Format (tab-separated, no header):
///   uniprot_acc \t reactome_id \t url \t pathway_name \t evidence_code \t species
///
/// Optionally filter to a specific species (e.g., "Homo sapiens").
pub fn parse_uniprot_reactome(
    content: &str,
    release: &str,
    species_filter: Option<&str>,
) -> Result<Vec<ProteinPathwayLink>> {
    let mut links = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }

        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 6 {
            warn!(line = line_num, "UniProt2Reactome: skipping malformed line");
            continue;
        }

        let species = cols[5].trim();
        if let Some(filter) = species_filter {
            if species != filter {
                continue;
            }
        }

        let uniprot_acc = cols[0].trim().to_string();
        // Skip isoform-specific entries (e.g., P04637-1)
        if uniprot_acc.contains('-') { continue; }

        links.push(ProteinPathwayLink {
            uniprot_acc,
            reactome_id: cols[1].trim().to_string(),
            pathway_name: cols[3].trim().to_string(),
            evidence_type: {
                let ev = cols[4].trim();
                if ev.is_empty() { None } else { Some(ev.to_string()) }
            },
            species_name: species.to_string(),
            reactome_release: release.to_string(),
        });
    }

    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pathways() {
        let content = "R-HSA-9612973\tActivation of AMPK downstream\tHomo sapiens\nR-MMU-9612973\tActivation of AMPK downstream\tMus musculus\n";
        let pathways = parse_pathways(content, "114").unwrap();
        assert_eq!(pathways.len(), 2);
        assert_eq!(pathways[0].reactome_id, "R-HSA-9612973");
        assert_eq!(pathways[0].species_name, "Homo sapiens");
    }

    #[test]
    fn test_parse_uniprot_reactome() {
        let content = "P04637\tR-HSA-9612973\thttps://reactome.org/...\tActivation of AMPK\tTAS\tHomo sapiens\nP12345\tR-MMU-9612973\thttps://reactome.org/...\tSome pathway\tTAS\tMus musculus\n";
        let links = parse_uniprot_reactome(content, "114", Some("Homo sapiens")).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].uniprot_acc, "P04637");
        assert_eq!(links[0].reactome_id, "R-HSA-9612973");
        assert_eq!(links[0].evidence_type.as_deref(), Some("TAS"));
    }

    #[test]
    fn test_skip_isoforms() {
        let content = "P04637-1\tR-HSA-9612973\thttp://\tpathway\tTAS\tHomo sapiens\n";
        let links = parse_uniprot_reactome(content, "114", None).unwrap();
        assert_eq!(links.len(), 0, "isoform entries should be skipped");
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p bdp-ingest pipelines::reactome::parser 2>&1 | tail -15
```
Expected: 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/bdp-ingest/src/pipelines/reactome/parser.rs
git commit -m "feat(bdp-ingest): add Reactome TSV parsers for pathways and protein mappings"
```

---

## Task 4: Storage layer

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/reactome/storage.rs`

- [ ] **Step 1: Create storage.rs**

```rust
// crates/bdp-ingest/src/pipelines/reactome/storage.rs

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use crate::common::batch::BatchConfig;
use crate::pipelines::reactome::models::*;

pub struct ReactomeStorage {
    pool: PgPool,
    batch: BatchConfig,
}

impl ReactomeStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool, batch: BatchConfig::default() }
    }

    pub async fn ingest_release(
        &self,
        org_id: Uuid,
        release: &str,
        pathways: &[Pathway],
        links: &[ProteinPathwayLink],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let data_source_id = self.upsert_registry(&mut tx, org_id).await?;
        self.upsert_version(&mut tx, data_source_id, release).await?;

        info!(count = pathways.len(), "storing Reactome pathways");
        // Store pathways and collect reactome_id → UUID map
        let pathway_id_map = self.store_pathways(&mut tx, data_source_id, pathways).await?;

        info!(count = links.len(), "storing protein→pathway associations");
        self.store_links(&mut tx, &pathway_id_map, links).await?;

        tx.commit().await?;
        info!(release, "Reactome ingest complete");
        Ok(())
    }

    async fn upsert_registry(&self, tx: &mut Transaction<'_, Postgres>, org_id: Uuid) -> Result<Uuid> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO registry_entries (organization_id, slug, name, entry_type)
             VALUES ($1, 'reactome', 'Reactome Pathway Database', 'data_source')
             ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name RETURNING id"
        ).bind(org_id).fetch_one(&mut **tx).await?;

        sqlx::query("INSERT INTO data_sources (id, source_type, external_id) VALUES ($1, 'pathway', 'reactome') ON CONFLICT (id) DO NOTHING")
            .bind(id).execute(&mut **tx).await?;
        Ok(id)
    }

    async fn upsert_version(&self, tx: &mut Transaction<'_, Postgres>, ds_id: Uuid, release: &str) -> Result<()> {
        sqlx::query("INSERT INTO versions (entry_id, version, release_date) VALUES ($1, $2, CURRENT_DATE) ON CONFLICT (entry_id, version) DO NOTHING")
            .bind(ds_id).bind(release).execute(&mut **tx).await?;
        Ok(())
    }

    /// Store pathways and return a map of reactome_id → UUID.
    async fn store_pathways(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ds_id: Uuid,
        pathways: &[Pathway],
    ) -> Result<HashMap<String, Uuid>> {
        let mut id_map = HashMap::with_capacity(pathways.len());

        for chunk in pathways.chunks(self.batch.chunk_size) {
            for p in chunk {
                let id: Uuid = sqlx::query_scalar(
                    "INSERT INTO pathway_terms (data_source_id, reactome_id, name, species_name, reactome_release)
                     VALUES ($1, $2, $3, $4, $5)
                     ON CONFLICT (reactome_id, reactome_release)
                     DO UPDATE SET name = EXCLUDED.name, species_name = EXCLUDED.species_name
                     RETURNING id"
                )
                .bind(ds_id).bind(&p.reactome_id).bind(&p.name).bind(&p.species_name).bind(&p.reactome_release)
                .fetch_one(&mut **tx).await
                .context("insert pathway term")?;

                id_map.insert(p.reactome_id.clone(), id);
            }
        }

        Ok(id_map)
    }

    async fn store_links(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        pathway_id_map: &HashMap<String, Uuid>,
        links: &[ProteinPathwayLink],
    ) -> Result<()> {
        let mut skipped = 0usize;

        for chunk in links.chunks(self.batch.chunk_size) {
            for link in chunk {
                let pathway_uuid = match pathway_id_map.get(&link.reactome_id) {
                    Some(id) => id,
                    None => {
                        // Pathway not in pathway_terms (e.g., cross-species link)
                        skipped += 1;
                        continue;
                    }
                };

                sqlx::query(
                    "INSERT INTO protein_pathway_associations
                     (uniprot_acc, pathway_id, reactome_id, evidence_type, species_name, reactome_release)
                     VALUES ($1, $2, $3, $4, $5, $6)
                     ON CONFLICT (uniprot_acc, pathway_id, reactome_release) DO NOTHING"
                )
                .bind(&link.uniprot_acc)
                .bind(pathway_uuid)
                .bind(&link.reactome_id)
                .bind(&link.evidence_type)
                .bind(&link.species_name)
                .bind(&link.reactome_release)
                .execute(&mut **tx).await
                .context("insert protein_pathway_association")?;
            }
        }

        if skipped > 0 {
            info!(skipped, "skipped links for unknown pathways");
        }
        Ok(())
    }
}
```

- [ ] **Step 2: Compile check + commit**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -10
git add crates/bdp-ingest/src/pipelines/reactome/storage.rs
git commit -m "feat(bdp-ingest): add Reactome storage with protein_pathway_associations"
```

---

## Task 5: Runner + registration

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/reactome/runner.rs`
- Modify: `crates/bdp-ingest/src/pipelines/mod.rs`

- [ ] **Step 1: Create runner.rs**

```rust
// crates/bdp-ingest/src/pipelines/reactome/runner.rs

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::common::http::download_text;
use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::reactome::{parser, storage::ReactomeStorage, REACTOME_PATHWAYS_URL, REACTOME_UNIPROT_URL};

#[derive(Debug, Clone)]
pub struct ReactomeConfig {
    pub pathways_url: String,
    pub uniprot_url: String,
    pub max_retries: u32,
    pub release: String,
    pub org_id: Uuid,
    /// Filter to specific species (e.g. "Homo sapiens"). None = all species.
    pub species_filter: Option<String>,
}

impl ReactomeConfig {
    pub fn human_only(release: impl Into<String>, org_id: Uuid) -> Self {
        Self {
            pathways_url: REACTOME_PATHWAYS_URL.to_string(),
            uniprot_url: REACTOME_UNIPROT_URL.to_string(),
            max_retries: 3,
            release: release.into(),
            org_id,
            species_filter: Some("Homo sapiens".to_string()),
        }
    }

    pub fn all_species(release: impl Into<String>, org_id: Uuid) -> Self {
        Self {
            species_filter: None,
            ..Self::human_only(release, org_id)
        }
    }
}

pub struct ReactomePipelineRunner {
    config: ReactomeConfig,
    pool: PgPool,
}

impl ReactomePipelineRunner {
    pub fn new(config: ReactomeConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }
}

impl PipelineRunner for ReactomePipelineRunner {
    fn name(&self) -> &'static str { "reactome" }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());

        // 1. Pathways
        info!("downloading ReactomePathways.txt");
        let pathways_content = download_text(&self.config.pathways_url, self.config.max_retries).await?;
        let pathways = parser::parse_pathways(&pathways_content, &self.config.release)?;

        // 2. UniProt→Reactome mappings
        info!("downloading UniProt2Reactome.txt");
        let uniprot_content = download_text(&self.config.uniprot_url, self.config.max_retries).await?;
        let links = parser::parse_uniprot_reactome(
            &uniprot_content,
            &self.config.release,
            self.config.species_filter.as_deref(),
        )?;

        stats.records_ingested = (pathways.len() + links.len()) as u64;

        info!(pathways = pathways.len(), links = links.len(), "Reactome parsed");

        let storage = ReactomeStorage::new(self.pool);
        storage.ingest_release(self.config.org_id, &self.config.release, &pathways, &links).await?;

        Ok(stats)
    }
}
```

- [ ] **Step 2: Add to pipelines/mod.rs**

```rust
pub mod reactome;
```

- [ ] **Step 3: Add integration test**

In `crates/bdp-ingest/tests/obo_integration.rs` (rename to `pipeline_integration.rs` or add there):

```rust
#[tokio::test]
#[ignore = "downloads from reactome.org"]
async fn test_parse_reactome_pathways() {
    use bdp_ingest::pipelines::reactome::parser;

    let url = "https://reactome.org/download/current/ReactomePathways.txt";
    let content = bdp_ingest::common::http::download_text(url, 3).await.unwrap();
    let pathways = parser::parse_pathways(&content, "114").unwrap();

    assert!(pathways.len() > 20_000, "expected >20K pathways, got {}", pathways.len());

    let human = pathways.iter().filter(|p| p.species_name == "Homo sapiens").count();
    assert!(human > 2_000, "expected >2K human pathways, got {}", human);

    println!("Reactome: {} total pathways, {} human", pathways.len(), human);
}

#[tokio::test]
#[ignore = "downloads ~100MB from reactome.org"]
async fn test_parse_reactome_uniprot_human() {
    use bdp_ingest::pipelines::reactome::parser;

    let url = "https://reactome.org/download/current/UniProt2Reactome.txt";
    let content = bdp_ingest::common::http::download_text(url, 3).await.unwrap();
    let links = parser::parse_uniprot_reactome(&content, "114", Some("Homo sapiens")).unwrap();

    assert!(links.len() > 100_000, "expected >100K human links, got {}", links.len());

    // TP53 (P04637) should map to many pathways
    let tp53_links: Vec<_> = links.iter().filter(|l| l.uniprot_acc == "P04637").collect();
    assert!(!tp53_links.is_empty(), "P04637 (TP53) should map to pathways");

    println!("Reactome: {} human protein→pathway links, {} TP53 pathways", links.len(), tp53_links.len());
}
```

Run:
```bash
cargo test -p bdp-ingest --test obo_integration test_parse_reactome -- --ignored --nocapture 2>&1 | tail -15
```

- [ ] **Step 4: All unit tests**

```bash
cargo test -p bdp-ingest --lib 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add crates/bdp-ingest/src/pipelines/reactome/runner.rs \
        crates/bdp-ingest/src/pipelines/mod.rs \
        crates/bdp-ingest/tests/obo_integration.rs
git commit -m "feat(bdp-ingest): complete Reactome pipeline — pathways + protein_pathway_associations"
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
git log --oneline -12
```
