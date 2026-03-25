-- migrations/20260325000011_go_annotation_extensions.sql
-- Relational table for GO annotation extensions (replaces annotation_extension JSONB).
-- annotation_extension in GAF format stores relation(DB:ID) tuples like
-- occurs_in(CL:0000236). The JSONB stores these parsed into objects.

CREATE TABLE go_annotation_extensions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    annotation_id UUID NOT NULL REFERENCES go_annotations(id) ON DELETE CASCADE,
    relation      TEXT NOT NULL,   -- 'occurs_in', 'has_input', 'part_of', 'has_output'
    filler_db     TEXT NOT NULL,   -- 'CL', 'CHEBI', 'GO', 'UBERON'
    filler_id     TEXT NOT NULL,   -- '0000236', '33709', '0006955'
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_go_ann_ext_ann    ON go_annotation_extensions(annotation_id);
CREATE INDEX idx_go_ann_ext_filler ON go_annotation_extensions(filler_db, filler_id);

-- Data migration: annotation_extension JSONB is stored as array of objects:
-- [{"relation": "occurs_in", "filler_db": "CL", "filler_id": "0000236"}, ...]
-- If the format is different in your data, adjust the JSON path accordingly.
INSERT INTO go_annotation_extensions (annotation_id, relation, filler_db, filler_id)
SELECT
    a.id,
    ext->>'relation',
    ext->>'filler_db',
    ext->>'filler_id'
FROM go_annotations a
CROSS JOIN LATERAL jsonb_array_elements(
    COALESCE(a.annotation_extension, '[]'::jsonb)
) AS ext
WHERE a.annotation_extension IS NOT NULL
  AND jsonb_array_length(a.annotation_extension) > 0
  AND ext->>'relation' IS NOT NULL
ON CONFLICT DO NOTHING;
