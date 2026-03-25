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

-- 6. Drop the materialized view that depends on data_sources.source_type
DROP MATERIALIZED VIEW IF EXISTS search_registry_entries_mv;

-- 7. Drop the CHECK-constrained column and rename FK column
ALTER TABLE data_sources DROP COLUMN source_type;
ALTER TABLE data_sources RENAME COLUMN source_type_fk TO source_type;

-- 8. Recreate the materialized view (now referencing the new FK-backed source_type column)
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
