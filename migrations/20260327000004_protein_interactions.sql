-- STRING protein-protein interactions (human, v12.0)
CREATE TABLE protein_interactions (
    id                    BIGSERIAL PRIMARY KEY,
    protein_a_id          UUID NOT NULL REFERENCES data_sources(id),
    protein_b_id          UUID NOT NULL REFERENCES data_sources(id),
    score_neighborhood    SMALLINT,
    score_fusion          SMALLINT,
    score_cooccurrence    SMALLINT,
    score_coexpression    SMALLINT,
    score_experimental    SMALLINT,
    score_database        SMALLINT,
    score_textmining      SMALLINT,
    combined_score        SMALLINT NOT NULL,
    source_version        TEXT NOT NULL DEFAULT 'string_v12',
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(protein_a_id, protein_b_id)
);
CREATE INDEX ON protein_interactions(protein_a_id);
CREATE INDEX ON protein_interactions(protein_b_id);
CREATE INDEX ON protein_interactions(combined_score DESC);
