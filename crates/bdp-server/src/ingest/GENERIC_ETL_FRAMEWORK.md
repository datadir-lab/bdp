# Generic ETL Ingestion Framework

## Overview

A **distributed, parallel, idempotent ETL pipeline** that works with ANY data source type:
- UniProt proteins
- NCBI genomes
- PubChem compounds
- PubMed papers
- And any other structured data source

All state tracked in PostgreSQL for resilience, resumability, and observability.

## Design Principles

### 1. Source-Agnostic
The framework doesn't care what you're ingesting:
```rust
// UniProt proteins
record_type = "protein"
record_identifier = "p01234"
record_data = {accession, sequence, organism, ...}

// NCBI genomes
record_type = "genome"
record_identifier = "GCF_000001405.40"
record_data = {assembly, chromosomes, organism, ...}

// PubChem compounds
record_type = "compound"
record_identifier = "CID_2244"
record_data = {molecular_formula, smiles, inchi, ...}
```

### 2. Parallel & Distributed
Multiple workers process different batches simultaneously:
```
Worker 1: Processing proteins 0-999
Worker 2: Processing proteins 1000-1999
Worker 3: Processing proteins 2000-2999
... all at the same time!
```

### 3. Idempotent & Resumable
Crash at any point, resume from last checkpoint:
- All state in database (no file markers)
- Automatic dead worker detection
- Retry failed batches (with limits)

### 4. MD5 Verified
Every file has MD5 checksums:
- Raw downloads: verified against source metadata
- Generated files: computed and stored
- Stored in database: `data_sources.primary_file_md5`, `version_files.checksum`

## Database Schema

### Job Lifecycle Tables

```sql
ingestion_jobs              -- Main job tracker
├── ingestion_raw_files     -- Downloaded files (ingest/ folder)
├── ingestion_work_units    -- Parallelizable batches
├── ingestion_staged_records -- Parsed data (JSONB staging)
├── ingestion_file_uploads  -- S3 uploads to final location
└── ingestion_job_logs      -- Audit trail
```

### Generic Record Storage

```sql
-- Staging area (type-agnostic)
CREATE TABLE ingestion_staged_records (
    record_type VARCHAR(100),        -- 'protein', 'genome', 'compound', etc.
    record_identifier VARCHAR(500),  -- Primary ID (accession, genome_id, CID, etc.)
    record_data JSONB,               -- Flexible: any structure
    content_md5 VARCHAR(32),         -- MD5 for deduplication
    sequence_md5 VARCHAR(32)         -- MD5 of main content (optional)
);
```

**Example data**:

```json
// UniProt protein
{
    "record_type": "protein",
    "record_identifier": "p01234",
    "record_data": {
        "accession": "P01234",
        "entry_name": "001r_frg3g",
        "protein_name": "Putative transcription factor 001R",
        "sequence": "MAFSAEDVLKEYDRR...",
        "organism": "Frog virus 3",
        "taxonomy_id": 654924
    }
}

// NCBI genome
{
    "record_type": "genome",
    "record_identifier": "gcf_000001405.40",
    "record_data": {
        "assembly_accession": "GCF_000001405.40",
        "organism": "Homo sapiens",
        "assembly_name": "GRCh38.p14",
        "chromosomes": 24,
        "total_length": 3099734149
    }
}

// PubChem compound
{
    "record_type": "compound",
    "record_identifier": "cid_2244",
    "record_data": {
        "cid": 2244,
        "iupac_name": "aspirin",
        "molecular_formula": "C9H8O4",
        "smiles": "CC(=O)Oc1ccccc1C(=O)O",
        "molecular_weight": 180.16
    }
}
```

## Pipeline Stages

### Stage 1: Download & Verify

**Objective**: Download raw files with MD5 verification

