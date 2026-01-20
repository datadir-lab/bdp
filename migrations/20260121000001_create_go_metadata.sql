-- Gene Ontology (GO) Metadata Tables
-- This migration creates tables for storing GO terms, relationships, and annotations

-- ============================================================================
-- 1. Update data_sources.source_type constraint to include 'go_term'
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
        'bundle'
    )
);

-- ============================================================================
-- 2. GO Term Metadata Table
-- ============================================================================
-- Stores GO term definitions and metadata
-- Links to data_sources via data_source_id for versioning support

CREATE TABLE IF NOT EXISTS go_term_metadata (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,

    -- GO identifiers
    go_id TEXT NOT NULL,                    -- e.g., "GO:0008150"
    go_accession BIGINT NOT NULL,           -- Numeric part: 8150

    -- Term information
    name TEXT NOT NULL,                     -- e.g., "biological_process"
    definition TEXT,                        -- Term definition
    namespace TEXT NOT NULL,                -- 'biological_process', 'molecular_function', 'cellular_component'

    -- Status
    is_obsolete BOOLEAN NOT NULL DEFAULT FALSE,

    -- Additional metadata (JSONB for flexibility)
    synonyms JSONB,                         -- Array of synonym objects: [{"type": "EXACT", "text": "..."}]
    xrefs JSONB,                            -- Array of cross-references: ["Wikipedia:...", "KEGG:..."]
    alt_ids JSONB,                          -- Array of alternative GO IDs
    comments TEXT,                          -- Additional notes

    -- Version tracking
    go_release_version TEXT NOT NULL,      -- e.g., "2026-01-01"

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure uniqueness per version
    CONSTRAINT unique_go_term_per_version UNIQUE (go_id, go_release_version)
);

-- Indexes for efficient queries
CREATE INDEX idx_go_term_go_id ON go_term_metadata(go_id);
CREATE INDEX idx_go_term_accession ON go_term_metadata(go_accession);
CREATE INDEX idx_go_term_namespace ON go_term_metadata(namespace);
CREATE INDEX idx_go_term_data_source ON go_term_metadata(data_source_id);
CREATE INDEX idx_go_term_version ON go_term_metadata(go_release_version);
CREATE INDEX idx_go_term_obsolete ON go_term_metadata(is_obsolete) WHERE is_obsolete = FALSE;

-- Full-text search on term names and definitions
CREATE INDEX idx_go_term_name_search ON go_term_metadata USING GIN (to_tsvector('english', name));
CREATE INDEX idx_go_term_definition_search ON go_term_metadata USING GIN (to_tsvector('english', COALESCE(definition, '')));

-- ============================================================================
-- 3. GO Relationships Table (DAG edges)
-- ============================================================================
-- Stores parent-child relationships between GO terms
-- Supports multiple relationship types (is_a, part_of, regulates, etc.)

CREATE TABLE IF NOT EXISTS go_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Relationship
    subject_go_id TEXT NOT NULL,            -- Child term (e.g., "GO:0006955")
    object_go_id TEXT NOT NULL,             -- Parent term (e.g., "GO:0006950")
    relationship_type TEXT NOT NULL,        -- 'is_a', 'part_of', 'regulates', 'positively_regulates', 'negatively_regulates'

    -- Version tracking
    go_release_version TEXT NOT NULL,       -- e.g., "2026-01-01"

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure uniqueness per version
    CONSTRAINT unique_go_relationship_per_version UNIQUE (subject_go_id, object_go_id, relationship_type, go_release_version)
);

-- Indexes for efficient graph traversal
CREATE INDEX idx_go_rel_subject ON go_relationships(subject_go_id);
CREATE INDEX idx_go_rel_object ON go_relationships(object_go_id);
CREATE INDEX idx_go_rel_type ON go_relationships(relationship_type);
CREATE INDEX idx_go_rel_version ON go_relationships(go_release_version);

-- Composite indexes for bidirectional traversal
CREATE INDEX idx_go_rel_subject_version ON go_relationships(subject_go_id, go_release_version);
CREATE INDEX idx_go_rel_object_version ON go_relationships(object_go_id, go_release_version);

