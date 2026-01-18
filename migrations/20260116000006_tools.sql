-- Tools
-- Inherits from registry_entries. Represents bioinformatics software/packages.

CREATE TABLE tools (
    id UUID PRIMARY KEY REFERENCES registry_entries(id) ON DELETE CASCADE,
    tool_type VARCHAR(50),  -- 'alignment', 'assembly', 'variant_calling', 'visualization'
    repository_url TEXT,
    homepage_url TEXT,
    license VARCHAR(100),
    additional_metadata JSONB
);

-- Indexes
CREATE INDEX tools_type_idx ON tools(tool_type);
CREATE INDEX tools_metadata_idx ON tools USING GIN (additional_metadata);
