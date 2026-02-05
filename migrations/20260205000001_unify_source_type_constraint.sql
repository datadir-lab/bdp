-- Unify data_sources source_type CHECK constraints
-- Two constraints exist due to naming mismatch between migrations:
--   source_type_check (from 20260120) - missing go_term, interpro_entry, genomic_sequence
--   check_source_type (from 20260121/20260128) - missing genome, transcript, annotation, structure, pathway, other
-- Drop both and create a single unified constraint with all valid source types.

ALTER TABLE data_sources DROP CONSTRAINT IF EXISTS source_type_check;
ALTER TABLE data_sources DROP CONSTRAINT IF EXISTS check_source_type;

ALTER TABLE data_sources ADD CONSTRAINT source_type_check CHECK (
    source_type IN (
        'protein',
        'genome',
        'genomic_sequence',
        'organism',
        'taxonomy',
        'go_term',
        'interpro_entry',
        'bundle',
        'transcript',
        'annotation',
        'structure',
        'pathway',
        'other'
    )
);
