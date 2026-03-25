-- migrations/20260325000013_go_term_drop_jsonb_columns.sql
-- Drop JSONB columns from go_term_metadata and go_annotations now that data
-- has been migrated to relational tables (migrations 10, 11, 12).
-- Only run after confirming storage.rs no longer writes to these columns.

ALTER TABLE go_term_metadata
    DROP COLUMN IF EXISTS synonyms,
    DROP COLUMN IF EXISTS xrefs,
    DROP COLUMN IF EXISTS alt_ids;

ALTER TABLE go_annotations
    DROP COLUMN IF EXISTS annotation_extension;
