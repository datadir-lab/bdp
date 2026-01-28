# BDP Ingestion Module Structure

**Date**: 2026-01-18
**Status**: 🚧 Implementation Guide

---

## Overview

Each data source organization has its own ingestion module following a consistent pattern. This ensures maintainability and extensibility as we add more organizations.

---

## Module Structure

```
crates/bdp-server/src/ingest/
├── framework/              # Shared framework (existing)
│   ├── mod.rs
│   ├── coordinator.rs
│   ├── worker.rs
│   ├── types.rs
│   └── ...
├── uniprot/               # UniProt ingestion
│   ├── mod.rs
│   ├── config.rs          # FTP config, versioning rules
│   ├── ftp.rs             # FTP client
│   ├── parser.rs          # DAT parser
│   ├── models.rs          # UniProtEntry, etc.
│   ├── versioning.rs      # Version bump detection
│   ├── changelog.rs       # Auto-generate changelog
│   ├── pipeline.rs        # Main ingestion pipeline
│   └── tests.rs           # Integration tests
├── ncbi/                  # NCBI ingestion (RefSeq, Taxonomy)
│   ├── mod.rs
│   ├── taxonomy/
│   │   ├── mod.rs
│   │   ├── parser.rs      # Taxonomy XML parser
│   │   ├── models.rs
│   │   └── pipeline.rs
│   ├── refseq/
│   │   ├── mod.rs
│   │   ├── parser.rs      # GenBank parser
│   │   ├── models.rs
│   │   └── pipeline.rs
│   └── config.rs
├── ensembl/               # Ensembl ingestion
│   ├── mod.rs
│   ├── rest_client.rs     # REST API client
│   ├── parser.rs          # JSON parser
│   ├── models.rs
│   ├── versioning.rs
│   └── pipeline.rs
├── pdb/                   # Protein Data Bank
│   ├── mod.rs
│   ├── rest_client.rs
│   ├── parser.rs
│   ├── models.rs
│   └── pipeline.rs
└── kegg/                  # KEGG Pathways
    ├── mod.rs
    ├── rest_client.rs
    ├── parser.rs
    ├── models.rs
    └── pipeline.rs
```

---

## Organization Module Template

### 1. Module Interface (`mod.rs`)

```rust
//! {Organization} data ingestion
//!
//! Handles ingestion of {data type} from {Organization}.
//!
//! # Versioning
//! {Versioning strategy summary}
//!
//! # License
//! {Data license info}

mod config;
mod parser;
mod models;
mod versioning;
mod changelog;
mod pipeline;

#[cfg(test)]
mod tests;

pub use config::*;
pub use parser::*;
pub use models::*;
pub use versioning::*;
pub use changelog::*;
pub use pipeline::*;

/// Organization ID (lazy static)
pub fn organization_id() -> uuid::Uuid {
    // Lookup or create organization
    todo!()
}
```

### 2. Configuration (`config.rs`)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {Org}Config {
    pub api_url: String,
    pub api_key: Option<String>,
    pub timeout_secs: u64,
    pub parse_limit: Option<usize>,
}

impl Default for {Org}Config {
    fn default() -> Self {
        Self {
            api_url: "{default API URL}".to_string(),
            api_key: std::env::var("{ORG}_API_KEY").ok(),
            timeout_secs: 30,
            parse_limit: None,
        }
    }
}

/// Versioning rules (Markdown)
pub const VERSIONING_RULES: &str = r#"
# {Organization} Versioning Rules

## MAJOR Version Bump
- {Breaking change 1}
- {Breaking change 2}

## MINOR Version Bump
- {Non-breaking change 1}
- {Non-breaking change 2}

## PATCH Version Bump
- {Minor correction 1}
- {Minor correction 2}

## License
{License info with attribution}
"#;
```

### 3. Models (`models.rs`)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {DataType}Entry {
    pub id: String,
    pub name: String,
    // ... fields specific to this data type
}

impl {DataType}Entry {
    /// Convert to registry entry
    pub fn to_registry_entry(&self, org_id: uuid::Uuid) -> CreateRegistryEntry {
        CreateRegistryEntry {
            slug: self.id.to_lowercase(),
            name: self.name.clone(),
            organization_id: org_id,
            entry_type: "data_source",
            license_id: {org_license_id},
        }
    }

    /// Convert to data source
    pub fn to_data_source(&self) -> CreateDataSource {
        CreateDataSource {
            source_type: "{data_type}",
            external_id: self.id.clone(),
        }
    }
}
```