```rust
// Create job
let job = create_job(CreateJobParams {
    organization_id: org_id,
    job_type: "uniprot_swissprot",     // or "ncbi_genome", etc.
    external_version: "2025_06",
    internal_version: "1.0",
    source_url: "https://ftp.uniprot.org/...",
    source_metadata: json!({
        "dataset": "sprot",
        "format": "dat"
    })
}).await?;

// Download with MD5 verification
download_and_verify(job_id, DownloadConfig {
    metalink_url: "https://ftp.uniprot.org/.../RELEASE.metalink",
    files: vec![
        FileToDownload {
            file_type: "dat",
            url: "https://ftp.uniprot.org/.../uniprot_sprot.dat.gz",
            expected_md5: Some("e3cd39d0c48231aa5abb3eca81b3c62a"),
            s3_key: "ingest/uniprot/2025_06/uniprot_sprot.dat.gz"
        }
    ]
}).await?;
```

**S3 Structure**:
```
ingest/
├── uniprot/2025_06/
│   ├── RELEASE.metalink
│   ├── uniprot_sprot.dat.gz        (MD5 verified)
│   └── uniprot_sprot.dat.gz.md5    (checksum file)
├── ncbi_genome/GRCh38.p14/
│   ├── GRCh38_latest_genomic.fna.gz
│   └── md5checksums.txt
└── pubchem/2025-01-15/
    ├── CID-Compound.xml.gz
    └── CID-Compound.xml.gz.md5
```

**Database**:
```sql
INSERT INTO ingestion_raw_files (job_id, file_type, s3_key, expected_md5, computed_md5, verified_md5, status)
VALUES ($1, 'dat', 'ingest/uniprot/2025_06/uniprot_sprot.dat.gz', 'e3cd...', 'e3cd...', TRUE, 'verified');
```

**Idempotent**: Skip if `status = 'verified'`

### Stage 2: Parse (Parallel)

**Objective**: Parse raw files into `ingestion_staged_records`

**Create Work Units**:
```sql
-- For 570k proteins, create 570 batches of 1000 each
INSERT INTO ingestion_work_units (job_id, unit_type, batch_number, start_offset, end_offset, record_count)
SELECT
    $1,                                  -- job_id
    'parse_batch',                       -- unit_type
    batch_num,                           -- batch_number
    batch_num * 1000,                    -- start_offset
    (batch_num + 1) * 1000 - 1,         -- end_offset
    1000                                 -- record_count
FROM generate_series(0, 569) as batch_num;
```

**Worker Claims Batch** (atomic):
```sql
SELECT * FROM claim_work_unit(
    p_job_id := $1,
    p_worker_id := $2,
    p_worker_hostname := 'worker-pod-123',
    p_unit_type := 'parse_batch'
);
-- Returns: {unit_id, batch_number: 42, start_offset: 42000, end_offset: 42999}
```

**Worker Processes Batch**:
```rust
// Worker claims batch 42: proteins 42000-42999
let unit = claim_work_unit(job_id, worker_id).await?;

// Download raw file from S3 (cached locally)
let raw_data = s3.download("ingest/uniprot/2025_06/uniprot_sprot.dat.gz").await?;

// Parse the specific range
let parser = get_parser_for_job_type(job.job_type).await?;
let records = parser.parse_range(&raw_data, unit.start_offset, unit.end_offset)?;

// Insert to staging (batch insert)
for record in records {
    let content_md5 = compute_md5(&serde_json::to_vec(&record.data)?);
    let sequence_md5 = record.data.get("sequence")
        .map(|s| compute_md5(s.as_str().unwrap().as_bytes()));

    insert_staged_record(InsertStagedRecord {
        job_id,
        work_unit_id: unit.id,
        record_type: "protein",
        record_identifier: record.accession.to_lowercase(),
        record_name: Some(record.entry_name.to_lowercase()),
        record_data: serde_json::to_value(&record)?,
        content_md5,
        sequence_md5,
        source_file: "ingest/uniprot/2025_06/uniprot_sprot.dat.gz",
        source_offset: record.file_offset
    }).await?;
}

// Mark unit complete
mark_work_unit_complete(unit.id).await?;
```

