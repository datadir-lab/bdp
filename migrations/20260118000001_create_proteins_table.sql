-- Create proteins table for storing ingested protein data
-- This table stores protein sequences and metadata from various sources (UniProt, etc.)

CREATE TABLE IF NOT EXISTS proteins (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    accession VARCHAR(50) UNIQUE NOT NULL,  -- Primary accession (e.g. P12345)
    name TEXT NOT NULL,                      -- Protein name
    organism TEXT,                           -- Organism common name
    organism_scientific TEXT,                -- Organism scientific name
    taxonomy_id INTEGER,                     -- NCBI Taxonomy ID
    sequence TEXT NOT NULL,                  -- Amino acid sequence
    sequence_length INTEGER NOT NULL,        -- Length of sequence
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_proteins_accession ON proteins(accession);
CREATE INDEX IF NOT EXISTS idx_proteins_taxonomy_id ON proteins(taxonomy_id);
CREATE INDEX IF NOT EXISTS idx_proteins_organism ON proteins(organism);

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION trigger_proteins_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_proteins_updated_at
    BEFORE UPDATE ON proteins
    FOR EACH ROW
    EXECUTE FUNCTION trigger_proteins_updated_at();
