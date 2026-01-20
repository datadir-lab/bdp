-- Remove organism_id from data_sources table
--
-- The organism_id column in data_sources was incorrectly added.
-- Organism relationships should only exist in metadata tables:
-- - protein_metadata.organism_id -> references data_sources (where source_type='organism')
-- - organism_metadata has taxonomy info directly
--
-- This migration removes the incorrect organism_id from the base data_sources table.

-- 1. Drop the foreign key constraint
ALTER TABLE data_sources
DROP CONSTRAINT IF EXISTS data_sources_organism_id_fkey;

-- 2. Drop the index
DROP INDEX IF EXISTS data_sources_organism_idx;

-- 3. Drop the column
ALTER TABLE data_sources
DROP COLUMN IF EXISTS organism_id;
