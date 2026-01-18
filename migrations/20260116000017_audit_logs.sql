-- Create audit_log table for CQRS audit trail
-- Tracks all commands executed in the system (not queries)

CREATE TABLE IF NOT EXISTS audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id UUID,
    changes JSONB,
    ip_address TEXT,
    user_agent TEXT,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB
);

-- Index for querying by timestamp
CREATE INDEX idx_audit_log_timestamp ON audit_log(timestamp DESC);

-- Index for querying by resource
CREATE INDEX idx_audit_log_resource ON audit_log(resource_type, resource_id);

-- Index for querying by user
CREATE INDEX idx_audit_log_user ON audit_log(user_id, timestamp DESC);

-- Index for querying by action
CREATE INDEX idx_audit_log_action ON audit_log(action, timestamp DESC);

-- Add comments
COMMENT ON TABLE audit_log IS 'Audit trail for all commands (CQRS write operations)';
COMMENT ON COLUMN audit_log.action IS 'Command action (e.g., create, update, delete)';
COMMENT ON COLUMN audit_log.resource_type IS 'Type of resource affected (e.g., organization, data_source)';
COMMENT ON COLUMN audit_log.resource_id IS 'ID of the affected resource';
COMMENT ON COLUMN audit_log.changes IS 'JSON representation of changes made';
COMMENT ON COLUMN audit_log.metadata IS 'Additional metadata about the command';
COMMENT ON COLUMN audit_log.ip_address IS 'Client IP address';
COMMENT ON COLUMN audit_log.user_agent IS 'Client user agent string';
