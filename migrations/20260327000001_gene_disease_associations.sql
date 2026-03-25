-- Gene-disease associations from Open Targets
CREATE TABLE gene_disease_associations (
    id               BIGSERIAL PRIMARY KEY,
    gene_id          UUID NOT NULL REFERENCES data_sources(id),
    disease_term_id  UUID NOT NULL REFERENCES disease_terms(id),
    association_type TEXT NOT NULL DEFAULT 'direct',
    score            FLOAT4,
    source           TEXT NOT NULL DEFAULT 'open_targets',
    source_version   TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(gene_id, disease_term_id, source)
);
CREATE INDEX ON gene_disease_associations(gene_id);
CREATE INDEX ON gene_disease_associations(disease_term_id);
CREATE INDEX ON gene_disease_associations(score DESC NULLS LAST);
