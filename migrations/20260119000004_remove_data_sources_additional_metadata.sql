-- Remove additional_metadata from data_sources table
--
-- The additional_metadata JSONB column in data_sources was a generic catch-all field.
-- Type-specific metadata should be stored in dedicated metadata tables:
-- - protein_metadata (for proteins)
-- - organism_metadata (for organisms)
-- - genome_metadata (for genomes)
-- - etc.
--
-- This enforces proper schema and makes queries more efficient.

-- 1. Drop the GIN index on additional_metadata
DROP INDEX IF EXISTS data_sources_metadata_idx;

-- 2. Drop the column
ALTER TABLE data_sources
DROP COLUMN IF EXISTS additional_metadata;

-- Add comments to clarify the design
COMMENT ON TABLE data_sources IS 'Base data source registry - contains only common fields. Type-specific metadata goes in *_metadata tables.';
COMMENT ON TABLE protein_metadata IS 'Protein-specific metadata for data sources where source_type=''protein''';
COMMENT ON TABLE organism_metadata IS 'Organism-specific metadata for data sources where source_type=''organism''';
