# UniProt ETL Framework Design

## Overview

Distributed, parallel, idempotent ETL pipeline with database-tracked checkpoints and worker coordination.

## Database-Driven Job Tracking

All state stored in PostgreSQL:
- `ingestion_jobs` - Main job tracker
- `ingestion_work_units` - Parallelizable work batches with worker claims
- `ingestion_raw_files` - Downloaded files with MD5 verification
- `ingestion_parsed_proteins` - Staging area before final commit
- `ingestion_file_uploads` - S3 upload tracking with MD5 audit

## Pipeline Stages

### Stage 1: Download & Verify (Single Worker)

```sql
-- Create job
INSERT INTO ingestion_jobs (organization_id, job_type, external_version, status)
VALUES ($1, 'uniprot_swissprot', '2025_06', 'downloading');

-- Track raw file download
INSERT INTO ingestion_raw_files (job_id, file_type, s3_key, status)
VALUES ($1, 'metalink', 'ingest/uniprot/2025_06/RELEASE.metalink', 'downloading');
```

**Operations**:
1. Download RELEASE.metalink
2. Parse expected MD5 checksums
3. Download uniprot_sprot.dat.gz
4. Verify MD5 checksum
5. Upload to S3: `ingest/uniprot/2025_06/`
6. Mark as `download_verified`

**Idempotent**: Check `ingestion_raw_files` status before re-downloading

### Stage 2: Parse (Parallel Workers)

```sql
-- Create work units for parallel processing
INSERT INTO ingestion_work_units (job_id, unit_type, batch_number, start_offset, end_offset, status)
SELECT
    $1,                          -- job_id
    'parse_batch',
    generate_series / 1000,      -- batch_number
    generate_series,             -- start_offset
    generate_series + 999,       -- end_offset (batches of 1000)
    'pending'
FROM generate_series(0, 569999, 1000);  -- 570 batches for 570k proteins
```

**Worker Claim System**:
```sql
-- Worker claims a batch (atomic)
UPDATE ingestion_work_units
SET
    status = 'claimed',
    worker_id = $1,              -- UUID of this worker
    claimed_at = NOW(),
    heartbeat_at = NOW()
WHERE id = (
    SELECT id
    FROM ingestion_work_units
    WHERE job_id = $2
      AND status = 'pending'
      AND unit_type = 'parse_batch'
    ORDER BY batch_number
    LIMIT 1
    FOR UPDATE SKIP LOCKED      -- ← Prevents race conditions
)
RETURNING id, start_offset, end_offset;
```

**Operations per batch**:
1. Download raw file from S3 (if not cached locally)
2. Parse proteins [start_offset..end_offset]
3. Insert into `ingestion_parsed_proteins` (batch insert)
4. Update work unit status to `completed`
5. Update job `records_processed` counter

**Idempotent**: Skip proteins already in `ingestion_parsed_proteins`

**Parallel**: Multiple workers claim different batches simultaneously

**Resumable**: Failed batches go back to `pending` (with retry limit)

### Stage 3: Store (Parallel Workers)

```sql
-- Create storage work units
INSERT INTO ingestion_work_units (job_id, unit_type, batch_number, start_offset, end_offset, status)
SELECT
    $1,
    'store_batch',
    (row_number() OVER () - 1) / 100,  -- Batches of 100 proteins
    id,
    id,
    'pending'
FROM ingestion_parsed_proteins
WHERE job_id = $1 AND status = 'parsed';
```

**Operations per batch**:
1. For each protein in batch:
   - Generate DAT, FASTA, JSON content
   - Compute MD5 checksums
   - Upload to S3: `{org}/{accession}/{version}/{file}`
   - Track in `ingestion_file_uploads` with MD5
   - Create registry_entry, data_source, protein_metadata
   - Create version and version_files with MD5
   - Mark protein as `stored`
2. Batch commit transaction
3. Update work unit to `completed`

**Transaction Boundaries**:
```rust
// Batch of 100 proteins
let mut tx = pool.begin().await?;

for protein in batch {
    // 1. Upload to S3 (outside transaction)
    upload_protein_files(&protein).await?;

    // 2. Insert DB records (in transaction)
    insert_protein_metadata(&tx, &protein).await?;
    insert_version_files(&tx, &protein).await?;

    // 3. Mark as stored
    mark_protein_stored(&tx, protein.id).await?;
}

tx.commit().await?;  // ← Atomic batch commit
```

**Idempotent**: Check if protein already exists in `protein_metadata`

## Worker Heartbeat & Dead Worker Detection

**Heartbeat Update**:
```sql
-- Worker updates heartbeat every 30 seconds
UPDATE ingestion_work_units
SET heartbeat_at = NOW()
WHERE id = $1 AND worker_id = $2;
```

