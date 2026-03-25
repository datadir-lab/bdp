# bdp-ingest Crate Setup + OBO Parser Extraction Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Set up `bdp-ingest` as the home for all new pipelines — add `PipelineRunner` trait, extract a generic OBO parser from the GO-specific parser, and create the common infrastructure (HTTP client, batch helpers) that Reactome/MONDO/HPO/ChEBI pipelines will use.

**Architecture:** New pipelines (Reactome, MONDO, HPO, ChEBI) are implemented from scratch in `bdp-ingest/src/pipelines/`. Existing pipelines (UniProt, NCBI, GenBank, GO, InterPro) remain in `bdp-server/src/ingest/` for now — they will be migrated in a follow-up. `bdp-server` imports `bdp-ingest` as a workspace dependency to run new pipelines from the same orchestrator. The generic `RawOboTerm`-based OBO parser lives in `bdp-ingest/src/common/obo.rs` and is shared by all four OBO-based pipelines.

**Tech Stack:** Rust, tokio, reqwest, sqlx (feature-gated), anyhow, cargo workspace

---

## File Map

**New files in `crates/bdp-ingest/src/`:**
- `common/obo.rs` — generic `RawOboTerm`, `OboParser` (extracted from gene_ontology/parser.rs)
- `common/http.rs` — shared HTTP client helpers (download with retry, content-type detection)
- `common/batch.rs` — `BatchConfig`, chunked insert helper
- `common/mod.rs` — re-exports (update existing)
- `framework/pipeline.rs` — `PipelineRunner` trait + `PipelineStats`
- `framework/mod.rs` — new module
- `pipelines/mod.rs` — pipeline registry
- `lib.rs` — updated to expose new modules

**Modified files:**
- `crates/bdp-ingest/Cargo.toml` — add `sqlx` to workspace dep, enable `database` feature by default
- `crates/bdp-server/Cargo.toml` — add `bdp-ingest` as workspace dependency
- `Cargo.toml` (workspace root) — ensure bdp-ingest is in workspace members

---

## Task 1: Add framework/pipeline.rs — PipelineRunner trait

**Files:**
- Create: `crates/bdp-ingest/src/framework/mod.rs`
- Create: `crates/bdp-ingest/src/framework/pipeline.rs`
- Modify: `crates/bdp-ingest/src/lib.rs`

- [ ] **Step 1: Create framework/mod.rs**

```rust
// crates/bdp-ingest/src/framework/mod.rs
pub mod pipeline;
pub use pipeline::{PipelineRunner, PipelineStats};
```

- [ ] **Step 2: Create framework/pipeline.rs**

```rust
// crates/bdp-ingest/src/framework/pipeline.rs

use std::future::Future;

/// Statistics returned by a completed pipeline run.
#[derive(Debug, Default, Clone)]
pub struct PipelineStats {
    pub pipeline_name: &'static str,
    pub records_ingested: u64,
    pub records_skipped: u64,
    pub records_failed: u64,
    pub duration_secs: u64,
}

impl PipelineStats {
    pub fn new(name: &'static str) -> Self {
        Self {
            pipeline_name: name,
            ..Default::default()
        }
    }
}

impl std::fmt::Display for PipelineStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: ingested={} skipped={} failed={} duration={}s",
            self.pipeline_name,
            self.records_ingested,
            self.records_skipped,
            self.records_failed,
            self.duration_secs,
        )
    }
}

/// Implemented by every ingest pipeline.
///
/// # Requirements
/// - `Send + 'static` so the pipeline can be spawned via `JoinSet::spawn`
/// - `run()` consumes `self` — pipelines are single-use
pub trait PipelineRunner: Send + 'static {
    fn name(&self) -> &'static str;

    fn run(self) -> impl Future<Output = anyhow::Result<PipelineStats>> + Send;
}
```

- [ ] **Step 3: Update lib.rs to expose framework module**

In `crates/bdp-ingest/src/lib.rs`, add:
```rust
pub mod framework;
```

- [ ] **Step 4: Compile check**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -20
```
Expected: zero `error:` lines.

