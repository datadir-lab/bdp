-- Human Phenotype Ontology (HPO) Tables

-- Add 'phenotype' source type (FK pattern — NO CHECK constraint modification)
INSERT INTO source_types (name, label, description) VALUES
    ('phenotype', 'Phenotype', 'Phenotype ontology terms (HPO)')
ON CONFLICT (name) DO NOTHING;

CREATE TABLE hpo_term_metadata (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    hpo_id TEXT NOT NULL,
    hpo_accession BIGINT NOT NULL,
    name TEXT NOT NULL,
    definition TEXT,
    comment TEXT,
    is_obsolete BOOLEAN NOT NULL DEFAULT FALSE,
    replaced_by TEXT,
    synonyms JSONB,
    xrefs JSONB,
    alt_ids JSONB,
    subset JSONB,
    hpo_release_version TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_hpo_term_per_version UNIQUE (hpo_id, hpo_release_version)
);

CREATE INDEX idx_hpo_term_hpo_id ON hpo_term_metadata(hpo_id);
CREATE INDEX idx_hpo_term_accession ON hpo_term_metadata(hpo_accession);
CREATE INDEX idx_hpo_term_data_source ON hpo_term_metadata(data_source_id);
CREATE INDEX idx_hpo_term_version ON hpo_term_metadata(hpo_release_version);
CREATE INDEX idx_hpo_term_obsolete ON hpo_term_metadata(is_obsolete) WHERE is_obsolete = FALSE;
CREATE INDEX idx_hpo_term_name_search ON hpo_term_metadata USING GIN (to_tsvector('english', name));

CREATE TABLE hpo_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_hpo_id TEXT NOT NULL,
    object_hpo_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    hpo_release_version TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_hpo_relationship_per_version
        UNIQUE (subject_hpo_id, object_hpo_id, relationship_type, hpo_release_version)
);

CREATE INDEX idx_hpo_rel_subject ON hpo_relationships(subject_hpo_id);
CREATE INDEX idx_hpo_rel_object ON hpo_relationships(object_hpo_id);
CREATE INDEX idx_hpo_rel_type ON hpo_relationships(relationship_type);

CREATE TABLE disease_phenotype_annotations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    disease_db TEXT NOT NULL,
    disease_id TEXT NOT NULL,
    disease_name TEXT NOT NULL,
    hpo_id TEXT NOT NULL,
    qualifier TEXT,
    reference TEXT,
    evidence TEXT,
    onset TEXT,
    frequency TEXT,
    sex TEXT,
    modifier TEXT,
    aspect TEXT,
    biocuration TEXT,
    hpo_release_version TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_disease_phenotype_per_version
        UNIQUE (disease_db, disease_id, hpo_id, hpo_release_version)
);

CREATE INDEX idx_dpa_disease ON disease_phenotype_annotations(disease_db, disease_id);
CREATE INDEX idx_dpa_hpo_id ON disease_phenotype_annotations(hpo_id);
CREATE INDEX idx_dpa_version ON disease_phenotype_annotations(hpo_release_version);
CREATE INDEX idx_dpa_evidence ON disease_phenotype_annotations(evidence);
CREATE INDEX idx_dpa_disease_hpo ON disease_phenotype_annotations(disease_db, disease_id, hpo_id);
CREATE INDEX idx_dpa_hpo_disease ON disease_phenotype_annotations(hpo_id, disease_db, disease_id);
