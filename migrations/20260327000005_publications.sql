-- PubMed publications and entity annotations
CREATE TABLE publications (
    id          BIGSERIAL PRIMARY KEY,
    pmid        INTEGER NOT NULL UNIQUE,
    pmcid       TEXT,
    doi         TEXT,
    title       TEXT NOT NULL,
    abstract    TEXT,
    pub_date    DATE,
    journal     TEXT,
    source      TEXT NOT NULL DEFAULT 'pubmed',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX ON publications(pmid);
CREATE INDEX ON publications(pub_date DESC NULLS LAST);
CREATE INDEX ON publications USING GIN (to_tsvector('english', coalesce(title,'') || ' ' || coalesce(abstract,'')));

CREATE TABLE publication_authors (
    id             BIGSERIAL PRIMARY KEY,
    publication_id BIGINT NOT NULL REFERENCES publications(id),
    position       SMALLINT NOT NULL,
    last_name      TEXT,
    fore_name      TEXT,
    collective     TEXT,
    affiliation    TEXT
);
CREATE INDEX ON publication_authors(publication_id);

CREATE TABLE publication_mesh (
    id             BIGSERIAL PRIMARY KEY,
    publication_id BIGINT NOT NULL REFERENCES publications(id),
    mesh_ui        TEXT NOT NULL,
    descriptor     TEXT NOT NULL,
    is_major_topic BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX ON publication_mesh(publication_id);
CREATE INDEX ON publication_mesh(mesh_ui);

CREATE TABLE publication_entities (
    id              BIGSERIAL PRIMARY KEY,
    publication_id  BIGINT NOT NULL REFERENCES publications(id),
    entity_type     TEXT NOT NULL,
    external_id     TEXT NOT NULL,
    entity_name     TEXT,
    gene_id         UUID REFERENCES data_sources(id),
    disease_term_id UUID REFERENCES disease_terms(id),
    compound_id     UUID REFERENCES data_sources(id)
);
CREATE INDEX ON publication_entities(publication_id);
CREATE INDEX ON publication_entities(entity_type, external_id);
CREATE INDEX ON publication_entities(gene_id) WHERE gene_id IS NOT NULL;
CREATE INDEX ON publication_entities(disease_term_id) WHERE disease_term_id IS NOT NULL;
CREATE INDEX ON publication_entities(compound_id) WHERE compound_id IS NOT NULL;

CREATE TABLE pubmed_ingest_files (
    id            BIGSERIAL PRIMARY KEY,
    filename      TEXT NOT NULL UNIQUE,
    record_count  INTEGER,
    status        TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT,
    processed_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
