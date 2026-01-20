-- Create sequence tables for GenBank/RefSeq ingestion
-- Storage strategy: S3 for sequences, PostgreSQL for metadata

-- Sequence metadata (queryable)
CREATE TABLE sequence_metadata (
    data_source_id UUID PRIMARY KEY REFERENCES data_sources(id) ON DELETE CASCADE,
    accession VARCHAR(50) NOT NULL,
    accession_version VARCHAR(50) NOT NULL UNIQUE,
    sequence_length INTEGER NOT NULL,
    molecule_type VARCHAR(50) NOT NULL,
    topology VARCHAR(20), -- linear, circular
    definition TEXT NOT NULL,
    organism VARCHAR(255),
    taxonomy_id INTEGER REFERENCES taxonomy_metadata(taxonomy_id),
    gene_name VARCHAR(255),
    locus_tag VARCHAR(100),
    protein_id VARCHAR(50), -- For CDS features
    product TEXT, -- Protein product description
    features JSONB, -- All features: CDS, gene, regulatory, etc.
    gc_content DECIMAL(5,2),
    sequence_hash VARCHAR(64) NOT NULL,
    s3_key VARCHAR(500) NOT NULL, -- Path to FASTA file in S3
    source_database VARCHAR(20) NOT NULL CHECK (source_database IN ('genbank', 'refseq')),
    division VARCHAR(20), -- viral, bacterial, plant, mammalian, etc.
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for fast queries
CREATE INDEX idx_sequence_metadata_accession ON sequence_metadata(accession);
CREATE INDEX idx_sequence_metadata_taxonomy_id ON sequence_metadata(taxonomy_id);
CREATE INDEX idx_sequence_metadata_gene_name ON sequence_metadata(gene_name);
CREATE INDEX idx_sequence_metadata_source_database ON sequence_metadata(source_database);
CREATE INDEX idx_sequence_metadata_division ON sequence_metadata(division);
CREATE INDEX idx_sequence_metadata_hash ON sequence_metadata(sequence_hash);
CREATE INDEX idx_sequence_metadata_protein_id ON sequence_metadata(protein_id);

-- Sequence to protein mappings (central dogma linking: DNA -> Protein)
CREATE TABLE sequence_protein_mappings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sequence_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    protein_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    mapping_type VARCHAR(50) NOT NULL, -- 'cds', 'translation', 'db_xref'
    cds_start INTEGER,
    cds_end INTEGER,
    strand VARCHAR(1), -- '+' or '-'
    codon_start INTEGER,
    transl_table INTEGER,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(sequence_data_source_id, protein_data_source_id, mapping_type)
);

-- Indexes for mapping queries
CREATE INDEX idx_sequence_protein_seq ON sequence_protein_mappings(sequence_data_source_id);
CREATE INDEX idx_sequence_protein_prot ON sequence_protein_mappings(protein_data_source_id);
CREATE INDEX idx_sequence_protein_type ON sequence_protein_mappings(mapping_type);
