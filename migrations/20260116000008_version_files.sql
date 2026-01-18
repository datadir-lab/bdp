-- Version Files
-- Multiple file formats per version (e.g., FASTA, XML, JSON for same protein).

CREATE TABLE version_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,
    format VARCHAR(50) NOT NULL,  -- 'fasta', 'xml', 'dat', 'json', 'tar.gz'
    s3_key TEXT NOT NULL,  -- S3 path: proteins/uniprot/P01308/1.0/P01308.fasta
    checksum VARCHAR(64) NOT NULL,  -- SHA-256
    size_bytes BIGINT NOT NULL,
    compression VARCHAR(20),  -- 'gzip', 'bzip2', 'none'
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(version_id, format)
);

-- Indexes
CREATE INDEX version_files_version_id_idx ON version_files(version_id);
CREATE INDEX version_files_format_idx ON version_files(format);
CREATE INDEX version_files_s3_key_idx ON version_files(s3_key);
