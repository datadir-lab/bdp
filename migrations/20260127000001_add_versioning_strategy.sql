-- Add versioning_strategy field to organizations
-- This field stores per-organization rules for version bump determination

ALTER TABLE organizations
ADD COLUMN versioning_strategy JSONB;

-- Add comment explaining the field structure
COMMENT ON COLUMN organizations.versioning_strategy IS
'Defines what constitutes MAJOR vs MINOR version bumps for this organization''s data sources. Structure:
{
  "major_triggers": [
    {"change_type": "removed", "category": "proteins", "description": "Proteins removed or deprecated"},
    {"change_type": "modified", "category": "sequences", "description": "Sequence data changed"},
    {"change_type": "removed", "category": "taxa", "description": "Taxa deleted or merged"},
    {"change_type": "removed", "category": "terms", "description": "GO terms obsoleted"}
  ],
  "minor_triggers": [
    {"change_type": "added", "category": "proteins", "description": "New proteins added"},
    {"change_type": "modified", "category": "annotations", "description": "Annotations updated"},
    {"change_type": "added", "category": "taxa", "description": "New taxa added"}
  ],
  "default_bump": "minor",
  "cascade_on_major": true,
  "cascade_on_minor": true
}';

-- Add GIN index for querying within the JSONB structure
CREATE INDEX idx_organizations_versioning_strategy ON organizations USING GIN (versioning_strategy)
    WHERE versioning_strategy IS NOT NULL;
