-- migrations/20260325000021_protein_xrefs_drop_jsonb.sql
-- Drop the metadata JSONB column from protein_cross_references.
-- Data has been migrated to the 'additional' TEXT column in migration 20.

ALTER TABLE protein_cross_references DROP COLUMN IF EXISTS metadata;
