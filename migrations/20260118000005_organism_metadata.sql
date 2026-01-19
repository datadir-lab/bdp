-- Organism Metadata Table
-- Organisms as data sources (not just FK)

CREATE TABLE organism_metadata (
    data_source_id UUID PRIMARY KEY REFERENCES data_sources(id) ON DELETE CASCADE,
    taxonomy_id INTEGER UNIQUE NOT NULL,        -- NCBI Taxonomy: 9606
    scientific_name VARCHAR(255) NOT NULL,      -- "Homo sapiens"
    common_name VARCHAR(255),                   -- "Human"
    rank VARCHAR(50),                           -- "species", "genus", etc.
    lineage TEXT,                               -- Full taxonomic lineage
    ncbi_tax_version VARCHAR(50),               -- NCBI Taxonomy version used
    genome_assembly_id UUID REFERENCES data_sources(id)  -- Optional: link to reference genome
);

-- Indexes
CREATE INDEX organism_metadata_taxonomy_idx ON organism_metadata(taxonomy_id);
CREATE INDEX organism_metadata_scientific_name_idx ON organism_metadata(scientific_name);
CREATE INDEX organism_metadata_common_name_idx ON organism_metadata(common_name);

-- Full-text search on organism names
CREATE INDEX organism_metadata_search_idx ON organism_metadata
    USING GIN (to_tsvector('english',
        scientific_name || ' ' ||
        COALESCE(common_name, '')
    ));

-- Update protein_metadata to reference organism data_source
ALTER TABLE protein_metadata
ADD COLUMN organism_id UUID REFERENCES data_sources(id),
ADD COLUMN uniprot_version VARCHAR(50);  -- External version (e.g., "2025_01")

-- Index for organism lookups
CREATE INDEX protein_metadata_organism_idx ON protein_metadata(organism_id);

-- Update data_sources.source_type constraint
ALTER TABLE data_sources DROP CONSTRAINT IF EXISTS source_type_check;

ALTER TABLE data_sources
ADD CONSTRAINT source_type_check CHECK (source_type IN (
    'protein',
    'genome',
    'organism',
    'bundle',
    'transcript',
    'annotation',
    'structure',
    'pathway',
    'other'
));

-- Comments
COMMENT ON TABLE organism_metadata IS 'Organisms as data sources for proper versioning';
COMMENT ON COLUMN organism_metadata.taxonomy_id IS 'NCBI Taxonomy identifier';
COMMENT ON COLUMN organism_metadata.ncbi_tax_version IS 'NCBI Taxonomy database version';
COMMENT ON COLUMN protein_metadata.organism_id IS 'Reference to organism data_source';
COMMENT ON COLUMN protein_metadata.uniprot_version IS 'External UniProt version (e.g., 2025_01)';
