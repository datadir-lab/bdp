-- migrations/20260325000004_ont_term_synonyms.sql
-- Unified synonym table for all ontology term types.
-- GIN full-text index enables fast synonym text search across all ontologies.

CREATE TABLE ont_term_synonyms (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id      UUID NOT NULL,
    term_table   TEXT NOT NULL,
    scope        TEXT NOT NULL CHECK (scope IN ('EXACT','BROAD','NARROW','RELATED')),
    text         TEXT NOT NULL,
    synonym_type TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ont_synonyms_term ON ont_term_synonyms(term_id, term_table);
CREATE INDEX idx_ont_synonyms_text ON ont_term_synonyms
    USING GIN (to_tsvector('english', text));
CREATE UNIQUE INDEX idx_ont_synonyms_dedup ON ont_term_synonyms(term_id, term_table, scope, text);
