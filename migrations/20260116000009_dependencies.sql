-- Dependencies
-- Links between versions - any version can depend on others.

CREATE TABLE dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,
    depends_on_entry_id UUID NOT NULL REFERENCES registry_entries(id) ON DELETE CASCADE,
    depends_on_version VARCHAR(64) NOT NULL,
    dependency_type VARCHAR(50) DEFAULT 'required',  -- 'required', 'optional'
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(version_id, depends_on_entry_id)
);

-- Indexes
CREATE INDEX dependencies_version_id_idx ON dependencies(version_id);
CREATE INDEX dependencies_depends_on_entry_idx ON dependencies(depends_on_entry_id);
CREATE INDEX dependencies_depends_on_version_idx ON dependencies(depends_on_version);
