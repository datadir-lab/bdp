-- migrations/20260325000012_go_term_migrate_jsonb.sql
-- Data migration: move GO synonyms and xrefs from JSONB columns to relational tables.
-- This runs AFTER ont_term_synonyms and ont_term_xrefs tables exist (migrations 3 & 4).

-- Migrate GO synonyms → ont_term_synonyms
-- JSONB schema: [{"type": "EXACT", "text": "immune response"}, ...]
INSERT INTO ont_term_synonyms (term_id, term_table, scope, text)
SELECT
    g.id,
    'go_term_metadata',
    UPPER(syn->>'type'),    -- 'EXACT', 'BROAD', 'NARROW', 'RELATED'
    syn->>'text'
FROM go_term_metadata g
CROSS JOIN LATERAL jsonb_array_elements(
    COALESCE(g.synonyms, '[]'::jsonb)
) AS syn
WHERE g.synonyms IS NOT NULL
  AND jsonb_array_length(g.synonyms) > 0
  AND syn->>'text' IS NOT NULL
  AND UPPER(syn->>'type') IN ('EXACT', 'BROAD', 'NARROW', 'RELATED')
ON CONFLICT (term_id, term_table, scope, text) DO NOTHING;

-- Migrate GO xrefs → ont_term_xrefs
-- JSONB schema: ["Wikipedia:Immune_response", "KEGG:ko04620", "Reactome:R-HSA-1", ...]
-- Split "DB:ID" → source_db='Wikipedia', source_id='Immune_response'
INSERT INTO ont_term_xrefs (term_id, term_table, source_db, source_id)
SELECT
    g.id,
    'go_term_metadata',
    CASE
        WHEN xref::TEXT LIKE '%:%' THEN split_part(xref::TEXT, ':', 1)
        ELSE 'unknown'
    END,
    CASE
        WHEN xref::TEXT LIKE '%:%' THEN substring(xref::TEXT from position(':' in xref::TEXT) + 1)
        ELSE xref::TEXT
    END
FROM go_term_metadata g
CROSS JOIN LATERAL jsonb_array_elements_text(
    COALESCE(g.xrefs, '[]'::jsonb)
) AS xref
WHERE g.xrefs IS NOT NULL
  AND jsonb_array_length(g.xrefs) > 0
ON CONFLICT DO NOTHING;
