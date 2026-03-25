-- migrations/20260325000020_protein_xrefs_columns.sql
-- Add typed columns for protein cross-reference metadata.
-- The metadata JSONB column stores a Vec<String> (array of raw UniProt DR line fields).
-- We preserve this data as a text field before dropping the JSONB column.

ALTER TABLE protein_cross_references
    ADD COLUMN additional TEXT;     -- Joined metadata strings from UniProt DR lines

-- Data migration: join the JSONB array of strings into a single text value
UPDATE protein_cross_references
SET additional = (
    SELECT string_agg(elem::TEXT, '; ')
    FROM jsonb_array_elements_text(
        CASE
            WHEN metadata IS NULL THEN '[]'::jsonb
            WHEN jsonb_typeof(metadata) = 'array' THEN metadata
            ELSE '[]'::jsonb
        END
    ) AS elem
)
WHERE metadata IS NOT NULL
  AND metadata != '[]'::jsonb
  AND metadata != 'null'::jsonb;
