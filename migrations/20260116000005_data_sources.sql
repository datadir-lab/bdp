-- Data Sources
-- Inherits from registry_entries. Represents biological data (proteins, genomes, annotations).

CREATE TABLE data_sources (
    id UUID PRIMARY KEY REFERENCES registry_entries(id) ON DELETE CASCADE,
    source_type VARCHAR(50) NOT NULL,  -- 'protein', 'genome', 'annotation', 'structure'
    external_id VARCHAR(100),  -- UniProt accession: P01308, NCBI ID, etc.
    organism_id UUID REFERENCES organisms(id) ON DELETE SET NULL,
    additional_metadata JSONB,  -- Flexible metadata storage

    CONSTRAINT source_type_check CHECK (source_type IN ('protein', 'genome', 'annotation', 'structure', 'other'))
);

-- Indexes
CREATE INDEX data_sources_type_idx ON data_sources(source_type);
CREATE INDEX data_sources_organism_idx ON data_sources(organism_id);
CREATE INDEX data_sources_external_id_idx ON data_sources(external_id);
CREATE INDEX data_sources_metadata_idx ON data_sources USING GIN (additional_metadata);
