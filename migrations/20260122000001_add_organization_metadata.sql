-- Add licensing, citation, and additional metadata fields to organizations table

-- Licensing fields
ALTER TABLE organizations
ADD COLUMN license TEXT,
ADD COLUMN license_url TEXT;

-- Citation fields
ALTER TABLE organizations
ADD COLUMN citation TEXT,
ADD COLUMN citation_url TEXT;

-- Versioning strategy fields (for BDP)
ALTER TABLE organizations
ADD COLUMN version_strategy TEXT,
ADD COLUMN version_description TEXT;

-- Additional metadata fields
ALTER TABLE organizations
ADD COLUMN data_source_url TEXT,
ADD COLUMN documentation_url TEXT,
ADD COLUMN contact_email TEXT;

-- Add helpful comments
COMMENT ON COLUMN organizations.license IS 'License type (e.g., CC-BY-4.0, MIT, Custom)';
COMMENT ON COLUMN organizations.license_url IS 'Link to full license text';
COMMENT ON COLUMN organizations.citation IS 'How to cite this organization''s data';
COMMENT ON COLUMN organizations.citation_url IS 'Link to citation guidelines';
COMMENT ON COLUMN organizations.version_strategy IS 'Versioning approach (e.g., semantic, date-based, release-based)';
COMMENT ON COLUMN organizations.version_description IS 'Description of how versions are managed';
COMMENT ON COLUMN organizations.data_source_url IS 'Link to the original data source';
COMMENT ON COLUMN organizations.documentation_url IS 'Link to documentation';
COMMENT ON COLUMN organizations.contact_email IS 'Contact email for questions';
