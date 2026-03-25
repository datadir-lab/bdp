-- migrations/20260325000005_entity_aliases.sql
-- Entity alias resolution table for agent queries.
-- Enables resolving gene symbols, HGNC IDs, Entrez IDs, etc. to canonical identifiers.
-- Trigram index enables fuzzy symbol search ("TP5" → "TP53").

CREATE EXTENSION IF NOT EXISTS pg_trgm;  -- for trigram index below

CREATE TABLE entity_aliases (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    canonical_db TEXT NOT NULL,     -- 'uniprot', 'gene_ontology', 'mondo'
    canonical_id TEXT NOT NULL,     -- 'P04637', 'GO:0006955', 'MONDO:0007254'
    alias_db     TEXT NOT NULL,     -- 'hgnc', 'entrez_gene', 'ensembl', 'symbol'
    alias_id     TEXT NOT NULL,     -- 'HGNC:11998', '7157', 'ENSG00000141510', 'TP53'
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(alias_db, alias_id)
);

CREATE INDEX idx_aliases_canonical ON entity_aliases(canonical_db, canonical_id);
CREATE INDEX idx_aliases_alias     ON entity_aliases(alias_db, alias_id);
-- Trigram index for fuzzy symbol search ("TP5" → "TP53")
CREATE INDEX idx_aliases_symbol_trgm ON entity_aliases
    USING GIN (alias_id gin_trgm_ops)
    WHERE alias_db = 'symbol';
