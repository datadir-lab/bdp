-- Citation Policies
-- Manages citation requirements and policies for organizations

-- Citation policies for organizations
CREATE TABLE citation_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    policy_url TEXT NOT NULL,
    license_id UUID REFERENCES licenses(id),
    requires_version_citation BOOLEAN DEFAULT false,  -- e.g., GO zenodo DOIs
    requires_accession_citation BOOLEAN DEFAULT false,  -- e.g., RefSeq accession.version
    citation_instructions TEXT,  -- Human-readable instructions for how to cite
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT one_policy_per_org UNIQUE(organization_id)
);

-- Required citations for each policy
CREATE TABLE policy_required_citations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id UUID NOT NULL REFERENCES citation_policies(id) ON DELETE CASCADE,
    citation_id UUID NOT NULL REFERENCES citations(id) ON DELETE CASCADE,
    requirement_type VARCHAR(20) NOT NULL CHECK (requirement_type IN ('required', 'recommended', 'conditional')),
    display_order INT NOT NULL,  -- 1 = primary, 2 = secondary, etc.
    context TEXT,  -- e.g., "When using GO enrichment analysis tools"
    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_policy_order UNIQUE(policy_id, display_order),
    CONSTRAINT unique_policy_citation UNIQUE(policy_id, citation_id)
);

-- Indexes
CREATE INDEX citation_policies_org_idx ON citation_policies(organization_id);
CREATE INDEX citation_policies_license_idx ON citation_policies(license_id);
CREATE INDEX policy_citations_policy_idx ON policy_required_citations(policy_id);
CREATE INDEX policy_citations_citation_idx ON policy_required_citations(citation_id);
CREATE INDEX policy_citations_requirement_idx ON policy_required_citations(requirement_type);

-- Comments
COMMENT ON TABLE citation_policies IS 'Citation policies and requirements for each organization';
COMMENT ON TABLE policy_required_citations IS 'Links citation policies to their required citations with ordering';
COMMENT ON COLUMN citation_policies.requires_version_citation IS 'Whether this organization requires version-specific citations (e.g., GO zenodo DOIs)';
COMMENT ON COLUMN citation_policies.requires_accession_citation IS 'Whether individual records require accession-level citations (e.g., RefSeq accession.version)';
COMMENT ON COLUMN policy_required_citations.display_order IS 'Order in which citations should be displayed (1 = primary)';
COMMENT ON COLUMN policy_required_citations.context IS 'Optional context for when this citation applies';
