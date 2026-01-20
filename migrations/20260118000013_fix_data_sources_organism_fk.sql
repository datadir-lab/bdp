-- Fix data_sources.organism_id foreign key to reference data_sources instead of organisms table
--
-- The organism_id in data_sources should reference other data_sources (where source_type='organism'),
-- not the old organisms table which was removed in favor of organism_metadata

-- 1. Drop the old foreign key that references organisms table (idempotent)
ALTER TABLE data_sources
DROP CONSTRAINT IF EXISTS data_sources_organism_id_fkey;

-- 2. Add new foreign key that references data_sources(id) for organisms (idempotent)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'data_sources_organism_id_fkey'
        AND conrelid = 'data_sources'::regclass
    ) THEN
        ALTER TABLE data_sources
        ADD CONSTRAINT data_sources_organism_id_fkey
        FOREIGN KEY (organism_id) REFERENCES data_sources(id) ON DELETE SET NULL;
    END IF;
END $$;

-- 3. Add comment to clarify usage
COMMENT ON COLUMN data_sources.organism_id IS 'References another data_source where source_type=''organism'' (for proteins/genomes that belong to an organism)';
