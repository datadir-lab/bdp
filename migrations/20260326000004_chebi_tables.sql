-- migrations/20260326000004_chebi_tables.sql

INSERT INTO source_types (name, label, description)
VALUES ('compound', 'Compound', 'Chemical compounds from ChEBI ontology')
ON CONFLICT (name) DO NOTHING;

CREATE TABLE compound_terms (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id  UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    chebi_id        TEXT NOT NULL,          -- 'CHEBI:33709'
    chebi_accession BIGINT NOT NULL,        -- 33709
    name            TEXT NOT NULL,
    definition      TEXT,
    comment         TEXT,
    is_obsolete     BOOLEAN NOT NULL DEFAULT FALSE,
    -- Chemical identifiers (extracted from OBO property_values)
    inchikey        TEXT,                   -- 'UHOVQNZJYSORNB-UHFFFAOYSA-N'
    smiles          TEXT,                   -- canonical SMILES
    inchi           TEXT,                   -- InChI string
    formula         TEXT,                   -- 'C6H12O6'
    mass_mono       DOUBLE PRECISION,       -- monoisotopic mass
    charge          INTEGER,
    chebi_release   TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_compound_per_release UNIQUE (chebi_id, chebi_release)
);

CREATE INDEX idx_compound_chebi_id    ON compound_terms(chebi_id);
CREATE INDEX idx_compound_accession   ON compound_terms(chebi_accession);
CREATE INDEX idx_compound_inchikey    ON compound_terms(inchikey) WHERE inchikey IS NOT NULL;
CREATE INDEX idx_compound_data_src    ON compound_terms(data_source_id);
CREATE INDEX idx_compound_obsolete    ON compound_terms(is_obsolete) WHERE is_obsolete = FALSE;
CREATE INDEX idx_compound_name_fts    ON compound_terms
    USING GIN (to_tsvector('english', name));

-- Hierarchical and structural relationships
CREATE TABLE compound_relationships (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_chebi_id    TEXT NOT NULL,
    object_chebi_id     TEXT NOT NULL,
    relationship_type   TEXT NOT NULL,   -- 'is_a', 'has_role', 'is_conjugate_acid_of', etc.
    chebi_release       TEXT NOT NULL,
    CONSTRAINT unique_compound_rel UNIQUE (subject_chebi_id, object_chebi_id, relationship_type, chebi_release)
);

CREATE INDEX idx_compound_rel_subject ON compound_relationships(subject_chebi_id);
CREATE INDEX idx_compound_rel_object  ON compound_relationships(object_chebi_id);
CREATE INDEX idx_compound_rel_type    ON compound_relationships(relationship_type);
