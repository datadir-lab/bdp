-- InterPro Integration Database Schema
-- Phase 1: Core relational tables for InterPro entries (NO JSONB for primary data)
-- Pattern: Individual Data Sources + Foreign Keys + MAJOR.MINOR Versioning

-- ============================================================================
-- 1. Update data_sources.source_type constraint to include 'interpro_entry'
-- ============================================================================

ALTER TABLE data_sources DROP CONSTRAINT IF EXISTS check_source_type;

ALTER TABLE data_sources
ADD CONSTRAINT check_source_type CHECK (
    source_type IN (
        'protein',
        'taxonomy',
        'organism',
        'genomic_sequence',
        'go_term',
        'interpro_entry',
        'bundle'
    )
);

-- ============================================================================
-- 2. InterPro Entry Metadata Table
-- ============================================================================
-- Core metadata for InterPro entries
-- Each entry is an individual data source with independent versioning
-- Pattern: IPR000001 (Kringle) → registry_entry → data_source → versions (1.0, 1.1, 2.0)

CREATE TABLE interpro_entry_metadata (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id UUID NOT NULL UNIQUE REFERENCES data_sources(id) ON DELETE CASCADE,

    -- Core identifiers
    interpro_id VARCHAR(20) NOT NULL UNIQUE,        -- IPR000001
    entry_type VARCHAR(50) NOT NULL,                -- Family, Domain, Repeat, Site, Homologous_superfamily
    name TEXT NOT NULL,                             -- "Kringle"
    short_name VARCHAR(255),                        -- "Kringle"
    description TEXT,                               -- Full description

    -- Status
    is_obsolete BOOLEAN DEFAULT FALSE,
    replacement_interpro_id VARCHAR(20),            -- If obsoleted, what replaces it

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT fk_interpro_data_source
        FOREIGN KEY (data_source_id) REFERENCES data_sources(id) ON DELETE CASCADE,
    CONSTRAINT fk_replacement
        FOREIGN KEY (replacement_interpro_id) REFERENCES interpro_entry_metadata(interpro_id) DEFERRABLE
);

-- Indexes for core metadata
CREATE INDEX idx_interpro_metadata_ds ON interpro_entry_metadata(data_source_id);
CREATE INDEX idx_interpro_metadata_id ON interpro_entry_metadata(interpro_id);
CREATE INDEX idx_interpro_metadata_type ON interpro_entry_metadata(entry_type);
CREATE INDEX idx_interpro_metadata_obsolete ON interpro_entry_metadata(is_obsolete) WHERE is_obsolete = FALSE;
CREATE INDEX idx_interpro_metadata_replacement ON interpro_entry_metadata(replacement_interpro_id) WHERE replacement_interpro_id IS NOT NULL;

-- Full-text search on names and descriptions
CREATE INDEX idx_interpro_metadata_name_search ON interpro_entry_metadata USING GIN (to_tsvector('english', name));
CREATE INDEX idx_interpro_metadata_description_search ON interpro_entry_metadata USING GIN (to_tsvector('english', COALESCE(description, '')));

COMMENT ON TABLE interpro_entry_metadata IS
'Core metadata for InterPro entries. Each entry is an individual data source with independent versioning.';

COMMENT ON COLUMN interpro_entry_metadata.interpro_id IS 'InterPro accession (e.g., IPR000001)';
COMMENT ON COLUMN interpro_entry_metadata.entry_type IS 'Classification: Family, Domain, Repeat, Site, Homologous_superfamily';
COMMENT ON COLUMN interpro_entry_metadata.replacement_interpro_id IS 'Replacement InterPro ID if this entry is obsolete';

-- ============================================================================
-- 3. Protein Signatures Registry (Pfam, SMART, PROSITE, etc.)
-- ============================================================================
-- Registry of protein signatures from member databases (reusable across InterPro entries)
-- Separate table for signature definitions (one signature can belong to multiple InterPro entries)

CREATE TABLE protein_signatures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Signature identification
    database VARCHAR(50) NOT NULL,                  -- 'Pfam', 'SMART', 'PROSITE', 'PRINTS', 'PANTHER', etc.
    accession VARCHAR(50) NOT NULL,                 -- 'PF00051', 'SM00130', 'PS50070'

    -- Metadata
    name VARCHAR(255),                              -- "7 transmembrane receptor"
    description TEXT,                               -- Full description from member database

    -- Pfam-specific (nullable for other databases)
    clan_accession VARCHAR(50),                     -- Pfam clan (e.g., CL0192)
    clan_name VARCHAR(255),                         -- Clan name

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_signature UNIQUE(database, accession)
);

