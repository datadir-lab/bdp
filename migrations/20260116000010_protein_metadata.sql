-- Protein Metadata
-- Protein-specific fields extending data_sources.

CREATE TABLE protein_metadata (
    data_source_id UUID PRIMARY KEY REFERENCES data_sources(id) ON DELETE CASCADE,
    accession VARCHAR(50) NOT NULL UNIQUE,  -- P01308
    entry_name VARCHAR(255),  -- INS_HUMAN
    protein_name TEXT,
    gene_name VARCHAR(255),
    sequence_length INT,
    mass_da BIGINT,  -- Molecular mass in Daltons
    sequence_checksum VARCHAR(64),  -- MD5 of amino acid sequence

    CONSTRAINT accession_format_check CHECK (accession ~ '^[A-Z0-9]+$')
);

-- Indexes
CREATE INDEX protein_metadata_accession_idx ON protein_metadata(accession);
CREATE INDEX protein_metadata_gene_name_idx ON protein_metadata(gene_name);

-- Full-text search
CREATE INDEX protein_metadata_search_idx ON protein_metadata
    USING GIN (to_tsvector('english',
        accession || ' ' ||
        COALESCE(entry_name, '') || ' ' ||
        COALESCE(protein_name, '') || ' ' ||
        COALESCE(gene_name, '')
    ));
