-- Add retry tracking columns to ingestion_jobs
-- This allows failed jobs to be retried automatically

ALTER TABLE ingestion_jobs
ADD COLUMN IF NOT EXISTS retry_count INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS max_retries INTEGER DEFAULT 3,
ADD COLUMN IF NOT EXISTS last_error TEXT,
ADD COLUMN IF NOT EXISTS error_details JSONB;

-- Add index for finding retryable failed jobs
CREATE INDEX IF NOT EXISTS idx_ingestion_jobs_retryable
ON ingestion_jobs (status, retry_count, max_retries)
WHERE status = 'failed' AND retry_count < max_retries;

-- Add comment for documentation
COMMENT ON COLUMN ingestion_jobs.retry_count IS 'Number of times this job has been retried';
COMMENT ON COLUMN ingestion_jobs.max_retries IS 'Maximum number of retry attempts allowed';
COMMENT ON COLUMN ingestion_jobs.last_error IS 'Error message from the most recent failure';
COMMENT ON COLUMN ingestion_jobs.error_details IS 'Detailed error information including stack trace and context';