-- Indexes for signatures
CREATE INDEX idx_signatures_database ON protein_signatures(database);
CREATE INDEX idx_signatures_accession ON protein_signatures(accession);
CREATE INDEX idx_signatures_clan ON protein_signatures(clan_accession) WHERE clan_accession IS NOT NULL;

-- Composite index for common lookups
CREATE INDEX idx_signatures_db_accession ON protein_signatures(database, accession);

COMMENT ON TABLE protein_signatures IS
'Registry of protein signatures from member databases (Pfam, SMART, PROSITE, etc.). Reusable across multiple InterPro entries.';

COMMENT ON COLUMN protein_signatures.database IS 'Member database name (Pfam, SMART, PROSITE, PRINTS, PANTHER, etc.)';
COMMENT ON COLUMN protein_signatures.accession IS 'Database-specific accession (PF00051, SM00130, PS50070, etc.)';

-- ============================================================================
-- 4. InterPro ↔ Member Signatures (Many-to-Many)
-- ============================================================================
-- Links InterPro entries to their constituent member database signatures
-- Example: IPR000001 integrates PF00051 (Pfam) + SM00130 (SMART)

CREATE TABLE interpro_member_signatures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Source: InterPro entry
    interpro_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,

    -- Target: Member signature
    signature_id UUID NOT NULL REFERENCES protein_signatures(id) ON DELETE CASCADE,

    -- Relationship metadata
    is_primary BOOLEAN DEFAULT FALSE,               -- Is this the primary signature for this entry?
    integration_date DATE,                          -- When was this signature integrated?

    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_interpro_signature UNIQUE(interpro_data_source_id, signature_id)
);

-- Indexes for bidirectional queries
CREATE INDEX idx_ims_interpro ON interpro_member_signatures(interpro_data_source_id);
CREATE INDEX idx_ims_signature ON interpro_member_signatures(signature_id);
CREATE INDEX idx_ims_primary ON interpro_member_signatures(is_primary) WHERE is_primary = TRUE;

COMMENT ON TABLE interpro_member_signatures IS
'Links InterPro entries to their constituent member database signatures (many-to-many). Example: IPR000001 integrates PF00051 (Pfam) + SM00130 (SMART).';

COMMENT ON COLUMN interpro_member_signatures.is_primary IS 'Whether this signature is the primary/representative signature for the InterPro entry';

-- ============================================================================
-- 5. InterPro ↔ GO Term Mappings (Many-to-Many with Version FKs)
-- ============================================================================
-- Links InterPro entries to Gene Ontology terms with version-specific foreign keys
-- Enables cascade versioning when GO terms update

CREATE TABLE interpro_go_mappings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Source: InterPro entry (version-specific!)
    interpro_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    interpro_version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,

    -- Target: GO term (version-specific!)
    go_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    go_version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,

    -- Evidence
    evidence_code VARCHAR(10),                      -- 'IEA' (Inferred from Electronic Annotation)

    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_interpro_go_mapping
        UNIQUE(interpro_data_source_id, go_data_source_id)
);

-- Indexes for version-specific queries
CREATE INDEX idx_igm_interpro_ds ON interpro_go_mappings(interpro_data_source_id);
CREATE INDEX idx_igm_interpro_ver ON interpro_go_mappings(interpro_version_id);
CREATE INDEX idx_igm_go_ds ON interpro_go_mappings(go_data_source_id);
CREATE INDEX idx_igm_go_ver ON interpro_go_mappings(go_version_id);

-- Composite indexes for common queries
CREATE INDEX idx_igm_interpro_go ON interpro_go_mappings(interpro_data_source_id, go_data_source_id);
CREATE INDEX idx_igm_go_interpro ON interpro_go_mappings(go_data_source_id, interpro_data_source_id);

COMMENT ON TABLE interpro_go_mappings IS
'Links InterPro entries to Gene Ontology terms with version-specific foreign keys. Enables cascade versioning when GO terms update.';

