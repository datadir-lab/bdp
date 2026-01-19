-- Version-Pinned Dependencies
-- Ensure bundles reference exact dependency versions for reproducibility

-- Add dependency_version_id to dependencies table
ALTER TABLE dependencies
ADD COLUMN dependency_version_id UUID REFERENCES versions(id);

-- Make version pinning required for new dependencies
-- (Existing dependencies may not have version pins)
CREATE INDEX dependencies_version_idx ON dependencies(dependency_version_id);

-- Function to resolve bundle dependencies
CREATE OR REPLACE FUNCTION get_bundle_dependencies(p_bundle_id UUID, p_bundle_version_id UUID)
RETURNS TABLE(
    dependency_id UUID,
    dependency_slug VARCHAR(255),
    version_id UUID,
    version_string VARCHAR(50)
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        d.dependency_id,
        re.slug as dependency_slug,
        d.dependency_version_id as version_id,
        v.version_string
    FROM dependencies d
    JOIN registry_entries re ON re.id = d.dependency_id
    JOIN versions v ON v.id = d.dependency_version_id
    WHERE d.dependent_id = p_bundle_id
      AND d.dependency_version_id IS NOT NULL;
END;
$$ LANGUAGE plpgsql;

-- Comments
COMMENT ON COLUMN dependencies.dependency_version_id IS 'Exact version of dependency (for reproducibility)';
COMMENT ON FUNCTION get_bundle_dependencies IS 'Resolve all dependencies of a bundle to exact versions';
