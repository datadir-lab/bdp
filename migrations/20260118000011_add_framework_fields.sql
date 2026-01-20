-- Add missing fields for ingestion framework compatibility

-- Add source_url and source_metadata to ingestion_jobs
ALTER TABLE ingestion_jobs
ADD COLUMN IF NOT EXISTS source_url TEXT,
ADD COLUMN IF NOT EXISTS source_metadata JSONB,
ADD COLUMN IF NOT EXISTS records_skipped BIGINT DEFAULT 0;

-- Add missing fields to ingestion_raw_files
ALTER TABLE ingestion_raw_files
ADD COLUMN IF NOT EXISTS file_purpose VARCHAR(50),
ADD COLUMN IF NOT EXISTS computed_md5 VARCHAR(32);

-- Add missing fields to ingestion_work_units
ALTER TABLE ingestion_work_units
ADD COLUMN IF NOT EXISTS started_processing_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS record_count INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS last_error TEXT;

-- Create ingestion_staged_records table (for parsed records before final storage)
CREATE TABLE IF NOT EXISTS ingestion_staged_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES ingestion_jobs(id) ON DELETE CASCADE,
    work_unit_id UUID REFERENCES ingestion_work_units(id) ON DELETE SET NULL,

    record_type VARCHAR(50) NOT NULL,
    record_identifier VARCHAR(255) NOT NULL,
    record_name VARCHAR(255),
    record_data JSONB NOT NULL,

    content_md5 VARCHAR(32),
    sequence_md5 VARCHAR(32),
    source_file VARCHAR(255),
    source_offset BIGINT,

    status VARCHAR(50) NOT NULL DEFAULT 'parsed',
    error_message TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    stored_at TIMESTAMPTZ,

    UNIQUE(job_id, record_identifier)
);

CREATE INDEX IF NOT EXISTS ingestion_staged_records_job_idx ON ingestion_staged_records(job_id);
CREATE INDEX IF NOT EXISTS ingestion_staged_records_status_idx ON ingestion_staged_records(status);
CREATE INDEX IF NOT EXISTS ingestion_staged_records_type_idx ON ingestion_staged_records(record_type);

COMMENT ON TABLE ingestion_staged_records IS 'Temporary staging for parsed records before committing to final tables';