- [ ] **Step 5: Commit**

```bash
git add crates/bdp-ingest/src/framework/ crates/bdp-ingest/src/lib.rs
git commit -m "feat(bdp-ingest): add PipelineRunner trait and PipelineStats"
```

---

## Task 2: common/batch.rs — BatchConfig + chunked insert helper

**Files:**
- Create: `crates/bdp-ingest/src/common/batch.rs`
- Modify: `crates/bdp-ingest/src/common/mod.rs`

- [ ] **Step 1: Create common/batch.rs**

```rust
// crates/bdp-ingest/src/common/batch.rs

/// Configuration for chunked batch inserts.
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum number of rows per INSERT statement.
    pub chunk_size: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self { chunk_size: 500 }
    }
}

impl BatchConfig {
    pub fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }
}

/// Split a slice into chunks of `chunk_size` for batch processing.
pub fn chunks<T>(items: &[T], chunk_size: usize) -> impl Iterator<Item = &[T]> {
    items.chunks(chunk_size)
}
```

- [ ] **Step 2: Update common/mod.rs**

Add `pub mod batch;` and `pub use batch::{BatchConfig, chunks};` to the existing `common/mod.rs`.

Check the current contents first:
```bash
cat crates/bdp-ingest/src/common/mod.rs
```
Then append the batch module declaration.

- [ ] **Step 3: Compile check + commit**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -20
git add crates/bdp-ingest/src/common/batch.rs crates/bdp-ingest/src/common/mod.rs
git commit -m "feat(bdp-ingest): add BatchConfig and chunked insert helper"
```

---

## Task 3: common/obo.rs — generic OBO parser extracted from gene_ontology

This is the core extraction. The existing `OboParser` in
`crates/bdp-server/src/ingest/gene_ontology/parser.rs` is tightly coupled to GO types.
We create a generic version here that returns `RawOboTerm` structs — the GO, MONDO, HPO,
and ChEBI pipelines each provide a thin adapter.

**Files:**
- Create: `crates/bdp-ingest/src/common/obo.rs`
- Modify: `crates/bdp-ingest/src/common/mod.rs`

- [ ] **Step 1: Read the existing parser to understand what OBO fields are used**

```bash
cat crates/bdp-server/src/ingest/gene_ontology/parser.rs
```
Note: the existing parser has OBO field handling for `id`, `name`, `namespace`, `def`, `synonym`,
`xref`, `alt_id`, `is_a`, `relationship`, `is_obsolete`, `comment`. We mirror all of these.

- [ ] **Step 2: Create common/obo.rs**

```rust
// crates/bdp-ingest/src/common/obo.rs
//
// Generic OBO 1.4 format parser.
// Returns RawOboTerm structs — each pipeline maps these to its domain types.
//
// Reference: https://owlcollab.github.io/oboformat/doc/GO.format.obo-1_4.html

use tracing::{debug, info, warn};

/// A synonym entry from an OBO term.
#[derive(Debug, Clone)]
pub struct RawOboSynonym {
    /// EXACT, BROAD, NARROW, or RELATED
    pub scope: String,
    pub text: String,
    /// Optional synonym type name (e.g., "systematic_synonym")
    pub synonym_type: Option<String>,
}

/// A typed relationship from an OBO term (not is_a).
#[derive(Debug, Clone)]
pub struct RawOboRelationship {
    /// Relation type: "part_of", "regulates", "positively_regulates", etc.
    pub rel_type: String,
    /// Target term ID: "GO:0006955", "MONDO:0004992"
    pub target: String,
}

