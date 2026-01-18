-- Audit Log Table
-- Comprehensive audit logging for all system actions

CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID,  -- Nullable - anonymous actions allowed
    action VARCHAR(50) NOT NULL,  -- 'create', 'update', 'delete', 'login', 'logout', etc.
    resource_type VARCHAR(50) NOT NULL,  -- 'organization', 'data_source', 'version', 'tool', etc.
    resource_id UUID,  -- Nullable - some actions may not have a specific resource
    changes JSONB,  -- Before/after state for updates, or creation data
    ip_address VARCHAR(45),  -- IPv4 or IPv6 address
    user_agent TEXT,  -- Browser/client user agent
    timestamp TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    metadata JSONB,  -- Additional context (request_id, session_id, etc.)

    CONSTRAINT action_check CHECK (action IN (
        'create', 'update', 'delete', 'read',
        'login', 'logout', 'register',
        'publish', 'unpublish', 'archive',
        'upload', 'download',
        'grant', 'revoke',
        'other'
    )),
    CONSTRAINT resource_type_check CHECK (resource_type IN (
        'organization', 'data_source', 'version', 'tool',
        'registry_entry', 'version_file', 'dependency',
        'organism', 'protein_metadata', 'citation',
        'tag', 'download', 'version_mapping',
        'user', 'session', 'api_key',
        'other'
    ))
);

-- Indexes for efficient querying
CREATE INDEX audit_log_timestamp_idx ON audit_log(timestamp DESC);
CREATE INDEX audit_log_resource_type_idx ON audit_log(resource_type);
CREATE INDEX audit_log_resource_id_idx ON audit_log(resource_id);
CREATE INDEX audit_log_user_id_idx ON audit_log(user_id);
CREATE INDEX audit_log_action_idx ON audit_log(action);
CREATE INDEX audit_log_composite_idx ON audit_log(resource_type, resource_id, timestamp DESC);
CREATE INDEX audit_log_user_composite_idx ON audit_log(user_id, timestamp DESC);

-- GIN indexes for JSONB columns for efficient JSON queries
CREATE INDEX audit_log_changes_idx ON audit_log USING GIN (changes);
CREATE INDEX audit_log_metadata_idx ON audit_log USING GIN (metadata);

-- Comment on table
COMMENT ON TABLE audit_log IS 'Comprehensive audit trail for all system actions';
COMMENT ON COLUMN audit_log.user_id IS 'User who performed the action (null for anonymous)';
COMMENT ON COLUMN audit_log.action IS 'Type of action performed';
COMMENT ON COLUMN audit_log.resource_type IS 'Type of resource affected';
COMMENT ON COLUMN audit_log.resource_id IS 'ID of the specific resource affected';
COMMENT ON COLUMN audit_log.changes IS 'Before/after state or creation data in JSON format';
COMMENT ON COLUMN audit_log.ip_address IS 'Client IP address (IPv4 or IPv6)';
COMMENT ON COLUMN audit_log.user_agent IS 'Client user agent string';
COMMENT ON COLUMN audit_log.metadata IS 'Additional contextual information (request_id, session_id, etc.)';
