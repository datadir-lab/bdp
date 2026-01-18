# Ingestion Pipeline Orchestrator Implementation Summary

## Overview

This document summarizes the implementation of the UniProt ingestion pipeline orchestrator using CQRS commands. The pipeline orchestrates: download → parse → upload to S3 → insert via CQRS commands.

## Implementation Status

### ✅ Completed Components

#### 1. CQRS Commands

**CreateOrganismCommand** (`crates/bdp-server/src/features/organisms/commands/create.rs`)
- Creates or updates organism records
- Uses ON CONFLICT to handle duplicates
- Validates taxonomy_id and scientific_name
- 10+ tests included

**InsertProteinMetadataCommand** (`crates/bdp-server/src/features/protein_metadata/commands/insert.rs`)
- Inserts protein metadata (accession, entry_name, gene_name, sequence_length, mass_da, checksum)
- Upserts on accession conflict
- Validates accession format (alphanumeric only)
- 15+ tests included

**AddVersionFilesCommand** (`crates/bdp-server/src/features/version_files/commands/add.rs`)
- Batch adds version files (FASTA, JSON, etc.)
- Upserts on (version_id, format) conflict
- Validates file metadata (format, s3_key, checksum, size)
- 10+ tests included

**PublishVersionCommand** (`crates/bdp-server/src/features/data_sources/commands/publish.rs`)
- Already exists
- Publishes new version for a data source
- Tracks external_version, release_date, size_bytes

**CreateDataSourceCommand** (`crates/bdp-server/src/features/data_sources/commands/create.rs`)
- Already exists
- Creates data source registry entry
- Links to organism_id

#### 2. Version Mapping Module

**VersionMapper** (`crates/bdp-server/src/ingest/version_mapping.rs`)
- Maps external versions (2024_01) to internal semantic versions (1.0)
- Logic:
  - First version → 1.0
  - Same year increment → minor (1.1, 1.2, ...)
  - New year → major (2.0, 3.0, ...)
- Stores mappings in `version_mappings` table
- 10+ tests included

### 🚧 Remaining Components

#### 3. UniProt Parser

**Required Files:**
- `crates/bdp-server/src/ingest/uniprot/models.rs` - UniProtEntry, ReleaseInfo
- `crates/bdp-server/src/ingest/uniprot/dat_parser.rs` - DAT file parser
- `crates/bdp-server/src/ingest/uniprot/ftp_client.rs` - FTP download client
- `crates/bdp-server/src/ingest/uniprot/mod.rs` - Module exports

**UniProtEntry Structure:**
```rust
pub struct UniProtEntry {
    pub accession: String,         // P01308
    pub entry_name: String,        // INS_HUMAN
    pub protein_name: String,      // Insulin
    pub gene_name: Option<String>, // INS
    pub organism_name: String,     // Homo sapiens
    pub taxonomy_id: i32,          // 9606
    pub sequence: String,          // MALWMRLLPL...
    pub sequence_length: i32,      // 110
    pub mass_da: i64,              // 11937
}
```

**ReleaseInfo Structure:**
```rust
pub struct ReleaseInfo {
    pub external_version: String,  // 2024_01
    pub release_date: NaiveDate,   // 2025-01-15
    pub swissprot_count: u64,      // 570000
    pub trembl_count: u64,         // 250000000
}
```

#### 4. Pipeline Orchestrator

**Required File:** `crates/bdp-server/src/ingest/uniprot/pipeline.rs`

