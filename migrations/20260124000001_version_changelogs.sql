-- Version changelogs table
-- Stores structured changelog information for each version bump,
-- tracking what changed and why (new releases, dependency cascades, etc.)

-- Create an enum type for changelog entry types
CREATE TYPE changelog_entry_type AS ENUM (
    'added',      -- New entries added
    'removed',    -- Entries removed/deprecated
    'modified',   -- Existing entries changed
    'schema',     -- Schema/format changes
    'dependency'  -- Dependency version updated
);

-- Create an enum type for bump types
CREATE TYPE version_bump_type AS ENUM (
    'major',  -- Breaking changes, significant updates
    'minor'   -- Non-breaking additions or updates
);

-- Create an enum type for change triggers
CREATE TYPE changelog_trigger_type AS ENUM (
    'upstream_dependency',  -- Triggered by upstream dependency update
    'new_release',          -- New upstream release available
    'manual'                -- Manually triggered update
);

-- Version changelogs table
CREATE TABLE version_changelogs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,
    bump_type version_bump_type NOT NULL,

    -- Structured changelog entries
    -- Example structure:
    -- [
    --   {"type": "added", "category": "proteins", "count": 1523, "description": "New proteins from SwissProt release"},
    --   {"type": "removed", "category": "proteins", "count": 42, "description": "Deprecated entries"},
    --   {"type": "modified", "category": "sequences", "count": 156, "description": "Sequence corrections"},
    --   {"type": "modified", "category": "annotations", "count": 8934, "description": "Updated GO annotations"}
    -- ]
    entries JSONB NOT NULL DEFAULT '[]',

    -- Summary statistics
    -- Example:
    -- {
    --   "total_entries_before": 568000,
    --   "total_entries_after": 569523,
    --   "entries_added": 1523,
    --   "entries_removed": 42,
    --   "entries_modified": 9090,
    --   "triggered_by": "upstream_dependency"
    -- }
    summary JSONB NOT NULL DEFAULT '{}',

    -- Human-readable summary text
    summary_text TEXT,

    -- For dependency cascades - references the version that triggered this update
    triggered_by_version_id UUID REFERENCES versions(id) ON DELETE SET NULL,

    -- Trigger type for categorization
    triggered_by changelog_trigger_type,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for looking up changelogs by version
CREATE INDEX idx_version_changelogs_version_id ON version_changelogs(version_id);

-- Index for finding cascaded changes (only index non-null values)
CREATE INDEX idx_version_changelogs_triggered_by ON version_changelogs(triggered_by_version_id)
    WHERE triggered_by_version_id IS NOT NULL;

-- Index for querying by trigger type
CREATE INDEX idx_version_changelogs_trigger_type ON version_changelogs(triggered_by);

-- Index for time-based queries (finding recent changelogs)
CREATE INDEX idx_version_changelogs_created_at ON version_changelogs(created_at DESC);

-- GIN index for searching within entries JSONB
CREATE INDEX idx_version_changelogs_entries ON version_changelogs USING GIN (entries);

-- GIN index for searching within summary JSONB
CREATE INDEX idx_version_changelogs_summary ON version_changelogs USING GIN (summary);

-- Add helpful comments
COMMENT ON TABLE version_changelogs IS 'Stores structured changelog information for version bumps';
COMMENT ON COLUMN version_changelogs.version_id IS 'The version this changelog belongs to';
COMMENT ON COLUMN version_changelogs.bump_type IS 'Whether this was a major or minor version bump';
COMMENT ON COLUMN version_changelogs.entries IS 'Structured list of changes with type, category, count, and description';
COMMENT ON COLUMN version_changelogs.summary IS 'Statistical summary of changes (counts before/after, etc.)';
COMMENT ON COLUMN version_changelogs.summary_text IS 'Human-readable changelog summary';
COMMENT ON COLUMN version_changelogs.triggered_by_version_id IS 'Version that triggered this update (for dependency cascades)';
COMMENT ON COLUMN version_changelogs.triggered_by IS 'What triggered this version update';

COMMENT ON TYPE changelog_entry_type IS 'Types of changelog entries: added, removed, modified, schema, dependency';
COMMENT ON TYPE version_bump_type IS 'Version bump types: major (breaking), minor (non-breaking)';
COMMENT ON TYPE changelog_trigger_type IS 'What triggered a version change: upstream_dependency, new_release, manual';