### 4. Versioning Logic (`versioning.rs`)

```rust
use super::models::*;

pub enum VersionBump {
    Major,
    Minor,
    Patch,
    None,
}

/// Determine version bump based on changes
pub fn determine_version_bump(
    old: &{DataType}Metadata,
    new: &{DataType}Entry,
) -> VersionBump {
    // MAJOR: Breaking changes
    if {critical_field_changed} {
        return VersionBump::Major;
    }

    // MINOR: Non-breaking additions
    if {metadata_changed} {
        return VersionBump::Minor;
    }

    // PATCH: Minor corrections
    if {minor_change} {
        return VersionBump::Patch;
    }

    VersionBump::None
}

/// Calculate new version number
pub fn calculate_new_version(
    current: (i32, i32, i32),
    bump: VersionBump,
) -> (i32, i32, i32) {
    match bump {
        VersionBump::Major => (current.0 + 1, 0, 0),
        VersionBump::Minor => (current.0, current.1 + 1, 0),
        VersionBump::Patch => (current.0, current.1, current.2 + 1),
        VersionBump::None => current,
    }
}
```

### 5. Changelog Generation (`changelog.rs`)

```rust
use super::models::*;
use super::versioning::*;

/// Generate changelog for version
pub fn generate_changelog(
    old: &{DataType}Metadata,
    new: &{DataType}Entry,
    bump: &VersionBump,
) -> String {
    let mut major = vec![];
    let mut minor = vec![];
    let mut patch = vec![];

    // Detect changes
    if {critical_change} {
        major.push(format!("{field} changed: {} → {}", old.field, new.field));
    }

    if {metadata_change} {
        minor.push(format!("{field} updated: {} → {}", old.field, new.field));
    }

    if {minor_correction} {
        patch.push(format!("Fixed typo in {field}"));
    }

    // Format as Markdown
    format_changelog(major, minor, patch)
}

fn format_changelog(
    major: Vec<String>,
    minor: Vec<String>,
    patch: Vec<String>,
) -> String {
    let mut sections = vec![];

    if !major.is_empty() {
        sections.push(format!("### MAJOR Changes\n{}", major.join("\n- ")));
    }

    if !minor.is_empty() {
        sections.push(format!("### MINOR Changes\n{}", minor.join("\n- ")));
    }

    if !patch.is_empty() {
        sections.push(format!("### PATCH Changes\n{}", patch.join("\n- ")));
    }

    sections.join("\n\n")
}
```

### 6. Pipeline (`pipeline.rs`)