-- ============================================================================
-- 4. GO Annotations Table
-- ============================================================================
-- Links proteins/genes to GO terms with evidence codes
-- Supports both protein and gene annotations

CREATE TABLE IF NOT EXISTS go_annotations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Entity being annotated (protein or gene)
    entity_type TEXT NOT NULL,              -- 'protein' or 'gene'
    entity_id UUID NOT NULL,                -- References data_source_id from protein_metadata or gene_metadata

    -- GO term
    go_id TEXT NOT NULL,                    -- e.g., "GO:0006955"

    -- Evidence and qualifiers
    evidence_code TEXT NOT NULL,            -- e.g., "IDA", "IEA", "TAS", "IMP"
    qualifier TEXT,                         -- e.g., "NOT", "contributes_to", "colocalizes_with"

    -- Supporting information
    reference TEXT,                         -- e.g., "PMID:12345678" or "GO_REF:0000043"
    with_from TEXT,                         -- Supporting evidence (protein IDs, etc.)

    -- Annotation metadata
    annotation_source TEXT,                 -- e.g., "UniProtKB", "SGD"
    assigned_by TEXT,                       -- e.g., "UniProt", "MGI"
    annotation_date DATE,                   -- When annotation was made

    -- Taxonomy (organism)
    taxonomy_id BIGINT,                     -- NCBI Taxonomy ID

    -- Additional metadata
    annotation_extension JSONB,             -- Complex annotation properties
    gene_product_form_id TEXT,              -- Specific isoform/variant

    -- Version tracking
    goa_release_version TEXT NOT NULL,      -- e.g., "2026-01-15"

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT check_entity_type CHECK (entity_type IN ('protein', 'gene'))
);

-- Indexes for efficient queries

-- Entity → GO queries (get all GO terms for a protein/gene)
CREATE INDEX idx_go_ann_entity ON go_annotations(entity_type, entity_id);
CREATE INDEX idx_go_ann_entity_go ON go_annotations(entity_type, entity_id, go_id);

-- GO → Entity queries (get all proteins/genes for a GO term)
CREATE INDEX idx_go_ann_go_entity ON go_annotations(go_id, entity_type);

-- Evidence code filtering
CREATE INDEX idx_go_ann_evidence ON go_annotations(evidence_code);

-- Taxonomy filtering
CREATE INDEX idx_go_ann_taxonomy ON go_annotations(taxonomy_id) WHERE taxonomy_id IS NOT NULL;

-- Version tracking
CREATE INDEX idx_go_ann_version ON go_annotations(goa_release_version);

-- Assigned by filtering
CREATE INDEX idx_go_ann_assigned_by ON go_annotations(assigned_by);

-- Unique index for deduplication (using expression for nullable columns)
CREATE UNIQUE INDEX idx_go_ann_unique ON go_annotations(
    entity_type,
    entity_id,
    go_id,
    evidence_code,
    COALESCE(qualifier, ''),
    COALESCE(reference, ''),
    goa_release_version
);

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON TABLE go_term_metadata IS 'Gene Ontology terms with definitions and metadata';
COMMENT ON TABLE go_relationships IS 'Directed acyclic graph (DAG) relationships between GO terms';
COMMENT ON TABLE go_annotations IS 'Annotations linking proteins/genes to GO terms with evidence';

COMMENT ON COLUMN go_term_metadata.go_id IS 'GO identifier (e.g., GO:0008150)';
COMMENT ON COLUMN go_term_metadata.namespace IS 'One of: biological_process, molecular_function, cellular_component';
COMMENT ON COLUMN go_term_metadata.go_release_version IS 'GO release date (e.g., 2026-01-01)';

COMMENT ON COLUMN go_relationships.relationship_type IS 'Relationship type: is_a, part_of, regulates, positively_regulates, negatively_regulates';

COMMENT ON COLUMN go_annotations.evidence_code IS 'Evidence code (IDA, IEA, TAS, IMP, etc.)';
COMMENT ON COLUMN go_annotations.qualifier IS 'Qualifier: NOT, contributes_to, colocalizes_with, etc.';
COMMENT ON COLUMN go_annotations.goa_release_version IS 'GOA release date (e.g., 2026-01-15)';
