-- Protein Sequences Deduplication Table
-- Store sequences separately to avoid duplication across versions

CREATE TABLE protein_sequences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sequence TEXT NOT NULL,
    sequence_hash VARCHAR(64) UNIQUE NOT NULL,  -- SHA256 of sequence
    sequence_length INTEGER NOT NULL,
    sequence_md5 VARCHAR(32) NOT NULL,          -- MD5 for compatibility
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Unique index on hash for fast deduplication
CREATE UNIQUE INDEX protein_sequences_hash_idx ON protein_sequences(sequence_hash);

-- Index for length-based queries
CREATE INDEX protein_sequences_length_idx ON protein_sequences(sequence_length);

-- Trigram index for sequence substring search (motif search)
CREATE INDEX protein_sequences_trigram_idx ON protein_sequences USING GIN (sequence gin_trgm_ops);

-- Update protein_metadata to reference sequences
ALTER TABLE protein_metadata
ADD COLUMN sequence_id UUID REFERENCES protein_sequences(id);

-- Index for fast joins
CREATE INDEX protein_metadata_sequence_idx ON protein_metadata(sequence_id);

-- Comments
COMMENT ON TABLE protein_sequences IS 'Deduplicated protein sequences shared across versions';
COMMENT ON COLUMN protein_sequences.sequence_hash IS 'SHA256 hash of sequence for deduplication';
COMMENT ON COLUMN protein_sequences.sequence_md5 IS 'MD5 checksum for backward compatibility';
COMMENT ON COLUMN protein_metadata.sequence_id IS 'Reference to deduplicated sequence';