**Reclaim Dead Workers**:
```sql
-- Return stale work units to pending queue
UPDATE ingestion_work_units
SET
    status = 'pending',
    worker_id = NULL,
    claimed_at = NULL,
    retry_count = retry_count + 1
WHERE status = 'claimed'
  AND heartbeat_at < NOW() - INTERVAL '2 minutes'
  AND retry_count < max_retries;
```

## Parallel Worker Example

```rust
// Worker 1
let unit = claim_work_unit(worker_id, job_id).await?;
// Claims: batch 0 (proteins 0-999)

// Worker 2 (simultaneously)
let unit = claim_work_unit(worker_id, job_id).await?;
// Claims: batch 1 (proteins 1000-1999)

// Worker 3 (simultaneously)
let unit = claim_work_unit(worker_id, job_id).await?;
// Claims: batch 2 (proteins 2000-2999)

// All workers process in parallel!
```

## MD5 Checksum Flow

### 1. Download Verification
```rust
// From metalink
expected_md5 = "e3cd39d0c48231aa5abb3eca81b3c62a";

// After download
let mut hasher = md5::Md5::new();
hasher.update(&file_data);
let verified_md5 = format!("{:x}", hasher.finalize());

if expected_md5 != verified_md5 {
    return Err("MD5 mismatch - download corrupted");
}

// Store both in database
UPDATE ingestion_raw_files
SET
    expected_md5 = $1,
    verified_md5 = $2,
    verified_at = NOW(),
    status = 'verified'
WHERE id = $3;
```

### 2. File Upload Verification
```rust
// For each protein file
let fasta_content = protein.to_fasta();
let mut hasher = md5::Md5::new();
hasher.update(fasta_content.as_bytes());
let md5 = format!("{:x}", hasher.finalize());

// Upload to S3
s3.upload(&s3_key, fasta_content, content_type).await?;

// Store MD5 in database for audit
INSERT INTO ingestion_file_uploads (job_id, parsed_protein_id, format, s3_key, md5_checksum)
VALUES ($1, $2, 'fasta', $3, $4);

// Also store in version_files for main schema
INSERT INTO version_files (version_id, format, s3_key, checksum)
VALUES ($1, 'fasta', $2, $3);  -- checksum column stores MD5
```

### 3. Audit Trail
```sql
-- Query all MD5s for a job
SELECT
    ifu.format,
    ifu.s3_key,
    ifu.md5_checksum,
    ifu.uploaded_at,
    vf.checksum as version_file_md5
FROM ingestion_file_uploads ifu
LEFT JOIN version_files vf ON vf.s3_key = ifu.s3_key
WHERE ifu.job_id = $1;
```

## S3 Storage Structure

### Ingest Folder (Temporary, Archived)
```
ingest/
└── uniprot/
    └── 2025_06/
        ├── RELEASE.metalink                 (metadata with MD5s)
        ├── uniprot_sprot.dat.gz             (655MB raw download)
        ├── uniprot_sprot.dat.gz.md5         (checksum verification)
        └── .archived_at_2025-06-15          (marker file after completion)
```

**Lifecycle**:
1. Download phase: Create files
2. Parse phase: Read from here
3. Store phase complete: Archive (move to glacier/cheaper tier)
4. Retention: Keep for 90 days, then delete

### Final Storage (Permanent)
```
uniprot/                                     (organization slug)
├── p01234/                                  (data source slug - protein accession)
│   └── 1.0/                                 (internal version)
│       ├── p01234.dat                       (sequence data)
│       ├── p01234.fasta                     (FASTA format)
│       └── p01234.json                      (full metadata)
├── q6gzx4/
│   └── 1.0/
│       ├── q6gzx4.dat
│       ├── q6gzx4.fasta
│       └── q6gzx4.json
...
```

**All lowercase**: accessions, file names, paths

## Job Orchestration

### Main Coordinator
```rust
pub struct IngestionCoordinator {
    pool: PgPool,
    s3: Storage,
}

impl IngestionCoordinator {
    pub async fn run_job(&self, job_id: Uuid) -> Result<()> {
        loop {
            let job = self.get_job_status(job_id).await?;

            match job.status.as_str() {
                "pending" => {
                    self.start_download_phase(job_id).await?;
                }
                "download_verified" => {
                    self.create_parse_work_units(job_id).await?;
                    self.update_job_status(job_id, "parsing").await?;
                }
                "parsing" => {
                    if self.all_parse_units_complete(job_id).await? {
                        self.create_store_work_units(job_id).await?;
                        self.update_job_status(job_id, "storing").await?;
                    }
                }
                "storing" => {
                    if self.all_store_units_complete(job_id).await? {
                        self.finalize_job(job_id).await?;
                        self.update_job_status(job_id, "completed").await?;
                        break;
                    }
                }
                "completed" => break,
                "failed" => return Err(anyhow!("Job failed")),
                _ => {}
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        Ok(())
    }
}
```