```rust
use super::*;
use crate::ingest::framework::*;
use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

pub struct {Org}Pipeline {
    pool: Arc<PgPool>,
    organization_id: Uuid,
    config: {Org}Config,
    storage: Arc<Storage>,
}

impl {Org}Pipeline {
    pub fn new(
        pool: Arc<PgPool>,
        organization_id: Uuid,
        config: {Org}Config,
        storage: Arc<Storage>,
    ) -> Self {
        Self {
            pool,
            organization_id,
            config,
            storage,
        }
    }

    /// Ingest a specific version
    pub async fn ingest_version(&self, external_version: &str) -> Result<Uuid> {
        // 1. Download data
        let entries = self.fetch_data(external_version).await?;

        // 2. Determine internal version
        let internal_version = self.get_or_create_version_mapping(external_version).await?;

        // 3. Process each entry
        for entry in entries {
            self.process_entry(&entry, &internal_version, external_version).await?;
        }

        Ok(job_id)
    }

    async fn fetch_data(&self, version: &str) -> Result<Vec<{DataType}Entry>> {
        // Implementation specific to data source
        todo!()
    }

    async fn process_entry(
        &self,
        entry: &{DataType}Entry,
        internal_version: &str,
        external_version: &str,
    ) -> Result<()> {
        // Check if exists
        let existing = find_by_external_id(&entry.id).await?;

        match existing {
            None => self.create_new_entry(entry, internal_version, external_version).await?,
            Some(existing) => self.update_existing_entry(existing, entry).await?,
        }

        Ok(())
    }

    async fn create_new_entry(
        &self,
        entry: &{DataType}Entry,
        internal_version: &str,
        external_version: &str,
    ) -> Result<()> {
        // 1. Create registry entry
        let registry_id = create_registry_entry(entry.to_registry_entry(self.organization_id)).await?;

        // 2. Create data source
        create_data_source(entry.to_data_source()).await?;

        // 3. Create metadata
        create_{data_type}_metadata(...).await?;

        // 4. Create version 1.0.0
        let (major, minor, patch) = parse_version(internal_version)?;
        create_version(CreateVersion {
            data_source_id: registry_id,
            version_major: major,
            version_minor: minor,
            version_patch: patch,
            changelog: "Initial version".to_string(),
            external_version: external_version.to_string(),
        }).await?;

        // 5. Upload files to S3
        let s3_key = format!("sources/{org}/{slug}/{version}/{file}",
            org = "org_slug",
            slug = entry.id.to_lowercase(),
            version = internal_version,
            file = "file.ext"
        );
        self.storage.upload(&s3_key, &data).await?;

        // 6. Create version file
        create_version_file(...).await?;

        Ok(())
    }

    async fn update_existing_entry(
        &self,
        existing: DataSource,
        new_entry: &{DataType}Entry,
    ) -> Result<()> {
        // 1. Get current metadata
        let old_metadata = get_{data_type}_metadata(existing.id).await?;

        // 2. Determine version bump
        let bump = determine_version_bump(&old_metadata, new_entry);

        if bump == VersionBump::None {
            return Ok(());  // No changes
        }

        // 3. Get latest version
        let latest_version = get_latest_version(existing.id).await?;

        // 4. Calculate new version
        let (new_major, new_minor, new_patch) = calculate_new_version(
            (latest_version.major, latest_version.minor, latest_version.patch),
            bump,
        );

        // 5. Generate changelog
        let changelog = generate_changelog(&old_metadata, new_entry, &bump)?;

        // 6. Update metadata
        update_{data_type}_metadata(...).await?;

        // 7. Create new version
        create_version(CreateVersion {
            data_source_id: existing.id,
            version_major: new_major,
            version_minor: new_minor,
            version_patch: new_patch,
            changelog,
            external_version: external_version.to_string(),
        }).await?;

        // 8. Upload new files
        // ...

        Ok(())
    }
}
```

---

## Organization-Specific Implementations

### UniProt Module

**Data Types**: Proteins
**Source**: FTP (DAT files)
**Versioning**: Sequence-based (MAJOR) + Annotation (MINOR)

```rust
// crates/bdp-server/src/ingest/uniprot/mod.rs
mod config;      // FTP config
mod ftp;         // FTP client
mod parser;      // DAT parser
mod models;      // UniProtEntry
mod versioning;  // Sequence change detection
mod changelog;   // Auto-generate from DAT diff
mod pipeline;    // Main pipeline

pub use config::UniProtConfig;
pub use pipeline::UniProtPipeline;
```

**Key Features**:
- Sequence deduplication via `protein_sequences` table
- Auto-changelog from sequence diff
- Bundle support for SwissProt

### NCBI Module

**Data Types**: Organisms (Taxonomy), Genomes (RefSeq)
**Source**: REST API + FTP
**Versioning**: Assembly-based (MAJOR) + Annotation (MINOR)

```rust
// crates/bdp-server/src/ingest/ncbi/mod.rs
pub mod taxonomy;   // NCBI Taxonomy
pub mod refseq;     // RefSeq genomes

mod config;

pub use config::NCBIConfig;
```

**Submodules**:

1. **Taxonomy** (`ncbi/taxonomy/`)
   - Parses NCBI Taxonomy XML
   - Creates organisms as data_sources
   - Tracks taxonomic changes (MAJOR version on reclassification)

2. **RefSeq** (`ncbi/refseq/`)
   - Parses GenBank format
   - Creates genome data_sources
   - Links to organisms

### Ensembl Module

**Data Types**: Genes, Transcripts, Genomes
**Source**: REST API
**Versioning**: Release-based (matches Ensembl releases)

```rust
// crates/bdp-server/src/ingest/ensembl/mod.rs
mod config;
mod rest_client;
mod parser;       // JSON parser
mod models;
mod versioning;   // Ensembl release mapping
mod pipeline;

pub use pipeline::EnsemblPipeline;
```

