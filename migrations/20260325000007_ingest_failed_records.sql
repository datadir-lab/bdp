-- migrations/20260325000007_ingest_failed_records.sql
-- Dead letter queue for failed ingestion records.
-- Stores raw data and error messages for records that failed processing,
-- enabling retry and manual inspection.

CREATE TABLE ingest_failed_records (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline      TEXT NOT NULL,
    batch_id      UUID,
    raw_data      BYTEA NOT NULL,
    error_msg     TEXT NOT NULL,
    attempt_count SMALLINT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ingest_failed_pipeline ON ingest_failed_records(pipeline, created_at);