**Core Structure:**
```rust
pub struct IngestionPipeline {
    mediator: Box<dyn AsyncMediator>,
    pool: PgPool,
    storage: Storage,
    config: UniProtConfig,
}

impl IngestionPipeline {
    pub async fn run(&self, job: &UniProtIngestJob) -> Result<IngestStats> {
        // 1. Ensure organization exists
        let org_id = self.ensure_organization().await?;

        // 2. Download and parse
        let (entries, release_info) = self.download_and_parse(
            &job.version,
            job.limit
        ).await?;

        // 3. Map version
        let mapper = VersionMapper::new(self.pool.clone(), "uniprot".to_string());
        let internal_version = mapper.map_version(
            &release_info.external_version,
            release_info.release_date
        ).await?;

        // 4. Ingest entries
        let stats = self.ingest_entries(&entries, &internal_version, &release_info).await?;

        // 5. Update sync status
        self.update_sync_status(org_id, &internal_version, &release_info.external_version, &stats).await?;

        Ok(stats)
    }

    async fn ingest_uniprot_entry(
        &self,
        entry: &UniProtEntry,
        org_id: Uuid,
        internal_version: &str,
        external_version: &str
    ) -> Result<u64> {
        // 1. CreateOrganismCommand (ON CONFLICT returns existing)
        let organism_resp = self.mediator.send(CreateOrganismCommand {
            taxonomy_id: entry.taxonomy_id,
            scientific_name: entry.organism_name.clone(),
            common_name: None,
        }).await??;

        // 2. CreateDataSourceCommand
        let ds_resp = self.mediator.send(CreateDataSourceCommand {
            organization_id: org_id,
            slug: entry.accession.to_lowercase(),
            name: format!("{} [{}]", entry.protein_name, entry.organism_name),
            description: Some(format!("UniProt protein: {}", entry.protein_name)),
            source_type: "protein".to_string(),
            external_id: Some(entry.accession.clone()),
            organism_id: Some(organism_resp.id),
            additional_metadata: None,
        }).await??;

        // 3. Upload FASTA and JSON to S3
        let (fasta_result, json_result) = self.upload_files(
            entry,
            internal_version
        ).await?;

        // 4. PublishVersionCommand
        let version_resp = self.mediator.send(PublishVersionCommand {
            data_source_id: ds_resp.id,
            version: internal_version.to_string(),
            external_version: Some(external_version.to_string()),
            release_date: Some(release_info.release_date),
            size_bytes: Some(fasta_result.size + json_result.size),
            additional_metadata: None,
        }).await??;

        // 5. AddVersionFilesCommand (batch both files)
        self.mediator.send(AddVersionFilesCommand {
            version_id: version_resp.id,
            files: vec![
                VersionFileInput {
                    format: "fasta".to_string(),
                    s3_key: fasta_result.key,
                    checksum: fasta_result.checksum,
                    size_bytes: fasta_result.size,
                    compression: None,
                },
                VersionFileInput {
                    format: "json".to_string(),
                    s3_key: json_result.key,
                    checksum: json_result.checksum,
                    size_bytes: json_result.size,
                    compression: None,
                },
            ],
        }).await??;

        // 6. InsertProteinMetadataCommand
        self.mediator.send(InsertProteinMetadataCommand {
            data_source_id: ds_resp.id,
            accession: entry.accession.clone(),
            entry_name: Some(entry.entry_name.clone()),
            protein_name: Some(entry.protein_name.clone()),
            gene_name: entry.gene_name.clone(),
            sequence_length: Some(entry.sequence_length),
            mass_da: Some(entry.mass_da),
            sequence_checksum: Some(calculate_checksum(&entry.sequence)),
        }).await??;

        Ok(fasta_result.size + json_result.size)
    }
}
```

**Progress Tracking:**
```rust
use indicatif::{ProgressBar, ProgressStyle};

async fn ingest_entries(&self, entries: &[UniProtEntry], ...) -> Result<IngestStats> {
    let pb = ProgressBar::new(entries.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("##-")
    );

    let mut stats = IngestStats::default();

    for entry in entries {
        pb.set_message(format!("Processing {}", entry.accession));

        match self.ingest_uniprot_entry(entry, ...).await {
            Ok(size) => {
                stats.success_count += 1;
                stats.total_size += size;
            }
            Err(e) => {
                warn!("Failed to ingest {}: {}", entry.accession, e);
                stats.error_count += 1;
            }
        }

        pb.inc(1);
    }

    pb.finish_with_message("Ingestion complete!");
    Ok(stats)
}
```

#### 5. Job Handler

**Required File:** `crates/bdp-server/src/ingest/job.rs`

