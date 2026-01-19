-- Add 'ingest' action to audit_log action_check constraint
--
-- This allows auditing of data ingestion operations (UniProt, etc.)

-- Drop the existing constraint
ALTER TABLE audit_log DROP CONSTRAINT IF EXISTS action_check;

-- Re-create with 'ingest' added
ALTER TABLE audit_log
ADD CONSTRAINT action_check CHECK (action IN (
    'create', 'update', 'delete', 'read',
    'login', 'logout', 'register',
    'publish', 'unpublish', 'archive',
    'upload', 'download',
    'grant', 'revoke',
    'ingest',  -- NEW: Added for data ingestion auditing
    'other'
));