/// A raw, parsed OBO term with no domain-specific interpretation.
/// All fields are strings/vecs — the pipeline adapter does the semantic mapping.
#[derive(Debug, Clone, Default)]
pub struct RawOboTerm {
    /// Primary identifier: "GO:0008150", "MONDO:0004992", "HP:0000001"
    pub id: String,
    pub name: String,
    /// Raw namespace string: "biological_process", "disease", "HP"
    pub namespace: Option<String>,
    /// Term definition (the `def:` field, quotes stripped)
    pub definition: Option<String>,
    pub is_obsolete: bool,
    pub synonyms: Vec<RawOboSynonym>,
    /// Raw xref strings: "OMIM:123456", "Wikipedia:Immune_response"
    pub xrefs: Vec<String>,
    /// Alternative IDs for the same concept
    pub alt_ids: Vec<String>,
    pub comment: Option<String>,
    /// Parent term IDs from `is_a:` lines
    pub is_a: Vec<String>,
    /// Other typed relationships
    pub relationships: Vec<RawOboRelationship>,
    /// Property-value pairs: ("inchikey", "AAAA..."), ("smiles", "C(=O)...")
    pub property_values: Vec<(String, String)>,
}

#[derive(Debug, thiserror::Error)]
pub enum OboParseError {
    #[error("OBO parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
}

/// Generic OBO 1.4 parser.
/// Parses `[Term]` stanzas only (ignores `[Typedef]`).
pub struct OboParser;