COMMENT ON COLUMN interpro_go_mappings.evidence_code IS 'Evidence code (typically IEA for InterPro)';
COMMENT ON COLUMN interpro_go_mappings.interpro_version_id IS 'Version-specific FK enabling time-travel queries';
COMMENT ON COLUMN interpro_go_mappings.go_version_id IS 'Version-specific FK enabling cascade versioning';

-- ============================================================================
-- 6. Protein ↔ InterPro Matches (Many-to-Many with Coordinates)
-- ============================================================================
-- Main cross-reference table linking UniProt proteins to InterPro entries
-- Includes match coordinates, signature source, and quality scores
-- Version-specific foreign keys enable time-travel queries and cascade versioning

CREATE TABLE protein_interpro_matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Source: InterPro entry (version-specific!)
    interpro_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    interpro_version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,

    -- Target: UniProt protein (version-specific!)
    protein_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    protein_version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,

    -- Denormalized for fast lookups (but still have FK!)
    uniprot_accession VARCHAR(20) NOT NULL,

    -- Match origin: which signature triggered this match
    signature_id UUID NOT NULL REFERENCES protein_signatures(id),

    -- Match coordinates
    start_position INTEGER NOT NULL CHECK (start_position > 0),
    end_position INTEGER NOT NULL CHECK (end_position >= start_position),

    -- Match quality
    e_value DOUBLE PRECISION,
    score DOUBLE PRECISION,

    created_at TIMESTAMPTZ DEFAULT NOW(),

    -- Prevent duplicate matches for same protein-interpro-signature-position
    CONSTRAINT unique_match
        UNIQUE(protein_data_source_id, interpro_data_source_id, signature_id, start_position, end_position)
);

-- Critical indexes for bidirectional queries
CREATE INDEX idx_pim_interpro_ds ON protein_interpro_matches(interpro_data_source_id);
CREATE INDEX idx_pim_interpro_ver ON protein_interpro_matches(interpro_version_id);
CREATE INDEX idx_pim_protein_ds ON protein_interpro_matches(protein_data_source_id);
CREATE INDEX idx_pim_protein_ver ON protein_interpro_matches(protein_version_id);
CREATE INDEX idx_pim_accession ON protein_interpro_matches(uniprot_accession);
CREATE INDEX idx_pim_signature ON protein_interpro_matches(signature_id);

-- Partial indexes for filtered queries
CREATE INDEX idx_pim_positions ON protein_interpro_matches(start_position, end_position);
CREATE INDEX idx_pim_quality ON protein_interpro_matches(e_value) WHERE e_value IS NOT NULL;

-- Composite indexes for common query patterns
CREATE INDEX idx_pim_protein_interpro
ON protein_interpro_matches(protein_data_source_id, interpro_data_source_id);

CREATE INDEX idx_pim_interpro_protein
ON protein_interpro_matches(interpro_data_source_id, protein_data_source_id);

CREATE INDEX idx_pim_accession_interpro
ON protein_interpro_matches(uniprot_accession, interpro_data_source_id);

-- Version-specific composite indexes for time-travel queries
CREATE INDEX idx_pim_protein_ver_interpro_ver
ON protein_interpro_matches(protein_version_id, interpro_version_id);

COMMENT ON TABLE protein_interpro_matches IS
'Links UniProt proteins to InterPro entries with match coordinates. Version-specific foreign keys enable time-travel queries and cascade versioning.';

COMMENT ON COLUMN protein_interpro_matches.uniprot_accession IS 'Denormalized UniProt accession for fast lookups (still have FK to protein_data_source_id)';
COMMENT ON COLUMN protein_interpro_matches.signature_id IS 'Which member database signature triggered this match (Pfam, SMART, etc.)';
COMMENT ON COLUMN protein_interpro_matches.start_position IS 'Match start position in protein sequence (1-indexed)';
COMMENT ON COLUMN protein_interpro_matches.end_position IS 'Match end position in protein sequence (inclusive)';

-- ============================================================================
-- 7. InterPro External References (InterPro → PDB, Wikipedia, etc.)
-- ============================================================================
-- Cross-references from InterPro entries to external databases
-- Examples: PDB structures, CATH classifications, SCOP folds, Wikipedia articles, KEGG pathways