**Generic Parser Interface**:
```rust
trait DataSourceParser {
    async fn parse_range(&self, data: &[u8], start: usize, end: usize) -> Result<Vec<GenericRecord>>;
}

struct UniProtParser;
impl DataSourceParser for UniProtParser { ... }

struct NCBIGenomeParser;
impl DataSourceParser for NCBIGenomeParser { ... }

struct PubChemParser;
impl DataSourceParser for PubChemParser { ... }
```

**Idempotent**: Skip records already in `ingestion_staged_records`

### Stage 3: Store (Parallel)

**Objective**: Upload files to S3, insert into final tables

**Create Store Units**:
```sql
-- Create batches of 100 records each
INSERT INTO ingestion_work_units (job_id, unit_type, batch_number, start_offset, end_offset)
SELECT
    $1,
    'store_batch',
    (row_number() OVER () - 1) / 100,
    id,
    id
FROM ingestion_staged_records
WHERE job_id = $1 AND status = 'staged';
```

**Worker Processes Store Batch**:
```rust
// Claim batch of 100 staged records
let unit = claim_work_unit(job_id, worker_id, "store_batch").await?;
let records = get_staged_records(unit).await?;

// Begin transaction (batched for performance)
let mut tx = pool.begin().await?;

for record in records {
    // 1. Generate files based on record type
    let files = generate_files_for_record(&record)?;
    // For protein: {dat, fasta, json}
    // For genome: {fasta, gff, json}
    // For compound: {sdf, json}

    // 2. Upload to S3 (outside transaction)
    for (format, content) in files {
        let md5 = compute_md5(&content);
        let s3_key = format!(
            "{}/{}/{}/{}.{}",
            org_slug,                    // "uniprot"
            record.record_identifier,    // "p01234"
            internal_version,            // "1.0"
            record.record_identifier,    // "p01234"
            format                       // "fasta"
        );

        s3.upload(&s3_key, content, get_content_type(format)).await?;

        // Track upload with MD5
        insert_file_upload(&tx, InsertFileUpload {
            job_id,
            staged_record_id: record.id,
            format,
            s3_key: s3_key.clone(),
            size_bytes: content.len() as i64,
            md5_checksum: md5,
            content_type: get_content_type(format)
        }).await?;
    }

    // 3. Insert into type-specific tables (in transaction)
    match record.record_type.as_str() {
        "protein" => {
            insert_protein_to_final_tables(&tx, &record, &files).await?;
        }
        "genome" => {
            insert_genome_to_final_tables(&tx, &record, &files).await?;
        }
        "compound" => {
            insert_compound_to_final_tables(&tx, &record, &files).await?;
        }
        _ => return Err(anyhow!("Unknown record type"))
    }

    // 4. Mark staged record as stored
    mark_staged_record_stored(&tx, record.id).await?;
}

// Commit batch transaction (all 100 records atomic)
tx.commit().await?;

// Mark work unit complete
mark_work_unit_complete(unit.id).await?;
```

**Final S3 Structure**:
```
uniprot/                     (organization slug)
├── p01234/                  (data_source slug - lowercase)
│   └── 1.0/                 (internal version)
│       ├── p01234.dat       (MD5: abc123...)
│       ├── p01234.fasta     (MD5: def456...)
│       └── p01234.json      (MD5: ghi789...)
├── q6gzx4/
│   └── 1.0/
│       ├── q6gzx4.dat
│       ├── q6gzx4.fasta
│       └── q6gzx4.json

ncbi_genome/
├── gcf_000001405.40/
│   └── 1.0/
│       ├── gcf_000001405.40.fasta
│       ├── gcf_000001405.40.gff
│       └── gcf_000001405.40.json

pubchem/
├── cid_2244/
│   └── 1.0/
│       ├── cid_2244.sdf
│       └── cid_2244.json
```

