-- migrations/20260325000003_ont_term_xrefs.sql
-- Unified cross-reference table for all ontology term types.
-- Using term_id + term_table instead of a polymorphic FK so any term table can be referenced.

CREATE TABLE ont_term_xrefs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id     UUID NOT NULL,
    term_table  TEXT NOT NULL,
    source_db   TEXT NOT NULL,
    source_id   TEXT NOT NULL,
    xref_type   TEXT,           -- 'exact', 'related', 'broader', 'narrower'
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ont_xrefs_term    ON ont_term_xrefs(term_id, term_table);
CREATE INDEX idx_ont_xrefs_source  ON ont_term_xrefs(source_db, source_id);
CREATE INDEX idx_ont_xrefs_reverse ON ont_term_xrefs(source_db, source_id, term_table);
