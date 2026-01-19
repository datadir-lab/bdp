-- Add 'ingestion_job' resource type to audit_log resource_type_check constraint
--
-- This allows auditing of ingestion job operations

-- Drop the existing constraint
ALTER TABLE audit_log DROP CONSTRAINT IF EXISTS resource_type_check;

-- Re-create with 'ingestion_job' added
ALTER TABLE audit_log
ADD CONSTRAINT resource_type_check CHECK (resource_type IN (
    'organization', 'data_source', 'version', 'tool',
    'registry_entry', 'version_file', 'dependency',
    'organism', 'protein_metadata', 'citation',
    'tag', 'download', 'version_mapping',
    'user', 'session', 'api_key',
    'ingestion_job',  -- NEW: Added for ingestion job auditing
    'other'
));
