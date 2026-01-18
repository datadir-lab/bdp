-- Organizations table
-- Data providers like UniProt, NCBI, Ensembl, or user-created organizations

CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug VARCHAR(100) UNIQUE NOT NULL,  -- 'uniprot', 'ncbi', 'ensembl'
    name VARCHAR(256) NOT NULL,
    website TEXT,
    description TEXT,
    logo_url TEXT,
    is_system BOOLEAN DEFAULT FALSE,  -- true for hardcoded orgs we scrape
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX organizations_slug_idx ON organizations(slug);
CREATE INDEX organizations_system_idx ON organizations(is_system);
