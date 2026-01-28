-- Additional indexes to optimize search performance on base tables
-- These improve both direct queries and materialized view refresh performance

-- Composite index for version_files to speed up format filtering
-- This helps the EXISTS subquery in format filters
CREATE INDEX IF NOT EXISTS idx_version_files_version_format
    ON version_files (version_id, format);

-- Index on versions for aggregation queries (used in MV and search)
CREATE INDEX IF NOT EXISTS idx_versions_entry_downloads
    ON versions (entry_id, download_count);

-- Indexes for taxonomy_metadata to optimize organism filtering
CREATE INDEX IF NOT EXISTS idx_taxonomy_metadata_scientific_name
    ON taxonomy_metadata (scientific_name)
    WHERE scientific_name IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_taxonomy_metadata_common_name
    ON taxonomy_metadata (common_name)
    WHERE common_name IS NOT NULL;

-- Pattern matching indexes for case-insensitive ILIKE on taxonomy
CREATE INDEX IF NOT EXISTS idx_taxonomy_metadata_scientific_name_lower
    ON taxonomy_metadata (LOWER(scientific_name) text_pattern_ops)
    WHERE scientific_name IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_taxonomy_metadata_common_name_lower
    ON taxonomy_metadata (LOWER(common_name) text_pattern_ops)
    WHERE common_name IS NOT NULL;

-- Composite index for data_sources to optimize joins
CREATE INDEX IF NOT EXISTS idx_data_sources_source_type
    ON data_sources (source_type)
    WHERE source_type IS NOT NULL;

-- Index for protein_metadata joins
CREATE INDEX IF NOT EXISTS idx_protein_metadata_taxonomy
    ON protein_metadata (taxonomy_id)
    WHERE taxonomy_id IS NOT NULL;

-- Composite index for registry_entries filtering
CREATE INDEX IF NOT EXISTS idx_registry_entries_type_org
    ON registry_entries (entry_type, organization_id);

-- Index to speed up slug lookups (ensure both slugs exist for search results)
CREATE INDEX IF NOT EXISTS idx_registry_entries_slug_not_null
    ON registry_entries (slug)
    WHERE slug IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_organizations_slug_not_null
    ON organizations (slug)
    WHERE slug IS NOT NULL;

-- Add comments
COMMENT ON INDEX idx_version_files_version_format IS 'Speeds up format filtering in search queries';
COMMENT ON INDEX idx_versions_entry_downloads IS 'Optimizes download count aggregations';
COMMENT ON INDEX idx_taxonomy_metadata_scientific_name IS 'Optimizes organism filtering by scientific name';
COMMENT ON INDEX idx_taxonomy_metadata_common_name IS 'Optimizes organism filtering by common name';
COMMENT ON INDEX idx_taxonomy_metadata_scientific_name_lower IS 'Supports case-insensitive ILIKE on scientific names';
COMMENT ON INDEX idx_taxonomy_metadata_common_name_lower IS 'Supports case-insensitive ILIKE on common names';
COMMENT ON INDEX idx_data_sources_source_type IS 'Optimizes source type filtering';
COMMENT ON INDEX idx_protein_metadata_taxonomy IS 'Speeds up protein-to-taxonomy joins';
COMMENT ON INDEX idx_registry_entries_type_org IS 'Composite index for type and organization filtering';
