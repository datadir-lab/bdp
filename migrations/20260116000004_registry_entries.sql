-- Registry Entries (Base Table)
-- Abstract base for all registry items. Every data source and tool is a registry entry.

CREATE TABLE registry_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
    slug VARCHAR(255) UNIQUE NOT NULL,  -- 'P01308', 'blast', 'swissprot-all'
    name VARCHAR(255) NOT NULL,
    description TEXT,
    entry_type VARCHAR(50) NOT NULL,  -- 'data_source' or 'tool'
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT entry_type_check CHECK (entry_type IN ('data_source', 'tool'))
);

-- Indexes
CREATE INDEX registry_entries_org_idx ON registry_entries(organization_id);
CREATE INDEX registry_entries_type_idx ON registry_entries(entry_type);
CREATE INDEX registry_entries_slug_idx ON registry_entries(slug);

-- Full-text search
CREATE INDEX registry_entries_search_idx ON registry_entries
    USING GIN (to_tsvector('english', name || ' ' || COALESCE(description, '')));
