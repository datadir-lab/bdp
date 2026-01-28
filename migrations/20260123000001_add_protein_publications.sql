-- Add publications and entry history to protein metadata
-- Phase 4: Complete protein entry lifecycle tracking

-- Add entry history dates to protein_metadata
ALTER TABLE protein_metadata
ADD COLUMN entry_created DATE,
ADD COLUMN sequence_updated DATE,
ADD COLUMN annotation_updated DATE;

-- Create protein_publications table for references
CREATE TABLE protein_publications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protein_id UUID NOT NULL REFERENCES protein_metadata(data_source_id) ON DELETE CASCADE,
    reference_number INT NOT NULL,
    position TEXT,
    comments TEXT[] DEFAULT '{}',
    pubmed_id VARCHAR(50),
    doi VARCHAR(255),
    author_group TEXT,
    authors TEXT[] DEFAULT '{}',
    title TEXT,
    location TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    -- Ensure reference numbers are unique per protein
    CONSTRAINT protein_publications_unique_ref UNIQUE (protein_id, reference_number)
);

-- Create indexes for protein_publications
CREATE INDEX protein_publications_protein_id_idx ON protein_publications(protein_id);
CREATE INDEX protein_publications_pubmed_id_idx ON protein_publications(pubmed_id) WHERE pubmed_id IS NOT NULL;
CREATE INDEX protein_publications_doi_idx ON protein_publications(doi) WHERE doi IS NOT NULL;
CREATE INDEX protein_publications_ref_number_idx ON protein_publications(reference_number);

-- Add index for entry history date queries
CREATE INDEX protein_metadata_entry_created_idx ON protein_metadata(entry_created) WHERE entry_created IS NOT NULL;
CREATE INDEX protein_metadata_sequence_updated_idx ON protein_metadata(sequence_updated) WHERE sequence_updated IS NOT NULL;
CREATE INDEX protein_metadata_annotation_updated_idx ON protein_metadata(annotation_updated) WHERE annotation_updated IS NOT NULL;

-- Comments
COMMENT ON TABLE protein_publications IS 'Scientific publications and references supporting protein annotations';
COMMENT ON COLUMN protein_publications.reference_number IS 'Sequential reference number from UniProt entry (RN line)';
COMMENT ON COLUMN protein_publications.position IS 'Which part of the protein this reference supports (RP line)';
COMMENT ON COLUMN protein_publications.comments IS 'Context like tissue, strain, or conditions (RC line)';
COMMENT ON COLUMN protein_publications.pubmed_id IS 'PubMed identifier for the article (RX line)';
COMMENT ON COLUMN protein_publications.doi IS 'Digital Object Identifier for the article (RX line)';
COMMENT ON COLUMN protein_publications.author_group IS 'Author consortium or group name (RG line)';
COMMENT ON COLUMN protein_publications.authors IS 'List of article authors (RA line)';
COMMENT ON COLUMN protein_publications.title IS 'Article title (RT line)';
COMMENT ON COLUMN protein_publications.location IS 'Journal, volume, pages, year (RL line)';

COMMENT ON COLUMN protein_metadata.entry_created IS 'Date when the protein entry was first integrated into the database';
COMMENT ON COLUMN protein_metadata.sequence_updated IS 'Date of the last sequence update';
COMMENT ON COLUMN protein_metadata.annotation_updated IS 'Date of the last annotation update';
