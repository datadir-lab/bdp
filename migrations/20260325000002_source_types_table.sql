-- migrations/20260325000002_source_types_table.sql
-- Replace source_type CHECK constraint on data_sources with a FK to source_types lookup table.
-- Adding a new pipeline thereafter requires only an INSERT, no DDL.

-- 1. Create lookup table
CREATE TABLE source_types (
    name        TEXT PRIMARY KEY,
    label       TEXT NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Seed all current + future types
INSERT INTO source_types (name, label, description) VALUES
    ('protein',          'Protein',          'UniProt protein sequences and annotations'),
    ('taxonomy',         'Taxon',            'NCBI taxonomy nodes'),
    ('organism',         'Organism',         'Organism entries'),
    ('genomic_sequence', 'Genomic Sequence', 'GenBank/RefSeq nucleotide sequences'),
    ('genome',           'Genome',           'Assembled genome entries'),
    ('go_term',          'GO Term',          'Gene Ontology terms'),
    ('interpro_entry',   'InterPro Entry',   'InterPro protein family/domain entries'),
    ('pathway',          'Pathway',          'Biological pathways (Reactome, KEGG)'),
    ('disease',          'Disease',          'Disease terms (MONDO)'),
    ('phenotype',        'Phenotype',        'Phenotype terms (HPO)'),
    ('compound',         'Compound',         'Chemical compounds (ChEBI, PubChem)'),
    ('drug',             'Drug',             'Drug entities (ChEMBL, DrugBank)'),
    ('variant',          'Variant',          'Genomic variants (ClinVar, dbSNP)'),
    ('structure',        'Structure',        'Protein structures (PDB, AlphaFold)'),
    ('gene',             'Gene',             'Gene entries (Ensembl)'),
    ('transcript',       'Transcript',       'Transcript entries (Ensembl)'),
    ('annotation',       'Annotation',       'Annotation entries'),
    ('bundle',           'Bundle',           'Aggregate data source bundle'),
    ('other',            'Other',            'Uncategorized source type');

-- 3. Add FK column alongside existing TEXT column
ALTER TABLE data_sources ADD COLUMN source_type_fk TEXT REFERENCES source_types(name);

-- 4. Populate FK from existing TEXT column (all existing values are in the seed above)
UPDATE data_sources SET source_type_fk = source_type;

-- 5. Make FK NOT NULL now that data is migrated
ALTER TABLE data_sources ALTER COLUMN source_type_fk SET NOT NULL;

-- 6. Drop the CHECK-constrained column and rename FK column
ALTER TABLE data_sources DROP COLUMN source_type;
ALTER TABLE data_sources RENAME COLUMN source_type_fk TO source_type;