**Database Final Tables**:
```sql
-- Generic: registry_entries, data_sources, versions, version_files
INSERT INTO data_sources (id, ..., primary_file_md5, metadata_md5)
VALUES ($1, ..., 'abc123...', 'def456...');  -- Store MD5s for audit

INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes)
VALUES ($1, 'fasta', 'uniprot/p01234/1.0/p01234.fasta', 'abc123...', 12345);

-- Type-specific: protein_metadata, genome_metadata, compound_metadata
INSERT INTO protein_metadata (data_source_id, accession, sequence, ...)
VALUES (...);
```

**Idempotent**: Check if record exists in final tables before inserting

## MD5 Checksum Tracking

### 1. Download Verification
```rust
// Parse metalink/checksum file
let expected_md5 = parse_metalink(metalink_content)?
    .get_md5_for_file("uniprot_sprot.dat.gz")?;

// Download file
let file_data = download(url).await?;

// Verify MD5
let computed_md5 = compute_md5(&file_data);
if expected_md5 != computed_md5 {
    return Err("MD5 mismatch - download corrupted");
}

// Store in database
UPDATE ingestion_raw_files
SET expected_md5 = $1, computed_md5 = $2, verified_md5 = TRUE
WHERE id = $3;
```

### 2. Generated File Checksums
```rust
// Generate file content
let fasta_content = protein.to_fasta();

// Compute MD5
let md5 = compute_md5(fasta_content.as_bytes());

// Upload to S3
s3.upload(&s3_key, fasta_content, "text/plain").await?;

// Store MD5 in multiple places
INSERT INTO ingestion_file_uploads (md5_checksum) VALUES ($md5);
INSERT INTO version_files (checksum) VALUES ($md5);
UPDATE data_sources SET primary_file_md5 = $md5 WHERE id = $id;
```

### 3. Audit Query
```sql
-- Verify all checksums for a job
SELECT
    ifu.format,
    ifu.s3_key,
    ifu.md5_checksum as upload_md5,
    vf.checksum as version_file_md5,
    ds.primary_file_md5 as data_source_md5,
    CASE
        WHEN ifu.md5_checksum = vf.checksum
         AND ifu.md5_checksum = ds.primary_file_md5
        THEN 'VALID'
        ELSE 'MISMATCH'
    END as status
FROM ingestion_file_uploads ifu
LEFT JOIN version_files vf ON vf.s3_key = ifu.s3_key
LEFT JOIN data_sources ds ON ds.id = (
    SELECT data_source_id FROM protein_metadata
    WHERE accession = split_part(ifu.s3_key, '/', 2)  -- Extract from path
)
WHERE ifu.job_id = $1;
```

## Worker Coordination

### Atomic Work Unit Claim
```sql
-- PostgreSQL function with SKIP LOCKED
SELECT * FROM claim_work_unit(
    p_job_id := '123...',
    p_worker_id := '456...',
    p_worker_hostname := 'worker-pod-5'
);

-- Returns first available unit, skips locked rows (no waiting!)
```

### Heartbeat System
```rust
// Worker spawns heartbeat task
tokio::spawn(async move {
    loop {
        update_heartbeat(work_unit_id, worker_id).await;
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
});
```

### Dead Worker Recovery
```sql
-- Cron job runs every minute
SELECT reclaim_stale_work_units(120);  -- 2 minute timeout

-- Returns stale units to 'pending' with retry_count++
```

## Example: Adding a New Data Source

### 1. Define Record Structure
```rust
#[derive(Serialize, Deserialize)]
struct GenomeRecord {
    assembly_accession: String,
    organism: String,
    assembly_name: String,
    chromosomes: Vec<Chromosome>,
    total_length: u64,
}
```

