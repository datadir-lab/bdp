-- Add extended metadata columns to protein_metadata
-- Phase 2: Complete UniProt DAT format support

-- Alternative names (AltName, SubName from DE lines)
ALTER TABLE protein_metadata
ADD COLUMN alternative_names TEXT[] DEFAULT '{}';

-- EC numbers (enzyme classification from DE lines)
ALTER TABLE protein_metadata
ADD COLUMN ec_numbers TEXT[] DEFAULT '{}';

-- Protein existence level (1-5 from PE line)
ALTER TABLE protein_metadata
ADD COLUMN protein_existence INT CHECK (protein_existence BETWEEN 1 AND 5);

-- Keywords (functional classification from KW lines)
ALTER TABLE protein_metadata
ADD COLUMN keywords TEXT[] DEFAULT '{}';

-- Organelle origin (mitochondrion, plastid, plasmid from OG line)
ALTER TABLE protein_metadata
ADD COLUMN organelle VARCHAR(100);

-- Organism hosts (for viruses, from OH lines)
ALTER TABLE protein_metadata
ADD COLUMN organism_hosts TEXT[] DEFAULT '{}';

-- Create separate tables for complex structured data

-- Protein features (domains, sites, modifications, variants from FT lines)
CREATE TABLE protein_features (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protein_id UUID NOT NULL REFERENCES protein_metadata(data_source_id) ON DELETE CASCADE,
    feature_type VARCHAR(50) NOT NULL,
    start_pos INT,
    end_pos INT,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX protein_features_protein_id_idx ON protein_features(protein_id);
CREATE INDEX protein_features_type_idx ON protein_features(feature_type);
CREATE INDEX protein_features_position_idx ON protein_features(start_pos, end_pos);

-- Database cross-references (PDB, GO, InterPro, KEGG, Pfam from DR lines)
CREATE TABLE protein_cross_references (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protein_id UUID NOT NULL REFERENCES protein_metadata(data_source_id) ON DELETE CASCADE,
    database VARCHAR(50) NOT NULL,
    database_id VARCHAR(255) NOT NULL,
    metadata JSONB DEFAULT '[]'::jsonb,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX protein_xrefs_protein_id_idx ON protein_cross_references(protein_id);
CREATE INDEX protein_xrefs_database_idx ON protein_cross_references(database);
CREATE INDEX protein_xrefs_database_id_idx ON protein_cross_references(database, database_id);
CREATE INDEX protein_xrefs_metadata_idx ON protein_cross_references USING GIN (metadata);

-- Protein comments (function, location, disease from CC lines)
CREATE TABLE protein_comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protein_id UUID NOT NULL REFERENCES protein_metadata(data_source_id) ON DELETE CASCADE,
    topic VARCHAR(100) NOT NULL,
    text TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX protein_comments_protein_id_idx ON protein_comments(protein_id);
CREATE INDEX protein_comments_topic_idx ON protein_comments(topic);
-- Full-text search index removed due to IMMUTABLE requirement
-- Use pg_trgm or separate text search column if needed

-- Update full-text search to include new fields
-- Note: Full-text search index with to_tsvector removed due to IMMUTABLE requirement
-- Use pg_trgm extension or separate text search columns if needed
DROP INDEX IF EXISTS protein_metadata_search_idx;

-- Add index for protein existence queries
CREATE INDEX protein_metadata_existence_idx ON protein_metadata(protein_existence);

-- Add index for organelle queries
CREATE INDEX protein_metadata_organelle_idx ON protein_metadata(organelle) WHERE organelle IS NOT NULL;

-- Add GIN index for array searches
CREATE INDEX protein_metadata_keywords_idx ON protein_metadata USING GIN (keywords);
CREATE INDEX protein_metadata_ec_numbers_idx ON protein_metadata USING GIN (ec_numbers);
CREATE INDEX protein_metadata_alternative_names_idx ON protein_metadata USING GIN (alternative_names);
