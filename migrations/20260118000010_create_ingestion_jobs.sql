-- Ingestion Jobs Framework
-- Tracks ETL jobs with checkpoint support, parallel workers, and idempotent operations

-- Main job tracker
CREATE TABLE IF NOT EXISTS ingestion_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id),

    -- Job identification
    job_type VARCHAR(50) NOT NULL,           -- 'uniprot_swissprot', 'uniprot_trembl', etc.
    external_version VARCHAR(255) NOT NULL,  -- '2025_06' from UniProt
    internal_version VARCHAR(255) NOT NULL,  -- '1.0' our versioning

    -- Status tracking
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    -- 'pending' → 'downloading' → 'download_verified' → 'parsing' → 'storing' → 'completed' | 'failed'

    -- Timestamps
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    -- Metadata
    total_records BIGINT,                    -- Expected total (from metalink)
    records_processed BIGINT DEFAULT 0,
    records_stored BIGINT DEFAULT 0,
    records_failed BIGINT DEFAULT 0,
    metadata JSONB,                          -- Flexible storage for job-specific data

    UNIQUE(organization_id, job_type, external_version)
);

CREATE INDEX IF NOT EXISTS idx_ingestion_jobs_status ON ingestion_jobs(status);
CREATE INDEX IF NOT EXISTS idx_ingestion_jobs_org ON ingestion_jobs(organization_id);

-- Raw file downloads (ingest/ folder tracking)
CREATE TABLE IF NOT EXISTS ingestion_raw_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES ingestion_jobs(id) ON DELETE CASCADE,

    -- File identification
    file_type VARCHAR(50) NOT NULL,          -- 'dat', 'fasta', 'xml', 'metalink'
    s3_key TEXT NOT NULL,                    -- 'ingest/uniprot/2025_06/uniprot_sprot.dat.gz'

    -- Checksums for verification
    expected_md5 VARCHAR(32),                -- From metalink
    verified_md5 VARCHAR(32),                -- After download verification

    -- File metadata
    size_bytes BIGINT,
    compression VARCHAR(20),                 -- 'gzip', 'none'

    -- Status
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    -- 'pending' → 'downloading' → 'downloaded' → 'verifying' → 'verified' | 'failed'

    downloaded_at TIMESTAMPTZ,
    verified_at TIMESTAMPTZ,
    error_message TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(job_id, file_type)
);

CREATE INDEX IF NOT EXISTS idx_ingestion_raw_files_job ON ingestion_raw_files(job_id);
CREATE INDEX IF NOT EXISTS idx_ingestion_raw_files_status ON ingestion_raw_files(status);

-- Work units for parallel processing
CREATE TABLE IF NOT EXISTS ingestion_work_units (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES ingestion_jobs(id) ON DELETE CASCADE,

    -- Unit identification
    unit_type VARCHAR(50) NOT NULL,          -- 'parse_batch', 'store_batch'
    batch_number INTEGER NOT NULL,           -- Sequential batch number

    -- Batch range (for parallel processing)
    start_offset INTEGER NOT NULL,           -- Start at protein N
    end_offset INTEGER NOT NULL,             -- End at protein N

    -- Worker claim (for distributed processing)
    worker_id VARCHAR(255),                  -- UUID of worker that claimed this unit
    claimed_at TIMESTAMPTZ,
    heartbeat_at TIMESTAMPTZ,                -- Last heartbeat (for dead worker detection)

    -- Status
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    -- 'pending' → 'claimed' → 'processing' → 'completed' | 'failed'

    -- Retry logic
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,

    -- Completion tracking
    completed_at TIMESTAMPTZ,
    error_message TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(job_id, unit_type, batch_number)
);

CREATE INDEX IF NOT EXISTS idx_ingestion_work_units_job ON ingestion_work_units(job_id);
CREATE INDEX IF NOT EXISTS idx_ingestion_work_units_status ON ingestion_work_units(status);
CREATE INDEX IF NOT EXISTS idx_ingestion_work_units_pending ON ingestion_work_units(job_id, status)
    WHERE status = 'pending';