### 2. Implement Parser
```rust
struct NCBIGenomeParser;

impl DataSourceParser for NCBIGenomeParser {
    async fn parse_range(&self, data: &[u8], start: usize, end: usize) -> Result<Vec<GenericRecord>> {
        // Parse FASTA/GFF files
        let genomes = parse_ncbi_genome_files(data)?;

        Ok(genomes[start..=end].iter().map(|g| GenericRecord {
            record_type: "genome".to_string(),
            record_identifier: g.assembly_accession.to_lowercase(),
            record_name: Some(g.organism.to_lowercase()),
            record_data: serde_json::to_value(g)?,
            ..Default::default()
        }).collect())
    }
}
```

### 3. Implement Storage
```rust
async fn insert_genome_to_final_tables(tx: &mut Transaction<'_, Postgres>, record: &StagedRecord, files: &HashMap<String, Vec<u8>>) -> Result<()> {
    let genome: GenomeRecord = serde_json::from_value(record.record_data.clone())?;

    // Create registry entry
    let entry_id = create_registry_entry(tx, &genome).await?;

    // Create data_source with MD5
    let fasta_md5 = files.get("fasta").map(|f| compute_md5(f));
    create_data_source(tx, entry_id, fasta_md5).await?;

    // Create genome_metadata table (type-specific)
    sqlx::query!(
        "INSERT INTO genome_metadata (data_source_id, assembly_accession, organism, ...)
         VALUES ($1, $2, $3, ...)",
        entry_id, genome.assembly_accession, genome.organism
    ).execute(&mut **tx).await?;

    Ok(())
}
```

### 4. Register in Job System
```rust
// Create job
create_job(CreateJobParams {
    job_type: "ncbi_genome_grch38",  // New job type!
    external_version: "GRCh38.p14",
    ...
}).await?;

// Framework handles the rest automatically!
```

## Configuration

```rust
// config/ingestion.toml
[ingestion]
parse_batch_size = 1000      # Records per parse batch
store_batch_size = 100       # Records per store batch
max_retries = 3              # Max retries per work unit
heartbeat_interval_secs = 30 # Worker heartbeat frequency
worker_timeout_secs = 120    # Dead worker detection threshold

[s3]
ingest_prefix = "ingest/"    # Raw downloads
archive_prefix = "archive/"  # Archived ingests (lifecycle: 90 days)

[job_types.uniprot_swissprot]
parser = "UniProtParser"
formats = ["dat", "fasta", "json"]

[job_types.ncbi_genome]
parser = "NCBIGenomeParser"
formats = ["fasta", "gff", "json"]

[job_types.pubchem]
parser = "PubChemParser"
formats = ["sdf", "json"]
```

## Monitoring

```sql
-- Job progress dashboard
SELECT
    j.job_type,
    j.external_version,
    j.status,
    j.records_processed || '/' || j.total_records as progress,
    COUNT(wu.id) FILTER (WHERE wu.status = 'completed') || '/' || COUNT(wu.id) as work_units,
    ROUND(100.0 * j.records_processed / NULLIF(j.total_records, 0), 2) as percent_complete
FROM ingestion_jobs j
LEFT JOIN ingestion_work_units wu ON wu.job_id = j.id
GROUP BY j.id;

-- Active workers
SELECT
    wu.worker_hostname,
    COUNT(*) as active_units,
    MAX(wu.heartbeat_at) as last_heartbeat
FROM ingestion_work_units wu
WHERE wu.status = 'claimed'
GROUP BY wu.worker_hostname;
```

## Benefits

✅ **Generic**: Works with any data type (proteins, genomes, compounds, papers)
✅ **Parallel**: N workers process N batches simultaneously
✅ **Distributed**: Workers on different machines/pods
✅ **Resumable**: Crash-safe with database checkpoints
✅ **Idempotent**: Safe to retry any operation
✅ **Verified**: MD5 checksums at every stage
✅ **Observable**: Full visibility via database queries
✅ **Auditable**: Complete MD5 trail for compliance
✅ **Scalable**: Add workers to speed up processing
