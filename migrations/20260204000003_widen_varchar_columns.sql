-- Widen varchar columns that were too narrow for real UniProt data
-- Must drop/recreate materialized view that depends on registry_entries.name

-- Drop the materialized view and dependent functions
DROP MATERIALIZED VIEW IF EXISTS search_registry_entries_mv CASCADE;

-- Drop expression-based indexes on registry_entries.name that block ALTER TYPE
DROP INDEX IF EXISTS registry_entries_search_idx;
DROP INDEX IF EXISTS idx_registry_entries_name_trgm;

-- Widen columns
ALTER TABLE protein_features ALTER COLUMN feature_type TYPE varchar(100);
ALTER TABLE protein_cross_references ALTER COLUMN database TYPE varchar(100);
ALTER TABLE protein_signatures ALTER COLUMN database TYPE varchar(100);
ALTER TABLE protein_signatures ALTER COLUMN accession TYPE varchar(100);
ALTER TABLE protein_signatures ALTER COLUMN clan_accession TYPE varchar(100);
ALTER TABLE registry_entries ALTER COLUMN name TYPE varchar(500);

-- Recreate expression-based indexes on registry_entries.name
CREATE INDEX registry_entries_search_idx ON registry_entries
    USING GIN (to_tsvector('english', name || ' ' || COALESCE(description, '')));
CREATE INDEX idx_registry_entries_name_trgm
    ON registry_entries USING GIN (name gin_trgm_ops);

-- Recreate the materialized view
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
    COALESCE(org_ref.scientific_name, org_direct.scientific_name) as scientific_name,
    COALESCE(org_ref.common_name, org_direct.common_name) as common_name,
    COALESCE(org_ref.taxonomy_id, org_direct.taxonomy_id) as ncbi_taxonomy_id,
    lv.version as latest_version,
    lv.external_version as external_version,
    COALESCE(af.formats, ARRAY[]::VARCHAR[]) as available_formats,
    COALESCE(td.total, 0) as total_downloads,
    to_tsvector('english', re.name || ' ' || COALESCE(re.description, '')) as search_vector
FROM registry_entries re
JOIN organizations o ON o.id = re.organization_id
LEFT JOIN data_sources ds ON ds.id = re.id
LEFT JOIN tools t ON t.id = re.id
LEFT JOIN protein_metadata pm ON pm.data_source_id = ds.id
LEFT JOIN taxonomy_metadata org_ref ON org_ref.data_source_id = pm.taxonomy_id
LEFT JOIN taxonomy_metadata org_direct ON org_direct.data_source_id = ds.id AND ds.source_type = 'organism'
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

-- Recreate indexes
CREATE INDEX idx_search_mv_search_vector ON search_registry_entries_mv USING GIN (search_vector);
CREATE INDEX idx_search_mv_entry_type ON search_registry_entries_mv (entry_type);
CREATE INDEX idx_search_mv_source_type ON search_registry_entries_mv (source_type) WHERE source_type IS NOT NULL;
CREATE INDEX idx_search_mv_organization_id ON search_registry_entries_mv (organization_id);
CREATE INDEX idx_search_mv_scientific_name ON search_registry_entries_mv (scientific_name) WHERE scientific_name IS NOT NULL;
CREATE INDEX idx_search_mv_common_name ON search_registry_entries_mv (common_name) WHERE common_name IS NOT NULL;
CREATE INDEX idx_search_mv_scientific_name_pattern ON search_registry_entries_mv (LOWER(scientific_name) text_pattern_ops) WHERE scientific_name IS NOT NULL;
CREATE INDEX idx_search_mv_common_name_pattern ON search_registry_entries_mv (LOWER(common_name) text_pattern_ops) WHERE common_name IS NOT NULL;
CREATE INDEX idx_search_mv_available_formats ON search_registry_entries_mv USING GIN (available_formats);
CREATE INDEX idx_search_mv_downloads_created ON search_registry_entries_mv (total_downloads DESC, created_at DESC);
CREATE UNIQUE INDEX idx_search_mv_id ON search_registry_entries_mv (id);
