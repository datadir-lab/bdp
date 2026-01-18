-- Version Mappings
-- Maps external versions to our internal semantic versions.

CREATE TABLE version_mappings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_slug VARCHAR(100) NOT NULL,
    external_version VARCHAR(64) NOT NULL,
    internal_version VARCHAR(64) NOT NULL,
    release_date DATE,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(organization_slug, external_version)
);

-- Indexes
CREATE INDEX version_mappings_org_idx ON version_mappings(organization_slug);
CREATE INDEX version_mappings_external_idx ON version_mappings(external_version);
CREATE INDEX version_mappings_internal_idx ON version_mappings(internal_version);