### PDB Module

**Data Types**: Protein Structures
**Source**: REST API + mmCIF files
**Versioning**: Structure-based (MAJOR on coordinate change)

```rust
// crates/bdp-server/src/ingest/pdb/mod.rs
mod config;
mod rest_client;
mod parser;       // mmCIF parser
mod models;
mod pipeline;

pub use pipeline::PDBPipeline;
```

### KEGG Module

**Data Types**: Pathways, Reactions
**Source**: REST API
**Versioning**: Pathway-based (MAJOR on pathway topology change)

```rust
// crates/bdp-server/src/ingest/kegg/mod.rs
mod config;
mod rest_client;
mod parser;
mod models;
mod pipeline;

pub use pipeline::KEGGPipeline;
```

---

## Examples Directory

```
crates/bdp-server/examples/
├── ingest_uniprot.rs           # Ingest UniProt release
├── ingest_ncbi_taxonomy.rs     # Ingest NCBI Taxonomy
├── ingest_ncbi_refseq.rs       # Ingest RefSeq genome
├── ingest_ensembl.rs           # Ingest Ensembl release
├── ingest_pdb.rs               # Ingest PDB structures
└── ingest_kegg.rs              # Ingest KEGG pathways
```

**Example Template**:

```rust
// examples/ingest_{org}.rs
use anyhow::Result;
use bdp_server::ingest::{org}::{OrgPipeline, OrgConfig};
use bdp_server::storage::{Storage, StorageConfig};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Connect to database
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Initialize storage
    let storage_config = StorageConfig::from_env()?;
    let storage = Storage::new(storage_config).await?;

    // Get organization
    let org_id = get_or_create_organization(&pool, "{org_slug}").await?;

    // Create pipeline
    let config = OrgConfig::default();
    let pipeline = OrgPipeline::new(
        Arc::new(pool),
        org_id,
        config,
        Arc::new(storage),
    );

    // Ingest version
    let external_version = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "latest".to_string());

    pipeline.ingest_version(&external_version).await?;

    Ok(())
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_bump_detection() {
        let old = /* old metadata */;
        let new = /* new entry */;
        let bump = determine_version_bump(&old, &new);
        assert_eq!(bump, VersionBump::Major);
    }

    #[test]
    fn test_changelog_generation() {
        let changelog = generate_changelog(&old, &new, &VersionBump::Minor);
        assert!(changelog.contains("MINOR Changes"));
    }
}
```

### Integration Tests

```rust
#[sqlx::test]
async fn test_full_ingestion_pipeline(pool: PgPool) {
    let org_id = create_test_organization(&pool).await;
    let pipeline = OrgPipeline::new(Arc::new(pool.clone()), org_id, ...);

    let job_id = pipeline.ingest_version("test_version").await.unwrap();

    // Verify registry entry created
    let entry = get_registry_entry(&pool, "test_slug").await.unwrap();
    assert_eq!(entry.entry_type, "data_source");

    // Verify version created
    let version = get_latest_version(&pool, entry.id).await.unwrap();
    assert_eq!(version.version_string, "1.0.0");
}
```

---

## CLI Integration

Each module can be triggered via CLI:

```bash
# Ingest specific organization
bdp-cli ingest uniprot --version 2025_01
bdp-cli ingest ncbi-taxonomy --all
bdp-cli ingest ensembl --release 110
```

**CLI Implementation**:

```rust
// crates/bdp-cli/src/commands/ingest.rs
#[derive(Debug, clap::Parser)]
pub struct IngestCommand {
    /// Organization to ingest from
    #[clap(subcommand)]
    organization: OrganizationCommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum OrganizationCommand {
    Uniprot {
        #[clap(long)]
        version: String,
    },
    NcbiTaxonomy {
        #[clap(long)]
        all: bool,
    },
    Ensembl {
        #[clap(long)]
        release: String,
    },
    // ... more organizations
}
```

---

## Implementation Priority

1. **Phase 1**: UniProt (proteins) ✅ Already in progress
2. **Phase 2**: NCBI Taxonomy (organisms) - Required for protein metadata
3. **Phase 3**: NCBI RefSeq (genomes)
4. **Phase 4**: Ensembl (genes, transcripts)
5. **Phase 5**: PDB (structures)
6. **Phase 6**: KEGG (pathways)

---

**End of Document**
