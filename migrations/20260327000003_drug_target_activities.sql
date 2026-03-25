-- ChEMBL drug-target bioactivities
CREATE TABLE drug_target_activities (
    id              BIGSERIAL PRIMARY KEY,
    compound_id     UUID NOT NULL REFERENCES data_sources(id),
    target_gene_id  UUID NOT NULL REFERENCES data_sources(id),
    activity_type   TEXT,
    activity_value  FLOAT4,
    activity_unit   TEXT,
    relation        TEXT,
    assay_type      TEXT,
    chembl_assay_id TEXT,
    chembl_doc_id   TEXT,
    confidence      SMALLINT,
    source_version  TEXT NOT NULL DEFAULT 'chembl_36',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(compound_id, target_gene_id, chembl_assay_id)
);
CREATE INDEX ON drug_target_activities(compound_id);
CREATE INDEX ON drug_target_activities(target_gene_id);
CREATE INDEX ON drug_target_activities(activity_type, activity_value);
