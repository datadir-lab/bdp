-- Semantic Versioning for Versions Table
-- Add MAJOR.MINOR.PATCH versioning support

ALTER TABLE versions
ADD COLUMN version_major INTEGER NOT NULL DEFAULT 1,
ADD COLUMN version_minor INTEGER NOT NULL DEFAULT 0,
ADD COLUMN version_patch INTEGER NOT NULL DEFAULT 0,
ADD COLUMN version_string VARCHAR(50) GENERATED ALWAYS AS (
    version_major || '.' || version_minor || '.' || version_patch
) STORED,
ADD COLUMN changelog TEXT,           -- Auto-generated changelog
ADD COLUMN release_notes TEXT;       -- Human-written summary
-- Note: external_version already exists

-- Indexes for version lookups (using entry_id, which is the same as data_source_id)
CREATE INDEX versions_semver_idx ON versions(
    entry_id,
    version_major DESC,
    version_minor DESC,
    version_patch DESC
);
CREATE INDEX versions_string_idx ON versions(entry_id, version_string);

-- Function to get latest version (using entry_id which is same as data_source_id)
CREATE OR REPLACE FUNCTION get_latest_version(p_entry_id UUID)
RETURNS TABLE(
    id UUID,
    version_string VARCHAR(50),
    version_major INTEGER,
    version_minor INTEGER,
    version_patch INTEGER,
    changelog TEXT,
    external_version VARCHAR(64)
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        v.id,
        v.version_string,
        v.version_major,
        v.version_minor,
        v.version_patch,
        v.changelog,
        v.external_version
    FROM versions v
    WHERE v.entry_id = p_entry_id
    ORDER BY v.version_major DESC, v.version_minor DESC, v.version_patch DESC
    LIMIT 1;
END;
$$ LANGUAGE plpgsql;

-- Comments
COMMENT ON COLUMN versions.version_major IS 'MAJOR version: Breaking changes (sequence change)';
COMMENT ON COLUMN versions.version_minor IS 'MINOR version: Non-breaking additions (annotations)';
COMMENT ON COLUMN versions.version_patch IS 'PATCH version: Bug fixes (typos, corrections)';
COMMENT ON COLUMN versions.changelog IS 'Auto-generated changelog documenting changes';
