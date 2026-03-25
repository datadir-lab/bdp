-- MONDO Disease Ontology domain tables

-- Add 'disease' source type (FK pattern — NO CHECK constraint modification)
INSERT INTO source_types (name, label, description) VALUES
    ('disease', 'Disease', 'Disease ontology terms (MONDO, MeSH, DO)')
ON CONFLICT (name) DO NOTHING;

-- Primary disease term table
CREATE TABLE disease_terms (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id      UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    mondo_id            TEXT NOT NULL,
    mondo_accession     BIGINT NOT NULL,
    name                TEXT NOT NULL,
    definition          TEXT,
    is_obsolete         BOOLEAN NOT NULL DEFAULT FALSE,
    comment             TEXT,
    omim_id             TEXT,
    orphanet_id         TEXT,
    mondo_release       TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_disease_per_release UNIQUE (mondo_id, mondo_release)
);

CREATE INDEX idx_disease_mondo_id    ON disease_terms(mondo_id);
CREATE INDEX idx_disease_accession   ON disease_terms(mondo_accession);
CREATE INDEX idx_disease_omim        ON disease_terms(omim_id) WHERE omim_id IS NOT NULL;
CREATE INDEX idx_disease_orphanet    ON disease_terms(orphanet_id) WHERE orphanet_id IS NOT NULL;
CREATE INDEX idx_disease_data_source ON disease_terms(data_source_id);
CREATE INDEX idx_disease_obsolete    ON disease_terms(is_obsolete) WHERE is_obsolete = FALSE;
CREATE INDEX idx_disease_name_fts    ON disease_terms USING GIN (to_tsvector('english', name));

CREATE TABLE disease_term_synonyms (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id     UUID NOT NULL REFERENCES disease_terms(id) ON DELETE CASCADE,
    scope       TEXT NOT NULL,
    text        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_disease_syn_term ON disease_term_synonyms(term_id);

CREATE TABLE disease_term_xrefs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id     UUID NOT NULL REFERENCES disease_terms(id) ON DELETE CASCADE,
    source_db   TEXT NOT NULL,
    source_id   TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_disease_xref_term      ON disease_term_xrefs(term_id);
CREATE INDEX idx_disease_xref_source_db ON disease_term_xrefs(source_db, source_id);

CREATE TABLE disease_relationships (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_mondo_id    TEXT NOT NULL,
    object_mondo_id     TEXT NOT NULL,
    relationship_type   TEXT NOT NULL,
    mondo_release       TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_disease_rel UNIQUE (subject_mondo_id, object_mondo_id, relationship_type, mondo_release)
);
CREATE INDEX idx_disease_rel_subject ON disease_relationships(subject_mondo_id);
CREATE INDEX idx_disease_rel_object  ON disease_relationships(object_mondo_id);
CREATE INDEX idx_disease_rel_type    ON disease_relationships(relationship_type);
