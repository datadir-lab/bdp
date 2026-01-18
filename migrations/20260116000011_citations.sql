-- Citations
-- Academic references for data sources and tools.

CREATE TABLE citations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,
    citation_type VARCHAR(50),  -- 'primary', 'method', 'review'
    doi VARCHAR(255),
    pubmed_id VARCHAR(50),
    title TEXT,
    journal VARCHAR(255),
    publication_date DATE,
    volume VARCHAR(50),
    pages VARCHAR(50),
    authors TEXT,  -- Comma-separated author names
    bibtex TEXT,  -- Pre-generated BibTeX entry
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX citations_version_id_idx ON citations(version_id);
CREATE INDEX citations_doi_idx ON citations(doi);
CREATE INDEX citations_pubmed_idx ON citations(pubmed_id);
