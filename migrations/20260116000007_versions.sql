-- Versions
-- Version management for any registry entry (data sources or tools).

CREATE TABLE versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entry_id UUID NOT NULL REFERENCES registry_entries(id) ON DELETE CASCADE,
    version VARCHAR(64) NOT NULL,  -- Our opinionated: '1.0', '1.1', '2.0'
    external_version VARCHAR(64),  -- Original: '2025_01' (UniProt), '2.14.0' (BLAST)
    release_date DATE,
    size_bytes BIGINT,  -- Total size of all files
    download_count BIGINT DEFAULT 0,
    additional_metadata JSONB,
    dependency_cache JSONB,  -- Cached dependency list for performance
    dependency_count INT DEFAULT 0,
    published_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(entry_id, version)
);

-- Indexes
CREATE INDEX versions_entry_id_idx ON versions(entry_id);
CREATE INDEX versions_version_idx ON versions(version);
CREATE INDEX versions_release_date_idx ON versions(release_date);
CREATE INDEX versions_dependency_cache_idx ON versions USING GIN (dependency_cache);
