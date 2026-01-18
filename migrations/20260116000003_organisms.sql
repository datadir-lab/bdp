-- Organisms
-- Represents biological organisms (species) for categorization

CREATE TABLE organisms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ncbi_taxonomy_id INTEGER UNIQUE NOT NULL,
    scientific_name VARCHAR(255) NOT NULL,
    common_name VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX organisms_scientific_name_idx ON organisms(scientific_name);
CREATE INDEX organisms_common_name_idx ON organisms(common_name);
CREATE INDEX organisms_taxonomy_idx ON organisms(ncbi_taxonomy_id);
