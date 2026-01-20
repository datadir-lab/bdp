-- Migration: Rename organism → taxonomy for clearer semantics
-- Organisms are biological entities, taxonomy is classification data

-- 1. Rename table
ALTER TABLE organism_metadata RENAME TO taxonomy_metadata;

-- 2. Rename FK column in protein_metadata for clarity
ALTER TABLE protein_metadata RENAME COLUMN organism_id TO taxonomy_id;

-- 3. Remove UNIQUE constraint on taxonomy_id (allow multiple versions)
ALTER TABLE taxonomy_metadata
    DROP CONSTRAINT IF EXISTS organism_metadata_taxonomy_id_key;

-- 4. Add 'taxonomy' to source_type enum
ALTER TABLE data_sources DROP CONSTRAINT IF EXISTS source_type_check;
ALTER TABLE data_sources ADD CONSTRAINT source_type_check CHECK (
    source_type IN (
        'protein',
        'genome',
        'organism',     -- Keep for backward compat
        'taxonomy',     -- NEW: NCBI taxonomy data
        'bundle',
        'transcript',
        'annotation',
        'structure',
        'pathway',
        'other'
    )
);

-- 5. Update comments
COMMENT ON TABLE taxonomy_metadata IS
    'NCBI Taxonomy classification data. Each row is a versioned snapshot of taxonomy info for a taxon.';

COMMENT ON COLUMN taxonomy_metadata.data_source_id IS
    'FK to data_sources (registry entry for this taxonomy version)';

COMMENT ON COLUMN taxonomy_metadata.taxonomy_id IS
    'NCBI Taxonomy identifier (e.g., 9606 for Homo sapiens)';

COMMENT ON COLUMN taxonomy_metadata.ncbi_tax_version IS
    'NCBI Taxonomy database version used (external version date)';

COMMENT ON COLUMN protein_metadata.taxonomy_id IS
    'FK to taxonomy data source (e.g., ncbi:9606@1.0)';

-- 6. Rename indexes
ALTER INDEX IF EXISTS organism_metadata_taxonomy_idx
    RENAME TO taxonomy_metadata_taxonomy_idx;

ALTER INDEX IF EXISTS organism_metadata_scientific_name_idx
    RENAME TO taxonomy_metadata_scientific_name_idx;

ALTER INDEX IF EXISTS organism_metadata_common_name_idx
    RENAME TO taxonomy_metadata_common_name_idx;

ALTER INDEX IF EXISTS organism_metadata_search_idx
    RENAME TO taxonomy_metadata_search_idx;