impl OboParser {
    /// Parse an OBO format string into a list of raw terms.
    ///
    /// # Arguments
    /// * `content` - Full text of the .obo file
    /// * `limit` - Optional maximum number of terms to parse (for testing)
    pub fn parse(content: &str, limit: Option<usize>) -> Result<Vec<RawOboTerm>, OboParseError> {
        let mut terms = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        info!("OBO parse start: {} lines", lines.len());

        // Skip header
        while i < lines.len() && lines[i].trim() != "[Term]" {
            i += 1;
        }

        while i < lines.len() {
            if let Some(max) = limit {
                if terms.len() >= max {
                    info!("OBO parse limit reached: {} terms", max);
                    break;
                }
            }

            let line = lines[i].trim();

            if line == "[Term]" {
                i += 1;
                let (term, next_i) = Self::parse_stanza(&lines, i)?;
                if !term.id.is_empty() {
                    terms.push(term);
                }
                i = next_i;
            } else if line == "[Typedef]" {
                // Skip typedef stanzas
                i += 1;
                while i < lines.len() {
                    let l = lines[i].trim();
                    if l == "[Term]" || l == "[Typedef]" {
                        break;
                    }
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        info!("OBO parse complete: {} terms", terms.len());
        Ok(terms)
    }

    fn parse_stanza(
        lines: &[&str],
        start: usize,
    ) -> Result<(RawOboTerm, usize), OboParseError> {
        let mut term = RawOboTerm::default();
        let mut i = start;

        while i < lines.len() {
            let line = lines[i].trim();

            if line.is_empty() || line == "[Term]" || line == "[Typedef]" {
                break;
            }

            if let Some(rest) = line.strip_prefix("id: ") {
                term.id = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("name: ") {
                term.name = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("namespace: ") {
                term.namespace = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("def: ") {
                // def: "definition text" [xref1, xref2]
                // Strip leading/trailing quote
                let def = rest.trim().trim_start_matches('"');
                let def = if let Some(end) = def.find("\" [") {
                    &def[..end]
                } else if let Some(end) = def.rfind('"') {
                    &def[..end]
                } else {
                    def
                };
                term.definition = Some(def.to_string());
            } else if let Some(rest) = line.strip_prefix("is_obsolete: ") {
                term.is_obsolete = rest.trim() == "true";
            } else if let Some(rest) = line.strip_prefix("comment: ") {
                term.comment = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("synonym: ") {
                if let Some(syn) = Self::parse_synonym(rest.trim()) {
                    term.synonyms.push(syn);
                }
            } else if let Some(rest) = line.strip_prefix("xref: ") {
                // Take only the xref ID part, ignore trailing description
                let xref = rest.trim().split_whitespace().next().unwrap_or("").to_string();
                if !xref.is_empty() {
                    term.xrefs.push(xref);
                }
            } else if let Some(rest) = line.strip_prefix("alt_id: ") {
                term.alt_ids.push(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("is_a: ") {
                // is_a: GO:0006950 ! response to stress
                let parent_id = rest.trim().split_whitespace().next().unwrap_or("").to_string();
                if !parent_id.is_empty() {
                    term.is_a.push(parent_id);
                }
            } else if let Some(rest) = line.strip_prefix("relationship: ") {
                // relationship: part_of GO:0006950 ! response to stress
                let mut parts = rest.trim().splitn(3, ' ');
                if let (Some(rel_type), Some(target)) = (parts.next(), parts.next()) {
                    term.relationships.push(RawOboRelationship {
                        rel_type: rel_type.to_string(),
                        target: target.split('!').next().unwrap_or(target).trim().to_string(),
                    });
                }
            } else if let Some(rest) = line.strip_prefix("property_value: ") {
                // property_value: inchikey "UHOVQNZJYSORNB-UHFFFAOYSA-N" xsd:string
                let mut parts = rest.trim().splitn(3, ' ');
                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                    let value = value.trim_matches('"').to_string();
                    term.property_values.push((key.to_string(), value));
                }
            } else {
                debug!("OBO: unhandled field at line {}: {}", i, line);
            }

            i += 1;
        }

        Ok((term, i))
    }

    fn parse_synonym(text: &str) -> Option<RawOboSynonym> {
        // Format: "synonym text" EXACT [xrefs]
        // or: "synonym text" EXACT synonym_type_name [xrefs]
        let text = text.trim_start_matches('"');
        let end_quote = text.find('"')?;
        let synonym_text = text[..end_quote].to_string();
        let rest = text[end_quote + 1..].trim();

        let mut parts = rest.split_whitespace();
        let scope = parts.next()?.to_string();

        // Optional synonym type name (before the '[' of xrefs)
        let synonym_type = {
            let next = parts.next()?;
            if next.starts_with('[') {
                None
            } else {
                Some(next.to_string())
            }
        };

        Some(RawOboSynonym {
            scope,
            text: synonym_text,
            synonym_type,
        })
    }

    /// Split "DB:ID" xref strings into (db, id) pairs.
    /// "OMIM:604606" → ("OMIM", "604606")
    /// "Wikipedia:Immune_response" → ("Wikipedia", "Immune_response")
    pub fn split_xref(xref: &str) -> (String, String) {
        if let Some(colon) = xref.find(':') {
            (xref[..colon].to_string(), xref[colon + 1..].to_string())
        } else {
            ("unknown".to_string(), xref.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_OBO: &str = r#"
format-version: 1.2
ontology: test

[Term]
id: GO:0006955
name: immune response
namespace: biological_process
def: "A defense reaction by which organisms protect against infection." [GOC:mah]
synonym: "immune reactions" EXACT []
synonym: "immunity" BROAD []
xref: Wikipedia:Immune_response
xref: KEGG:ko04620
is_a: GO:0006950 ! response to stress
relationship: part_of GO:0002376 ! immune system process
alt_id: GO:0001234

[Term]
id: GO:0000000
name: obsolete term
namespace: biological_process
is_obsolete: true
"#;

    #[test]
    fn test_parse_basic_term() {
        let terms = OboParser::parse(SAMPLE_OBO, None).unwrap();
        assert_eq!(terms.len(), 2);

        let t = &terms[0];
        assert_eq!(t.id, "GO:0006955");
        assert_eq!(t.name, "immune response");
        assert_eq!(t.namespace.as_deref(), Some("biological_process"));
        assert!(!t.is_obsolete);
        assert_eq!(t.synonyms.len(), 2);
        assert_eq!(t.synonyms[0].scope, "EXACT");
        assert_eq!(t.synonyms[0].text, "immune reactions");
        assert_eq!(t.synonyms[1].scope, "BROAD");
        assert_eq!(t.xrefs.len(), 2);
        assert!(t.xrefs.contains(&"Wikipedia:Immune_response".to_string()));
        assert_eq!(t.is_a.len(), 1);
        assert_eq!(t.is_a[0], "GO:0006955");  // note: check actual value
        assert_eq!(t.alt_ids.len(), 1);
        assert_eq!(t.alt_ids[0], "GO:0001234");
        assert_eq!(t.relationships.len(), 1);
        assert_eq!(t.relationships[0].rel_type, "part_of");
    }

    #[test]
    fn test_parse_obsolete_term() {
        let terms = OboParser::parse(SAMPLE_OBO, None).unwrap();
        assert!(terms[1].is_obsolete);
    }

    #[test]
    fn test_parse_limit() {
        let terms = OboParser::parse(SAMPLE_OBO, Some(1)).unwrap();
        assert_eq!(terms.len(), 1);
    }

    #[test]
    fn test_split_xref() {
        assert_eq!(
            OboParser::split_xref("OMIM:604606"),
            ("OMIM".to_string(), "604606".to_string())
        );
        assert_eq!(
            OboParser::split_xref("Wikipedia:Immune_response"),
            ("Wikipedia".to_string(), "Immune_response".to_string())
        );
        assert_eq!(
            OboParser::split_xref("nocolon"),
            ("unknown".to_string(), "nocolon".to_string())
        );
    }

    #[test]
    fn test_parse_synonym_types() {
        let syn = OboParser::parse_synonym(r#""exact name" EXACT []"#);
        assert!(syn.is_some());
        let syn = syn.unwrap();
        assert_eq!(syn.scope, "EXACT");
        assert_eq!(syn.text, "exact name");
        assert!(syn.synonym_type.is_none());
    }
}
```

- [ ] **Step 3: Update common/mod.rs to expose obo module**

Add `pub mod obo;` and `pub use obo::{OboParser, RawOboTerm, RawOboSynonym, RawOboRelationship, OboParseError};`

- [ ] **Step 4: Run tests**

```bash
cargo test -p bdp-ingest common::obo 2>&1 | tail -30
```
Expected: all tests pass.

Fix the test assertion in step 2 that says `assert_eq!(t.is_a[0], "GO:0006955")` — it should be `GO:0006950`. Update before committing.

- [ ] **Step 5: Commit**

```bash
git add crates/bdp-ingest/src/common/obo.rs crates/bdp-ingest/src/common/mod.rs
git commit -m "feat(bdp-ingest): add generic OBO 1.4 parser with RawOboTerm"
```

---

## Task 4: common/http.rs — shared download helper

**Files:**
- Create: `crates/bdp-ingest/src/common/http.rs`
- Modify: `crates/bdp-ingest/src/common/mod.rs`

- [ ] **Step 1: Create common/http.rs**

```rust
// crates/bdp-ingest/src/common/http.rs

use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, info};

/// Download a URL to a String, with retry.
///
/// Tries up to `max_retries` times with exponential backoff.
pub async fn download_text(url: &str, max_retries: u32) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(300))
        .user_agent("bdp-ingest/0.1 (https://github.com/datadir-lab/bdp)")
        .build()
        .context("failed to build HTTP client")?;

    let mut last_error = None;
    for attempt in 0..=max_retries {
        if attempt > 0 {
            let backoff = Duration::from_secs(2u64.pow(attempt));
            info!("Retry {}/{} for {} (backoff: {:?})", attempt, max_retries, url, backoff);
            tokio::time::sleep(backoff).await;
        }

        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    let text = resp.text().await.context("failed to read response body")?;
                    debug!("Downloaded {} bytes from {}", text.len(), url);
                    return Ok(text);
                }
                last_error = Some(anyhow::anyhow!("HTTP {}: {}", status, url));
            }
            Err(e) => {
                last_error = Some(anyhow::anyhow!("request failed: {}: {}", url, e));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("download failed: {}", url)))
}

/// Download a URL to bytes, with retry.
pub async fn download_bytes(url: &str, max_retries: u32) -> Result<bytes::Bytes> {
    let client = Client::builder()
        .timeout(Duration::from_secs(600))
        .user_agent("bdp-ingest/0.1 (https://github.com/datadir-lab/bdp)")
        .build()
        .context("failed to build HTTP client")?;

    let mut last_error = None;
    for attempt in 0..=max_retries {
        if attempt > 0 {
            let backoff = Duration::from_secs(2u64.pow(attempt));
            tokio::time::sleep(backoff).await;
        }

        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                return resp.bytes().await.context("failed to read response body");
            }
            Ok(resp) => {
                last_error = Some(anyhow::anyhow!("HTTP {}: {}", resp.status(), url));
            }
            Err(e) => {
                last_error = Some(anyhow::anyhow!("{}", e));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("download failed: {}", url)))
}
```

- [ ] **Step 2: Add `bytes` to bdp-ingest Cargo.toml dependencies**

```toml
bytes = "1"
```

- [ ] **Step 3: Update common/mod.rs**

Add `pub mod http;` and re-export `download_text`, `download_bytes`.

- [ ] **Step 4: Compile check + commit**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -20
git add crates/bdp-ingest/src/common/http.rs crates/bdp-ingest/src/common/mod.rs \
        crates/bdp-ingest/Cargo.toml
git commit -m "feat(bdp-ingest): add HTTP download helper with retry"
```

---

## Task 5: pipelines/mod.rs — pipeline registry scaffold

**Files:**
- Create: `crates/bdp-ingest/src/pipelines/mod.rs`
- Modify: `crates/bdp-ingest/src/lib.rs`

- [ ] **Step 1: Create pipelines/mod.rs**

```rust
// crates/bdp-ingest/src/pipelines/mod.rs
//
// Pipeline registry — add new pipelines here.
// Each submodule implements PipelineRunner.
//
// Current pipelines:
//   (to be added as they are implemented)
//
// Existing pipelines (in bdp-server, pending migration):
//   uniprot, ncbi_taxonomy, genbank, gene_ontology, interpro

// Uncomment as pipelines are added:
// pub mod reactome;
// pub mod mondo;
// pub mod hpo;
// pub mod chebi;
```

- [ ] **Step 2: Update lib.rs**

```rust
// crates/bdp-ingest/src/lib.rs
pub mod common;
pub mod framework;
pub mod pipelines;

// Existing modules (stubs):
pub mod ensembl;
pub mod ncbi;
pub mod uniprot;
pub mod version_mapping;
```

- [ ] **Step 3: Compile check + commit**

```bash
SQLX_OFFLINE=true cargo check -p bdp-ingest 2>&1 | grep "^error" | head -20
git add crates/bdp-ingest/src/pipelines/ crates/bdp-ingest/src/lib.rs
git commit -m "feat(bdp-ingest): add pipeline registry scaffold"
```

---

## Task 6: Enable database feature in bdp-ingest + import from bdp-server

**Files:**
- Modify: `crates/bdp-ingest/Cargo.toml` — enable `database` feature, add sqlx pool types
- Modify: `crates/bdp-server/Cargo.toml` — add bdp-ingest as workspace dep

- [ ] **Step 1: Update bdp-ingest/Cargo.toml**

Change:
```toml
[features]
default = []
database = ["sqlx"]
```
To:
```toml
[features]
default = ["database"]
database = ["sqlx"]
```

Also ensure sqlx has the right features in workspace. Check `Cargo.toml` at root:
```bash
grep -A5 "sqlx" Cargo.toml
```

- [ ] **Step 2: Add bdp-ingest to bdp-server Cargo.toml**

In `crates/bdp-server/Cargo.toml`, add:
```toml
bdp-ingest = { workspace = true }
```

In workspace root `Cargo.toml`, add:
```toml
[workspace.dependencies]
bdp-ingest = { path = "crates/bdp-ingest" }
```
(If not already there.)

- [ ] **Step 3: Compile check**

```bash
SQLX_OFFLINE=true cargo check --workspace 2>&1 | grep "^error" | head -30
```

- [ ] **Step 4: Commit**

```bash
git add crates/bdp-ingest/Cargo.toml crates/bdp-server/Cargo.toml Cargo.toml
git commit -m "feat: enable bdp-ingest database feature, add as bdp-server dep"
```

---

## Task 7: End-to-end test — OBO parser roundtrip with real GO data

This test downloads the real GO OBO file and verifies the parser handles it correctly.
It's an integration test (downloads real data) — mark with `#[ignore]` so CI doesn't run it
on every push, but it can be run manually.

**Files:**
- Create: `crates/bdp-ingest/tests/obo_integration.rs`

- [ ] **Step 1: Write integration test**

```rust
// crates/bdp-ingest/tests/obo_integration.rs

use bdp_ingest::common::obo::OboParser;

/// Download and parse the real GO OBO file.
/// Run with: cargo test -p bdp-ingest --test obo_integration -- --ignored --nocapture
#[tokio::test]
#[ignore = "downloads ~50MB from internet"]
async fn test_parse_real_go_obo() {
    let url = "https://purl.obolibrary.org/obo/go/go-basic.obo";
    let content = bdp_ingest::common::http::download_text(url, 3)
        .await
        .expect("failed to download GO OBO");

    let terms = OboParser::parse(&content, None)
        .expect("failed to parse GO OBO");

    // GO has ~45,000 terms
    assert!(terms.len() > 40_000, "expected >40k terms, got {}", terms.len());

    // Spot check a known stable term
    let bp_root = terms.iter().find(|t| t.id == "GO:0008150");
    assert!(bp_root.is_some(), "biological_process root term not found");
    let bp = bp_root.unwrap();
    assert_eq!(bp.name, "biological_process");
    assert_eq!(bp.namespace.as_deref(), Some("biological_process"));

    // Count non-obsolete terms
    let active = terms.iter().filter(|t| !t.is_obsolete).count();
    assert!(active > 38_000, "expected >38k active terms, got {}", active);

    println!("Parsed {} total terms, {} active", terms.len(), active);
}

/// Parse a MONDO OBO slice (uses same format as GO)
#[tokio::test]
#[ignore = "downloads from internet"]
async fn test_parse_real_mondo_obo() {
    let url = "https://purl.obolibrary.org/obo/mondo.obo";
    let content = bdp_ingest::common::http::download_text(url, 3)
        .await
        .expect("failed to download MONDO OBO");

    let terms = OboParser::parse(&content, Some(1000))
        .expect("failed to parse MONDO OBO");

    // Just check we can parse 1000 MONDO terms without errors
    assert_eq!(terms.len(), 1000);
    // MONDO IDs start with MONDO:
    let mondo_ids: Vec<_> = terms.iter().filter(|t| t.id.starts_with("MONDO:")).collect();
    assert!(!mondo_ids.is_empty(), "no MONDO: prefixed IDs found");
}
```

- [ ] **Step 2: Run unit tests (fast, no network)**

```bash
cargo test -p bdp-ingest common::obo 2>&1 | tail -20
```
Expected: all pass.

- [ ] **Step 3: Run integration test (requires network)**

```bash
cargo test -p bdp-ingest --test obo_integration -- --ignored --nocapture 2>&1 | tail -30
```
Expected:
- Both tests pass
- Output shows term counts: ~45K for GO, 1000 for MONDO

- [ ] **Step 4: Commit**

```bash
git add crates/bdp-ingest/tests/obo_integration.rs
git commit -m "test(bdp-ingest): add OBO integration tests against real GO and MONDO data"
```

---

## Task 8: Final check — full workspace

- [ ] **Step 1: Full workspace compile**

```bash
SQLX_OFFLINE=true cargo build --workspace 2>&1 | grep "^error" | head -20
```
Expected: zero errors.

- [ ] **Step 2: All unit tests pass**

```bash
cargo test --workspace --lib 2>&1 | tail -20
```

- [ ] **Step 3: Commit summary**

```bash
git log --oneline -10
```

---

**Plan complete and saved to `docs/superpowers/plans/2026-03-25-bdp-ingest-obo-setup.md`.**
