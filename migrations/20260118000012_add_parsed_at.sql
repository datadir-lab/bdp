-- Add parsed_at column to ingestion_staged_records
ALTER TABLE ingestion_staged_records
ADD COLUMN IF NOT EXISTS parsed_at TIMESTAMPTZ DEFAULT NOW();

CREATE INDEX IF NOT EXISTS ingestion_staged_records_parsed_at_idx ON ingestion_staged_records(parsed_at);