### Worker Pool
```rust
pub struct IngestionWorker {
    worker_id: Uuid,
    pool: PgPool,
    s3: Storage,
}

impl IngestionWorker {
    pub async fn work_loop(&self, job_id: Uuid) -> Result<()> {
        loop {
            // Claim a work unit
            let unit = self.claim_work_unit(job_id).await?;

            if unit.is_none() {
                // No work available, wait and retry
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }

            let unit = unit.unwrap();

            // Start heartbeat task
            let heartbeat_handle = self.start_heartbeat(unit.id);

            // Process work unit
            let result = match unit.unit_type.as_str() {
                "parse_batch" => {
                    self.process_parse_batch(unit).await
                }
                "store_batch" => {
                    self.process_store_batch(unit).await
                }
                _ => Err(anyhow!("Unknown unit type"))
            };

            // Stop heartbeat
            heartbeat_handle.abort();

            // Mark complete or failed
            match result {
                Ok(_) => self.mark_unit_complete(unit.id).await?,
                Err(e) => self.mark_unit_failed(unit.id, &e.to_string()).await?,
            }
        }
    }

    async fn claim_work_unit(&self, job_id: Uuid) -> Result<Option<WorkUnit>> {
        let unit = sqlx::query_as!(
            WorkUnit,
            r#"
            UPDATE ingestion_work_units
            SET
                status = 'claimed',
                worker_id = $1,
                claimed_at = NOW(),
                heartbeat_at = NOW()
            WHERE id = (
                SELECT id
                FROM ingestion_work_units
                WHERE job_id = $2
                  AND status = 'pending'
                ORDER BY batch_number
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING id, unit_type, start_offset, end_offset
            "#,
            self.worker_id,
            job_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(unit)
    }
}
```

## Configuration

### Batch Sizes
```rust
const PARSE_BATCH_SIZE: usize = 1000;  // Proteins per parse batch
const STORE_BATCH_SIZE: usize = 100;   // Proteins per store batch
const MAX_RETRIES: i32 = 3;            // Max retries per work unit
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const WORKER_TIMEOUT: Duration = Duration::from_secs(120);
```

### Worker Scaling
```bash
# Run 10 workers in parallel
for i in {1..10}; do
    cargo run --bin bdp-worker -- \
        --job-id $JOB_ID \
        --worker-type uniprot-parser &
done
```

## Monitoring Queries

### Job Progress
```sql
SELECT
    j.external_version,
    j.status,
    j.total_records,
    j.records_processed,
    j.records_stored,
    COUNT(DISTINCT wu.id) as total_work_units,
    COUNT(DISTINCT wu.id) FILTER (WHERE wu.status = 'completed') as completed_units,
    COUNT(DISTINCT wu.id) FILTER (WHERE wu.status = 'pending') as pending_units,
    COUNT(DISTINCT wu.id) FILTER (WHERE wu.status = 'claimed') as active_units
FROM ingestion_jobs j
LEFT JOIN ingestion_work_units wu ON wu.job_id = j.id
WHERE j.id = $1
GROUP BY j.id;
```

### Worker Status
```sql
SELECT
    worker_id,
    COUNT(*) as units_claimed,
    MAX(heartbeat_at) as last_heartbeat,
    NOW() - MAX(heartbeat_at) as time_since_heartbeat
FROM ingestion_work_units
WHERE status = 'claimed'
  AND job_id = $1
GROUP BY worker_id;
```

### Failed Units
```sql
SELECT
    batch_number,
    retry_count,
    error_message,
    updated_at
FROM ingestion_work_units
WHERE job_id = $1
  AND status = 'failed'
ORDER BY batch_number;
```

## Benefits

1. **Parallel Processing**: Multiple workers process different batches simultaneously
2. **Resumable**: Crash at any point, resume from last checkpoint
3. **Idempotent**: Safe to retry any operation
4. **Distributed**: Workers can run on different machines
5. **Observable**: Full visibility into job progress via database
6. **Auditable**: All MD5 checksums stored for verification
7. **Efficient**: Archive raw files, don't re-download
8. **Scalable**: Add more workers to speed up processing

## Next Steps

1. Implement `IngestionCoordinator` struct
2. Implement `IngestionWorker` struct
3. Add MD5 verification at each stage
4. Implement S3 archival for ingest/ folder
5. Create CLI commands for job management
6. Add Prometheus metrics for monitoring