-- Parsed data staging (before committing to main tables)
CREATE TABLE IF NOT EXISTS ingestion_parsed_proteins (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES ingestion_jobs(id) ON DELETE CASCADE,
    work_unit_id UUID REFERENCES ingestion_work_units(id) ON DELETE SET NULL,

    -- Protein data (parsed from DAT)
    accession VARCHAR(255) NOT NULL,         -- 'p01234' (lowercase)
    entry_name VARCHAR(255) NOT NULL,        -- '001r_frg3g' (lowercase)
    protein_name TEXT NOT NULL,
    gene_name VARCHAR(255),
    organism_name VARCHAR(255) NOT NULL,
    taxonomy_id INTEGER NOT NULL,

    -- Sequence data
    sequence TEXT NOT NULL,
    sequence_length INTEGER NOT NULL,
    mass_da BIGINT NOT NULL,
    sequence_md5 VARCHAR(32) NOT NULL,       -- MD5 of sequence for deduplication

    -- Release metadata
    release_date DATE,

    -- Processing status
    parsed_at TIMESTAMPTZ DEFAULT NOW(),
    stored_at TIMESTAMPTZ,                   -- When committed to protein_metadata
    status VARCHAR(50) NOT NULL DEFAULT 'parsed',
    -- 'parsed' → 'uploading' → 'uploaded' → 'storing' → 'stored' | 'failed'

    error_message TEXT,

    UNIQUE(job_id, accession)
);

CREATE INDEX IF NOT EXISTS idx_ingestion_parsed_proteins_job ON ingestion_parsed_proteins(job_id);
CREATE INDEX IF NOT EXISTS idx_ingestion_parsed_proteins_status ON ingestion_parsed_proteins(status);
CREATE INDEX IF NOT EXISTS idx_ingestion_parsed_proteins_accession ON ingestion_parsed_proteins(accession);

-- File uploads tracking (for S3 uploads to final location)
CREATE TABLE IF NOT EXISTS ingestion_file_uploads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES ingestion_jobs(id) ON DELETE CASCADE,
    parsed_protein_id UUID REFERENCES ingestion_parsed_proteins(id) ON DELETE CASCADE,

    -- File identification
    format VARCHAR(50) NOT NULL,             -- 'dat', 'fasta', 'json'
    s3_key TEXT NOT NULL,                    -- 'uniprot/p01234/1.0/p01234.fasta'

    -- File metadata
    size_bytes BIGINT NOT NULL,
    md5_checksum VARCHAR(32) NOT NULL,       -- Store MD5 for audit
    content_type VARCHAR(100),

    -- Upload status
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    -- 'pending' → 'uploading' → 'uploaded' → 'verified' | 'failed'

    uploaded_at TIMESTAMPTZ,
    verified_at TIMESTAMPTZ,
    error_message TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(job_id, parsed_protein_id, format)
);

CREATE INDEX IF NOT EXISTS idx_ingestion_file_uploads_job ON ingestion_file_uploads(job_id);
CREATE INDEX IF NOT EXISTS idx_ingestion_file_uploads_status ON ingestion_file_uploads(status);

-- Add trigger for updated_at
CREATE OR REPLACE FUNCTION update_ingestion_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_ingestion_jobs_updated_at ON ingestion_jobs;
CREATE TRIGGER trigger_ingestion_jobs_updated_at
    BEFORE UPDATE ON ingestion_jobs
    FOR EACH ROW
    EXECUTE FUNCTION update_ingestion_updated_at();

DROP TRIGGER IF EXISTS trigger_ingestion_work_units_updated_at ON ingestion_work_units;
CREATE TRIGGER trigger_ingestion_work_units_updated_at
    BEFORE UPDATE ON ingestion_work_units
    FOR EACH ROW
    EXECUTE FUNCTION update_ingestion_updated_at();

-- Comments for documentation
COMMENT ON TABLE ingestion_jobs IS 'Tracks ETL ingestion jobs with status and metadata';
COMMENT ON TABLE ingestion_work_units IS 'Parallel work units for distributed processing with worker claims';
COMMENT ON TABLE ingestion_raw_files IS 'Tracks raw downloaded files in ingest/ folder with MD5 verification';
COMMENT ON TABLE ingestion_parsed_proteins IS 'Staging table for parsed proteins before committing to main tables';
COMMENT ON TABLE ingestion_file_uploads IS 'Tracks S3 uploads to final locations with MD5 checksums';
