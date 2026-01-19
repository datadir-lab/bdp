-- Organization Versioning Rules
-- Add versioning rules documentation for researchers

ALTER TABLE organizations
ADD COLUMN versioning_rules TEXT;  -- Markdown documentation

-- Comments
COMMENT ON COLUMN organizations.versioning_rules IS 'Versioning rules documentation (Markdown) shown to researchers';

-- Seed UniProt versioning rules
UPDATE organizations
SET versioning_rules = $markdown$
# UniProt Versioning Rules

## Overview
UniProt data sources follow **semantic versioning** (MAJOR.MINOR.PATCH) for reproducibility.

## Version Numbering

### MAJOR Version Bump (X.0.0)
Breaking changes that affect downstream analysis:
- Amino acid sequence changed
- Protein length changed
- Organism reclassified
- Protein merged or split
- Accession changed

### MINOR Version Bump (x.Y.0)
Non-breaking metadata updates:
- Gene name changed
- Protein name updated
- Functional annotation added
- New cross-references

### PATCH Version Bump (x.y.Z)
Minor corrections:
- Typo fixed in description
- Cross-reference URL updated
- Minor metadata corrections

## Reproducibility
All versions are **immutable**. Version 1.2.3 always returns the same data.

## License
UniProt data: **CC-BY-4.0** (attribution required)

Attribution:
> UniProt Consortium. UniProt: the Universal Protein Knowledgebase in 2025.
> Nucleic Acids Res. 2025 Jan; 53(D1):D609-D618.
$markdown$
WHERE slug = 'uniprot';
