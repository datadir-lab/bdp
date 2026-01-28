-- Seed default versioning strategies for existing system organizations
-- These define MAJOR vs MINOR version bump rules specific to each organization's data types

-- UniProt versioning strategy
-- MAJOR: protein removal, sequence changes
-- MINOR: protein additions, annotation updates
UPDATE organizations
SET versioning_strategy = '{
  "major_triggers": [
    {"change_type": "removed", "category": "proteins", "description": "Proteins removed or deprecated from SwissProt"},
    {"change_type": "modified", "category": "sequences", "description": "Protein sequences corrected or updated"}
  ],
  "minor_triggers": [
    {"change_type": "added", "category": "proteins", "description": "New proteins added from SwissProt release"},
    {"change_type": "modified", "category": "annotations", "description": "Protein annotations updated (GO terms, features, etc.)"}
  ],
  "default_bump": "minor",
  "cascade_on_major": true,
  "cascade_on_minor": true
}'::jsonb
WHERE slug = 'uniprot';

-- NCBI versioning strategy (covers NCBI Taxonomy, GenBank, RefSeq)
-- MAJOR: taxa removal/merging, sequence removal, scientific name changes
-- MINOR: new taxa/sequences, lineage updates, annotation changes
UPDATE organizations
SET versioning_strategy = '{
  "major_triggers": [
    {"change_type": "removed", "category": "taxa", "description": "Taxonomy nodes removed or merged"},
    {"change_type": "modified", "category": "names", "description": "Scientific names changed"},
    {"change_type": "removed", "category": "sequences", "description": "Sequences withdrawn or superseded"},
    {"change_type": "modified", "category": "sequences", "description": "Sequence data corrected"}
  ],
  "minor_triggers": [
    {"change_type": "added", "category": "taxa", "description": "New taxonomy nodes added"},
    {"change_type": "modified", "category": "lineage", "description": "Lineage relationships refined"},
    {"change_type": "added", "category": "sequences", "description": "New sequences added"},
    {"change_type": "modified", "category": "annotations", "description": "Sequence annotations updated"}
  ],
  "default_bump": "minor",
  "cascade_on_major": true,
  "cascade_on_minor": true
}'::jsonb
WHERE slug = 'ncbi';

-- Gene Ontology versioning strategy
-- MAJOR: term obsolescence
-- MINOR: new terms, definition updates, relationship changes
UPDATE organizations
SET versioning_strategy = '{
  "major_triggers": [
    {"change_type": "removed", "category": "terms", "description": "GO terms marked as obsolete"}
  ],
  "minor_triggers": [
    {"change_type": "added", "category": "terms", "description": "New GO terms added"},
    {"change_type": "modified", "category": "definitions", "description": "GO term definitions updated"},
    {"change_type": "modified", "category": "relationships", "description": "Term relationships added or updated"}
  ],
  "default_bump": "minor",
  "cascade_on_major": true,
  "cascade_on_minor": false
}'::jsonb
WHERE slug = 'go' OR slug = 'gene-ontology';

-- Ensembl versioning strategy
-- MAJOR: gene/transcript removal, coordinate changes
-- MINOR: new genes/transcripts, annotation updates
UPDATE organizations
SET versioning_strategy = '{
  "major_triggers": [
    {"change_type": "removed", "category": "genes", "description": "Genes removed or deprecated"},
    {"change_type": "removed", "category": "transcripts", "description": "Transcripts removed or deprecated"},
    {"change_type": "modified", "category": "coordinates", "description": "Genomic coordinates changed"}
  ],
  "minor_triggers": [
    {"change_type": "added", "category": "genes", "description": "New genes added"},
    {"change_type": "added", "category": "transcripts", "description": "New transcripts added"},
    {"change_type": "modified", "category": "annotations", "description": "Gene annotations updated"}
  ],
  "default_bump": "minor",
  "cascade_on_major": true,
  "cascade_on_minor": true
}'::jsonb
WHERE slug = 'ensembl';

-- PDB (Protein Data Bank) versioning strategy
-- MAJOR: structure withdrawal, coordinate corrections
-- MINOR: new structures, metadata updates
UPDATE organizations
SET versioning_strategy = '{
  "major_triggers": [
    {"change_type": "removed", "category": "structures", "description": "Structures withdrawn or obsoleted"},
    {"change_type": "modified", "category": "coordinates", "description": "Atomic coordinates corrected"}
  ],
  "minor_triggers": [
    {"change_type": "added", "category": "structures", "description": "New structures released"},
    {"change_type": "modified", "category": "metadata", "description": "Structure metadata updated"}
  ],
  "default_bump": "minor",
  "cascade_on_major": true,
  "cascade_on_minor": false
}'::jsonb
WHERE slug = 'pdb' OR slug = 'rcsb';

-- ChEMBL versioning strategy
-- MAJOR: compound removal, assay data corrections
-- MINOR: new compounds, new assays, annotation updates
UPDATE organizations
SET versioning_strategy = '{
  "major_triggers": [
    {"change_type": "removed", "category": "compounds", "description": "Compounds removed or deprecated"},
    {"change_type": "modified", "category": "assays", "description": "Assay data corrected"}
  ],
  "minor_triggers": [
    {"change_type": "added", "category": "compounds", "description": "New compounds added"},
    {"change_type": "added", "category": "assays", "description": "New assay data added"},
    {"change_type": "modified", "category": "annotations", "description": "Compound annotations updated"}
  ],
  "default_bump": "minor",
  "cascade_on_major": true,
  "cascade_on_minor": true
}'::jsonb
WHERE slug = 'chembl';

-- InterPro versioning strategy
-- MAJOR: family removal, significant reclassification
-- MINOR: new families, member updates
UPDATE organizations
SET versioning_strategy = '{
  "major_triggers": [
    {"change_type": "removed", "category": "families", "description": "Protein families removed or merged"},
    {"change_type": "modified", "category": "classification", "description": "Major reclassification of family hierarchy"}
  ],
  "minor_triggers": [
    {"change_type": "added", "category": "families", "description": "New protein families added"},
    {"change_type": "modified", "category": "members", "description": "Family member proteins updated"},
    {"change_type": "modified", "category": "annotations", "description": "Family annotations updated"}
  ],
  "default_bump": "minor",
  "cascade_on_major": true,
  "cascade_on_minor": false
}'::jsonb
WHERE slug = 'interpro';

-- Pfam versioning strategy
-- MAJOR: domain removal, HMM model changes
-- MINOR: new domains, annotation updates
UPDATE organizations
SET versioning_strategy = '{
  "major_triggers": [
    {"change_type": "removed", "category": "domains", "description": "Pfam domains removed or deprecated"},
    {"change_type": "modified", "category": "models", "description": "HMM models significantly changed"}
  ],
  "minor_triggers": [
    {"change_type": "added", "category": "domains", "description": "New Pfam domains added"},
    {"change_type": "modified", "category": "annotations", "description": "Domain annotations updated"},
    {"change_type": "modified", "category": "clans", "description": "Clan memberships updated"}
  ],
  "default_bump": "minor",
  "cascade_on_major": true,
  "cascade_on_minor": false
}'::jsonb
WHERE slug = 'pfam';
