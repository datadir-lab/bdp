-- Text embeddings: 512-dim Matryoshka via text-embedding-3-small
-- halfvec = float16, saves 50% vs float32
-- Table disk: 10M × 512 × 2 bytes ≈ 10GB
-- HNSW index RAM: ~5-8GB (separate from table)
CREATE TABLE entry_embeddings (
    entry_id     UUID PRIMARY KEY REFERENCES registry_entries(id) ON DELETE CASCADE,
    model        VARCHAR(100) NOT NULL DEFAULT 'text-embedding-3-small',
    vector       halfvec(512) NOT NULL,
    embedded_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- HNSW for cosine ANN search
-- m=16, ef_construction=64: ~97% recall, ~1-2h build at 10M rows
-- After large batch inserts (>1M rows): run REINDEX CONCURRENTLY to restore recall
CREATE INDEX entry_embeddings_vector_idx ON entry_embeddings
    USING hnsw (vector halfvec_cosine_ops)
    WITH (m = 16, ef_construction = 64);
