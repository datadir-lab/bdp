-- Drop all ingestion-related tables and indexes to start fresh

-- Drop tables in reverse dependency order
DROP TABLE IF EXISTS ingestion_file_uploads CASCADE;
DROP TABLE IF EXISTS ingestion_parsed_proteins CASCADE;
DROP TABLE IF EXISTS ingestion_staged_records CASCADE;
DROP TABLE IF EXISTS ingestion_work_units CASCADE;
DROP TABLE IF EXISTS ingestion_raw_files CASCADE;
DROP TABLE IF EXISTS ingestion_jobs CASCADE;

-- Drop functions
DROP FUNCTION IF EXISTS claim_work_unit(UUID, UUID, VARCHAR) CASCADE;
DROP FUNCTION IF EXISTS reclaim_stale_work_units(BIGINT) CASCADE;
DROP FUNCTION IF EXISTS update_ingestion_updated_at() CASCADE;