```rust
use apalis::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniProtIngestJob {
    pub version: String,           // "2024_01" or "latest"
    pub limit: Option<usize>,      // For testing
    pub databases: Vec<String>,    // ["swissprot", "trembl"]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestStats {
    pub success_count: u64,
    pub error_count: u64,
    pub total_size: u64,
    pub duration_secs: u64,
}

pub async fn handle_job(
    job: UniProtIngestJob,
    ctx: JobContext,
) -> Result<IngestStats, JobError> {
    let pool = ctx.data::<PgPool>().expect("PgPool not found");
    let storage = ctx.data::<Storage>().expect("Storage not found");
    let mediator = ctx.data::<AppMediator>().expect("Mediator not found");

    let config = UniProtConfig::load()?;

    let pipeline = IngestionPipeline {
        mediator: Box::new(mediator.clone()),
        pool: pool.clone(),
        storage: storage.clone(),
        config,
    };

    let start = Instant::now();
    let mut stats = pipeline.run(&job).await?;
    stats.duration_secs = start.elapsed().as_secs();

    Ok(stats)
}
```

#### 6. Mediator Registration

**Update:** `crates/bdp-server/src/cqrs/mod.rs`

```rust
pub fn build_mediator(pool: PgPool) -> AppMediator {
    DefaultAsyncMediator::builder()
        // ... existing handlers ...

        // Organisms
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::organisms::commands::create::handle(pool, cmd).await }
            }
        })

        // Protein Metadata
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::protein_metadata::commands::insert::handle(pool, cmd).await }
            }
        })

        // Version Files
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::version_files::commands::add::handle(pool, cmd).await }
            }
        })

        .build()
}
```

#### 7. Module Updates

**Update:** `crates/bdp-server/src/features/mod.rs`

```rust
pub mod data_sources;
pub mod files;
pub mod organizations;
pub mod organisms;
pub mod protein_metadata;  // ADD THIS
pub mod resolve;
pub mod search;
pub mod version_files;     // ADD THIS
```

**Update:** `crates/bdp-server/src/lib.rs`

```rust
pub mod ingest;  // ADD THIS
```

**Create:** `crates/bdp-server/src/ingest/mod.rs`

```rust
pub mod config;
pub mod job;
pub mod uniprot;
pub mod version_mapping;

pub use config::IngestConfig;
pub use job::{handle_job, IngestStats, UniProtIngestJob};
pub use version_mapping::VersionMapper;
```

#### 8. Docker Compose Environment

**Update:** `docker-compose.yml`

```yaml
services:
  server:
    environment:
      # ... existing vars ...

      # Ingestion Configuration
      INGEST_ENABLED: "true"
      INGEST_UNIPROT_ENABLED: "true"
      INGEST_UNIPROT_BASE_VERSION: "1.0"
      INGEST_UNIPROT_FTP_URL: "ftp://ftp.uniprot.org/pub/databases/uniprot/"
      INGEST_UNIPROT_DATABASES: "swissprot"
      INGEST_PARALLEL_UPLOADS: "10"
      INGEST_BATCH_SIZE: "1000"
```

## Testing Summary

### Unit Tests (Completed: 35+)

1. **CreateOrganismCommand**: 10 tests
2. **InsertProteinMetadataCommand**: 15 tests
3. **AddVersionFilesCommand**: 10 tests
4. **VersionMapper**: 10 tests

### Integration Tests (Required: 5+)

1. **test_full_pipeline_single_entry** - End-to-end ingestion of one protein
2. **test_pipeline_multiple_entries** - Batch ingestion
3. **test_pipeline_handles_duplicates** - Idempotency
4. **test_pipeline_error_recovery** - Skip failures and continue
5. **test_version_mapping_across_years** - Version progression

**Example Integration Test:**

