-- Pre-computed 2D UMAP coords for the /vectors page.
-- Denormalized display fields (label, entry_type, etc.) avoid joins at
-- query time when serving 10M+ rows.
-- entry_type values: 'data_source' | 'tool' (mirrors registry_entries constraint)
CREATE TABLE entry_projections (
    entry_id     UUID PRIMARY KEY REFERENCES registry_entries(id) ON DELETE CASCADE,
    x            FLOAT4 NOT NULL,
    y            FLOAT4 NOT NULL,
    label        TEXT NOT NULL,
    entry_type   VARCHAR(50) NOT NULL,
    source_type  VARCHAR(50),
    org_slug     VARCHAR(100) NOT NULL,
    slug         VARCHAR(255) NOT NULL,
    projected_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX entry_projections_xy_idx ON entry_projections (x, y);
CREATE INDEX entry_projections_source_type_idx ON entry_projections (source_type);
CREATE INDEX entry_projections_type_source_idx ON entry_projections (entry_type, source_type);