CREATE TABLE interpro_external_references (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    interpro_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,

    -- External database
    database VARCHAR(50) NOT NULL,                  -- 'PDB', 'CATH', 'SCOP', 'Wikipedia', 'KEGG', etc.
    database_id VARCHAR(255) NOT NULL,              -- '1KRI', 'Kringle_domain', etc.

    -- Optional metadata
    description TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_interpro_xref
        UNIQUE(interpro_data_source_id, database, database_id)
);

-- Indexes for external references
CREATE INDEX idx_ixr_interpro ON interpro_external_references(interpro_data_source_id);
CREATE INDEX idx_ixr_database ON interpro_external_references(database);
CREATE INDEX idx_ixr_db_id ON interpro_external_references(database_id);

-- Composite index for lookups
CREATE INDEX idx_ixr_db_dbid ON interpro_external_references(database, database_id);

COMMENT ON TABLE interpro_external_references IS
'Cross-references from InterPro entries to external databases (PDB structures, Wikipedia articles, KEGG pathways, etc.).';

COMMENT ON COLUMN interpro_external_references.database IS 'External database name (PDB, CATH, SCOP, Wikipedia, KEGG, etc.)';
COMMENT ON COLUMN interpro_external_references.database_id IS 'Identifier in external database';

-- ============================================================================
-- 8. InterPro Entry Statistics (Cached Aggregates)
-- ============================================================================
-- Cached statistics for InterPro entries to avoid expensive COUNT queries
-- Updated by triggers when protein matches are inserted/deleted

CREATE TABLE interpro_entry_stats (
    interpro_data_source_id UUID PRIMARY KEY REFERENCES data_sources(id) ON DELETE CASCADE,

    -- Cached counts
    protein_count INTEGER NOT NULL DEFAULT 0,
    species_count INTEGER NOT NULL DEFAULT 0,
    signature_count INTEGER NOT NULL DEFAULT 0,

    last_updated TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE interpro_entry_stats IS
'Cached statistics for InterPro entries to avoid expensive COUNT queries. Updated by triggers.';

COMMENT ON COLUMN interpro_entry_stats.protein_count IS 'Number of proteins with matches to this InterPro entry';
COMMENT ON COLUMN interpro_entry_stats.species_count IS 'Number of species represented in protein matches';
COMMENT ON COLUMN interpro_entry_stats.signature_count IS 'Number of member signatures in this InterPro entry';

-- ============================================================================
-- 9. Trigger Function to Update Statistics
-- ============================================================================
-- Automatically updates interpro_entry_stats when protein matches are added/removed

CREATE OR REPLACE FUNCTION update_interpro_stats()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO interpro_entry_stats (interpro_data_source_id, protein_count)
        VALUES (NEW.interpro_data_source_id, 1)
        ON CONFLICT (interpro_data_source_id)
        DO UPDATE SET
            protein_count = interpro_entry_stats.protein_count + 1,
            last_updated = NOW();
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE interpro_entry_stats
        SET protein_count = GREATEST(0, protein_count - 1),
            last_updated = NOW()
        WHERE interpro_data_source_id = OLD.interpro_data_source_id;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- 10. Trigger on protein_interpro_matches
-- ============================================================================
-- Fire trigger after INSERT or DELETE on protein_interpro_matches

CREATE TRIGGER trigger_update_interpro_stats
AFTER INSERT OR DELETE ON protein_interpro_matches
FOR EACH ROW EXECUTE FUNCTION update_interpro_stats();

COMMENT ON FUNCTION update_interpro_stats() IS
'Trigger function to maintain interpro_entry_stats.protein_count when matches are added or removed';

-- ============================================================================
-- Migration Complete
-- ============================================================================
-- This migration creates:
-- - 7 tables (interpro_entry_metadata, protein_signatures, interpro_member_signatures,
--   interpro_go_mappings, protein_interpro_matches, interpro_external_references,
--   interpro_entry_stats)
-- - 30+ indexes (single-column, composite, partial)
-- - All foreign key constraints with ON DELETE CASCADE
-- - CHECK constraints for data integrity
-- - Trigger function and trigger for automatic statistics updates
-- - Comprehensive table and column comments
--
-- Pattern: Fully relational design with foreign keys (NO JSONB for primary data)
-- Versioning: Version-specific foreign keys enable time-travel queries and cascade versioning
-- Architecture: Individual data sources per InterPro entry with MAJOR.MINOR versioning
