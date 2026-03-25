-- migrations/20260326000001_reactome_tables.sql

-- 'pathway' source type may already exist from seed — INSERT OR IGNORE
INSERT INTO source_types (name, label, description)
VALUES ('pathway', 'Pathway', 'Biological pathways from Reactome')
ON CONFLICT (name) DO NOTHING;

-- Biological pathway terms
CREATE TABLE pathway_terms (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id  UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    reactome_id     TEXT NOT NULL,       -- 'R-HSA-9612973'
    name            TEXT NOT NULL,
    species_name    TEXT NOT NULL,       -- 'Homo sapiens'
    species_taxid   BIGINT,             -- 9606 (populated from NCBI taxonomy if available)
    is_top_level    BOOLEAN NOT NULL DEFAULT FALSE,
    reactome_release TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_pathway_per_release UNIQUE (reactome_id, reactome_release)
);

CREATE INDEX idx_pathway_reactome_id ON pathway_terms(reactome_id);
CREATE INDEX idx_pathway_species     ON pathway_terms(species_name);
CREATE INDEX idx_pathway_taxid       ON pathway_terms(species_taxid) WHERE species_taxid IS NOT NULL;
CREATE INDEX idx_pathway_top_level   ON pathway_terms(is_top_level) WHERE is_top_level;
CREATE INDEX idx_pathway_data_src    ON pathway_terms(data_source_id);
CREATE INDEX idx_pathway_name_fts    ON pathway_terms
    USING GIN (to_tsvector('english', name));

-- Pathway hierarchy (parent-child) — populated from UniProt2ReactomeAll.txt or inferred
-- Reactome doesn't provide explicit hierarchy file; top-level detection from species-specific file
-- (Pathway hierarchy will be populated in a future enhancement via Reactome's SBML export)

-- TYPED EDGE: protein participates_in pathway (Biolink: biolink:participates_in)
CREATE TABLE protein_pathway_associations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Protein side: UniProt accession (resolved to protein_metadata.id when proteins are loaded)
    uniprot_acc     TEXT NOT NULL,       -- 'P04637' (denormalized — join to protein_metadata at query time)
    -- Pathway side
    pathway_id      UUID NOT NULL REFERENCES pathway_terms(id) ON DELETE CASCADE,
    reactome_id     TEXT NOT NULL,       -- denormalized for fast lookup
    -- Association details
    evidence_type   TEXT,               -- 'IEA', 'inferred_from_experiment', etc.
    species_name    TEXT NOT NULL,
    source_db       TEXT NOT NULL DEFAULT 'reactome',
    reactome_release TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_protein_pathway UNIQUE (uniprot_acc, pathway_id, reactome_release)
);

CREATE INDEX idx_ppa_uniprot    ON protein_pathway_associations(uniprot_acc);
CREATE INDEX idx_ppa_pathway    ON protein_pathway_associations(pathway_id);
CREATE INDEX idx_ppa_species    ON protein_pathway_associations(species_name);
CREATE INDEX idx_ppa_reactome   ON protein_pathway_associations(reactome_id);
