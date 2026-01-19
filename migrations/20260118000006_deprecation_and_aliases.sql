-- Deprecation and Alias Support
-- Handle merged, deleted, and renamed data sources

-- Add deprecation fields to registry_entries
ALTER TABLE registry_entries
ADD COLUMN deprecated BOOLEAN DEFAULT FALSE,
ADD COLUMN deprecated_at TIMESTAMPTZ,
ADD COLUMN deprecated_reason TEXT,
ADD COLUMN superseded_by_id UUID REFERENCES registry_entries(id);

-- Index for filtering deprecated entries
CREATE INDEX registry_entries_deprecated_idx ON registry_entries(deprecated)
WHERE deprecated = FALSE;

-- Aliases table for accession changes
CREATE TABLE data_source_aliases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    alias VARCHAR(255) NOT NULL,
    alias_type VARCHAR(50) NOT NULL,  -- 'previous_accession', 'synonym', 'legacy'
    valid_from TIMESTAMPTZ,
    valid_until TIMESTAMPTZ,
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(alias, alias_type)
);

-- Indexes for fast alias lookups
CREATE INDEX aliases_lookup_idx ON data_source_aliases(alias);
CREATE INDEX aliases_data_source_idx ON data_source_aliases(data_source_id);
CREATE INDEX aliases_type_idx ON data_source_aliases(alias_type);

-- Constraint for alias_type values
ALTER TABLE data_source_aliases
ADD CONSTRAINT alias_type_check CHECK (alias_type IN (
    'previous_accession',
    'synonym',
    'legacy',
    'alternative_name'
));

-- Comments
COMMENT ON COLUMN registry_entries.deprecated IS 'Whether this entry is deprecated';
COMMENT ON COLUMN registry_entries.deprecated_reason IS 'Why this entry was deprecated';
COMMENT ON COLUMN registry_entries.superseded_by_id IS 'Replacement entry if deprecated';
COMMENT ON TABLE data_source_aliases IS 'Aliases for data sources (previous accessions, synonyms)';
COMMENT ON COLUMN data_source_aliases.alias IS 'Alternative identifier for the data source';
COMMENT ON COLUMN data_source_aliases.alias_type IS 'Type of alias (previous_accession, synonym, etc.)';
