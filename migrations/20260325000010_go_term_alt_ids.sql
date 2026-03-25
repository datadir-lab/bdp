-- migrations/20260325000010_go_term_alt_ids.sql
-- Relational table for GO term alternative IDs (replaces alt_ids JSONB array).
-- Each alt_id is a deprecated GO ID that redirects to the canonical term.

CREATE TABLE go_term_alt_ids (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    go_term_id UUID NOT NULL REFERENCES go_term_metadata(id) ON DELETE CASCADE,
    alt_go_id  TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(go_term_id, alt_go_id)
);

CREATE INDEX idx_go_alt_ids_term ON go_term_alt_ids(go_term_id);
CREATE INDEX idx_go_alt_ids_alt  ON go_term_alt_ids(alt_go_id);

-- Data migration: move existing alt_ids JSONB → relational rows
-- alt_ids JSONB is an array of text: ["GO:0006955", "GO:1234567"]
INSERT INTO go_term_alt_ids (go_term_id, alt_go_id)
SELECT
    g.id,
    alt_id.value::TEXT
FROM go_term_metadata g
CROSS JOIN LATERAL jsonb_array_elements_text(
    COALESCE(g.alt_ids, '[]'::jsonb)
) AS alt_id(value)
WHERE g.alt_ids IS NOT NULL
  AND jsonb_array_length(g.alt_ids) > 0
ON CONFLICT (go_term_id, alt_go_id) DO NOTHING;