```rust
#[sqlx::test]
async fn test_full_pipeline_single_entry(pool: PgPool) -> sqlx::Result<()> {
    // Setup
    let storage = setup_test_storage().await;
    let mediator = build_mediator(pool.clone());

    let entry = UniProtEntry {
        accession: "P01308".to_string(),
        entry_name: "INS_HUMAN".to_string(),
        protein_name: "Insulin".to_string(),
        gene_name: Some("INS".to_string()),
        organism_name: "Homo sapiens".to_string(),
        taxonomy_id: 9606,
        sequence: "MALWMRLLPL...".to_string(),
        sequence_length: 110,
        mass_da: 11937,
    };

    let pipeline = IngestionPipeline {
        mediator: Box::new(mediator),
        pool: pool.clone(),
        storage,
        config: test_config(),
    };

    // Execute
    let result = pipeline.ingest_uniprot_entry(
        &entry,
        org_id,
        "1.0",
        "2024_01"
    ).await;

    // Assert
    assert!(result.is_ok());

    // Verify organism created
    let organism = sqlx::query!("SELECT * FROM organisms WHERE ncbi_taxonomy_id = 9606")
        .fetch_one(&pool)
        .await?;
    assert_eq!(organism.scientific_name, "Homo sapiens");

    // Verify data source created
    let ds_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM data_sources WHERE external_id = 'P01308'"
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(ds_count, Some(1));

    // Verify version published
    let version = sqlx::query!(
        "SELECT * FROM versions WHERE external_version = '2024_01'"
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(version.version, "1.0");

    // Verify files added
    let file_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM version_files WHERE version_id = $1",
        version.id
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(file_count, Some(2)); // FASTA + JSON

    // Verify protein metadata
    let metadata = sqlx::query!(
        "SELECT * FROM protein_metadata WHERE accession = 'P01308'"
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(metadata.entry_name, Some("INS_HUMAN".to_string()));

    Ok(())
}
```

## File Structure

```
crates/bdp-server/src/
├── features/
│   ├── organisms/
│   │   ├── commands/
│   │   │   ├── create.rs           ✅ (with ON CONFLICT)
│   │   │   └── mod.rs              ✅
│   │   └── mod.rs                  ✅
│   ├── protein_metadata/
│   │   ├── commands/
│   │   │   ├── insert.rs           ✅ (15+ tests)
│   │   │   └── mod.rs              ✅
│   │   └── mod.rs                  ✅
│   ├── version_files/
│   │   ├── commands/
│   │   │   ├── add.rs              ✅ (10+ tests)
│   │   │   └── mod.rs              ✅
│   │   └── mod.rs                  ✅
│   └── mod.rs                      🚧 (needs protein_metadata, version_files exports)
├── ingest/
│   ├── config.rs                   🚧
│   ├── job.rs                      🚧
│   ├── version_mapping.rs          ✅ (10+ tests)
│   ├── uniprot/
│   │   ├── dat_parser.rs           🚧
│   │   ├── ftp_client.rs           🚧
│   │   ├── models.rs               🚧
│   │   ├── pipeline.rs             🚧 (core orchestrator)
│   │   └── mod.rs                  🚧
│   └── mod.rs                      🚧
└── cqrs/mod.rs                     🚧 (needs handler registration)
```

## Success Criteria

- ✅ Version mapping (date-based → semantic)
- ✅ InsertProteinMetadataCommand with 15+ tests
- ✅ AddVersionFilesCommand with 10+ tests
- ✅ CreateOrganismCommand with 10+ tests
- ✅ VersionMapper with 10+ tests
- 🚧 Pipeline uses ONLY CQRS commands (structure defined, needs implementation)
- 🚧 Progress tracking with indicatif (code provided)
- 🚧 Error handling (skip failures) (code provided)
- 🚧 Sync status updates (code provided)
- ✅ 35+ unit tests completed (target: 50+)
- 🚧 5+ integration tests (code provided, needs implementation)
- 🚧 NO direct SQL except version_mappings metadata table (architecture enforced)

## Next Steps

1. Implement UniProt parser (DAT parser, FTP client, models)
2. Implement pipeline orchestrator with all CQRS commands
3. Register new command handlers in mediator
4. Create job handler with apalis
5. Add integration tests
6. Update docker-compose.yml with environment variables
7. Update feature module exports

## Notes

- All CQRS commands use ON CONFLICT for idempotency
- Version mapping uses metadata table (allowed direct SQL)
- Pipeline only uses CQRS commands (no direct SQL)
- Progress bars with indicatif
- Error handling: log and skip failures
- Tests use sqlx::test for database testing
