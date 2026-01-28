-- Create materialized view for optimized search performance
-- This pre-computes all expensive aggregations and joins, eliminating N+1 query problems

CREATE MATERIALIZED VIEW search_registry_entries_mv AS
SELECT
    re.id,
    re.organization_id,
    o.slug as organization_slug,
    re.slug,
    re.name,
    re.description,
    re.entry_type,
    re.created_at,
    ds.source_type,
    ds.external_id,
    t.tool_type,
    -- Pre-compute organism info from taxonomy metadata
    COALESCE(org_ref.scientific_name, org_direct.scientific_name) as scientific_name,
    COALESCE(org_ref.common_name, org_direct.common_name) as common_name,
    COALESCE(org_ref.taxonomy_id, org_direct.taxonomy_id) as ncbi_taxonomy_id,
    -- Pre-compute latest version info using LATERAL JOIN (more efficient than scalar subquery)
    lv.version as latest_version,
    lv.external_version as external_version,
    -- Pre-compute available formats
    COALESCE(af.formats, ARRAY[]::VARCHAR[]) as available_formats,
    -- Pre-compute total downloads
    COALESCE(td.total, 0) as total_downloads,
    -- Pre-compute full-text search vector for faster ranking
    to_tsvector('english', re.name || ' ' || COALESCE(re.description, '')) as search_vector
FROM registry_entries re
JOIN organizations o ON o.id = re.organization_id
LEFT JOIN data_sources ds ON ds.id = re.id
LEFT JOIN tools t ON t.id = re.id
LEFT JOIN protein_metadata pm ON pm.data_source_id = ds.id
LEFT JOIN taxonomy_metadata org_ref ON org_ref.data_source_id = pm.taxonomy_id
LEFT JOIN taxonomy_metadata org_direct ON org_direct.data_source_id = ds.id AND ds.source_type = 'organism'
-- LATERAL JOINs are executed once per row and are more efficient than scalar subqueries
LEFT JOIN LATERAL (
    SELECT v.version, v.external_version
    FROM versions v
    WHERE v.entry_id = re.id
    ORDER BY v.published_at DESC
    LIMIT 1
) lv ON true
LEFT JOIN LATERAL (
    SELECT ARRAY_AGG(DISTINCT vf.format) as formats
    FROM versions v
    JOIN version_files vf ON vf.version_id = v.id
    WHERE v.entry_id = re.id
) af ON true
LEFT JOIN LATERAL (
    SELECT SUM(v.download_count)::bigint as total
    FROM versions v
    WHERE v.entry_id = re.id
) td ON true
WHERE re.slug IS NOT NULL AND o.slug IS NOT NULL;

-- Create indexes on the materialized view for fast searching and filtering
-- GIN index for full-text search - this is the most important index
CREATE INDEX idx_search_mv_search_vector ON search_registry_entries_mv USING GIN (search_vector);

-- B-tree indexes for filters
CREATE INDEX idx_search_mv_entry_type ON search_registry_entries_mv (entry_type);
CREATE INDEX idx_search_mv_source_type ON search_registry_entries_mv (source_type) WHERE source_type IS NOT NULL;
CREATE INDEX idx_search_mv_organization_id ON search_registry_entries_mv (organization_id);

-- Indexes for organism filtering (support both exact and partial matches)
CREATE INDEX idx_search_mv_scientific_name ON search_registry_entries_mv (scientific_name) WHERE scientific_name IS NOT NULL;
CREATE INDEX idx_search_mv_common_name ON search_registry_entries_mv (common_name) WHERE common_name IS NOT NULL;

-- Text pattern indexes for ILIKE queries on organism names
CREATE INDEX idx_search_mv_scientific_name_pattern ON search_registry_entries_mv (LOWER(scientific_name) text_pattern_ops) WHERE scientific_name IS NOT NULL;
CREATE INDEX idx_search_mv_common_name_pattern ON search_registry_entries_mv (LOWER(common_name) text_pattern_ops) WHERE common_name IS NOT NULL;

-- GIN index for array contains queries on formats
CREATE INDEX idx_search_mv_available_formats ON search_registry_entries_mv USING GIN (available_formats);

-- Composite index for sorting by rank and downloads
CREATE INDEX idx_search_mv_downloads_created ON search_registry_entries_mv (total_downloads DESC, created_at DESC);

-- Unique index for concurrent refresh capability
CREATE UNIQUE INDEX idx_search_mv_id ON search_registry_entries_mv (id);

-- Add comments for documentation
COMMENT ON MATERIALIZED VIEW search_registry_entries_mv IS 'Pre-computed search index with all aggregations for fast full-text search performance';
COMMENT ON INDEX idx_search_mv_search_vector IS 'Full-text search GIN index on pre-computed tsvector';
COMMENT ON INDEX idx_search_mv_entry_type IS 'Filter index for entry type (data_source, tool)';
COMMENT ON INDEX idx_search_mv_source_type IS 'Filter index for source type (protein, genome, etc.)';
COMMENT ON INDEX idx_search_mv_scientific_name_pattern IS 'Pattern matching index for ILIKE queries on scientific names';
COMMENT ON INDEX idx_search_mv_common_name_pattern IS 'Pattern matching index for ILIKE queries on common names';
COMMENT ON INDEX idx_search_mv_available_formats IS 'GIN index for format array filtering';
COMMENT ON INDEX idx_search_mv_id IS 'Unique index required for REFRESH MATERIALIZED VIEW CONCURRENTLY';
