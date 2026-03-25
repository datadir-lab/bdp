# BDP MCP Server Design

**Date**: 2026-03-25
**Issues**: BDP-90 (tool schema design), BDP-91 (crate bootstrap)
**Status**: Approved for implementation

---

## Goal

Build `crates/bdp-mcp` — a standalone Rust binary that exposes the BDP biological knowledge graph as an MCP server. AI agents (Claude Desktop, Claude Code, custom agents) use this server to autonomously traverse:

```
gene → disease → phenotype → pathway → drug → clinical trial → literature
```

---

## Transport Strategy

**Phase 1 — stdio** (Claude Desktop / Claude Code compatible):
- rmcp `stdio()` transport, no network required
- Credentials from environment (`DATABASE_URL`)
- No OAuth needed — stdio transport spec-exempts authentication

**Phase 2 — Streamable HTTP** (remote, multi-client):
- Single `/mcp` endpoint (POST for requests, GET for SSE push)
- rmcp `StreamableHttpService` mounted on axum
- OAuth 2.1 + PKCE required (RFC 9728 metadata at `/.well-known/oauth-protected-resource`)
- Bind to `127.0.0.1` for local, validate `Origin` header for remote
- Session identity via `Mcp-Session-Id` header

MCP spec version: **2025-11-25** (implemented by rmcp v1.2.0). HTTP+SSE (2024-11-05) is deprecated — we use Streamable HTTP only.

---

## Crate Structure

```
crates/bdp-mcp/
├── Cargo.toml
└── src/
    ├── main.rs           # --transport stdio|http, --port, --db-url
    ├── config.rs         # Config from env + CLI args
    ├── server.rs         # BdpMcpServer struct; #[tool_router] + #[tool_handler]
    ├── tools/
    │   ├── mod.rs        # re-exports all tool modules
    │   ├── genes.rs      # get_gene, get_gene_pathways, get_gene_diseases (stub)
    │   ├── diseases.rs   # get_disease, get_disease_phenotypes, get_disease_genes (stub)
    │   ├── phenotypes.rs # get_phenotype, get_phenotype_diseases
    │   ├── pathways.rs   # get_pathway, get_pathway_proteins
    │   ├── compounds.rs  # get_compound, get_compound_roles, get_compound_targets (stub)
    │   ├── literature.rs # search_literature (stub), get_publication (stub)
    │   └── traversal.rs  # traverse (partial live), find_connection (stub)
    └── db/
        ├── mod.rs
        ├── resolve.rs    # ID pattern matching + FTS/trgm name resolution
        ├── queries.rs    # sqlx::query() (runtime only — no macro cache dependency)
        └── audit.rs      # agent_query_log INSERT (failure is warn!, never blocks tool result)
```

**Key decisions:**
- `db/queries.rs` uses `sqlx::query()` not `sqlx::query!()` — no SQLx offline cache in the new crate
- `BdpMcpServer` holds `Arc<PgPool>`; derives `#[tool_router]` and `#[tool_handler]` from rmcp macros
- `schemars` feature enabled — JSON Schema auto-derived from Rust types via `#[derive(JsonSchema)]`

---

## Entity Resolution

All tools accept **either** a canonical ID or a name/alias string.

**For gene entities**, resolution uses `entity_aliases` (populated by UniProt/NCBI pipelines):
```
input: "BRCA1"  OR  "P38398"
  ├── if looks like UniProt accession (regex) → direct protein_metadata lookup
  ├── if looks like NCBI Gene ID (numeric) → entity_aliases where alias_db='entrez_gene'
  └── else → entity_aliases where alias_db='symbol' OR 'hgnc', then FTS on entry_name
```

**For ontology entities** (disease, phenotype, compound, pathway), `entity_aliases` is NOT populated — resolution goes directly to canonical ID match or FTS:

```
input: "Alzheimer disease"  OR  "MONDO:0004975"
         │
         ▼
resolve::resolve_entity(input, EntityType::Disease)
├── if matches "MONDO:\d+"  → direct lookup: disease_terms.mondo_id
├── if matches "HP:\d+"     → direct lookup: hpo_term_metadata.hpo_id
├── if matches "CHEBI:\d+"  → direct lookup: compound_terms.chebi_id
├── if matches "R-HSA-\d+"  → direct lookup: pathway_terms.reactome_id
└── else → FTS search (uses existing to_tsvector indexes):
    SELECT id, name, ts_rank(search_vec, query) AS score
    FROM disease_terms, plainto_tsquery('english', $1) query
    WHERE search_vec @@ query
    ORDER BY score DESC LIMIT 5
    -- fallback: also check disease_term_synonyms.text via FTS
```

> **Note on trgm vs FTS**: The existing migrations add `to_tsvector('english', name)` FTS indexes on `disease_terms`, `hpo_term_metadata`, `compound_terms`, and `pathway_terms` but do NOT add `gin_trgm_ops` indexes. Resolution therefore uses FTS (`@@`) not the trgm `%` operator. If trgm fuzzy matching is needed in future (e.g., typo tolerance), a separate migration must add `gin_trgm_ops` indexes on the name and synonym columns. Input is capped at 500 chars regardless.

**Ambiguity handling:**
- Single match (rank > threshold): proceed
- Multiple matches: return `{"error": "ambiguous", "candidates": [{id, name}]}`
- No match: return `{"error": "not_found", "message": "No disease matching '...'. Did you mean: [top candidate]?"}`

---

## Tool Schema

### Output Format (all tools)

Every tool returns dual format per MCP spec **2025-11-25**. `content[0]` (TextContent) is the primary response consumed by the LLM. `structured_content` is supplemental typed JSON for programmatic agent use — not all MCP clients support it yet.

```rust
CallToolResult {
    content: vec![TextContent::new(human_readable_summary)],
    structured_content: Some(json!({
        // tool-specific fields
        "_meta": {
            "datasets_used": [{"name": "mondo", "release": "2026-01-15", "ingested_at": "..."}],
            "query_logged": true,
            "duration_ms": 12
        }
    })),
    is_error: false,
}
```

### Pagination (list tools)

All tools returning arrays support cursor-based pagination:

```
// Request
{ "id": "MONDO:0004975", "cursor": null, "limit": 50 }

// Response structured_content
{ "items": [...], "next_cursor": "eyJvZmZzZXQiOjUwfQ==" }
```

Cursor is base64-encoded `{"offset": N}`. Default limit: 50. Max limit: 200 (enforced server-side).
`total_count` is intentionally omitted — it requires an expensive `COUNT(*)` on every page and agents do not need it (they follow `next_cursor` until null). MCP 2025-11-25 does not require it.

---

### Live Tools (backed by current DB)

#### `get_gene`
```
input:  { id: str }   // UniProt accession, gene symbol, NCBI Gene ID, or name
output: {
  uniprot_acc: str, entry_name: str, gene_name: str,
  organism: str,            // from taxonomy_metadata.scientific_name
  ncbi_taxon_id: int,       // from taxonomy_metadata.taxon_id (NOT protein_metadata directly)
  sequence_length?: int,
  _meta: Meta
}
```
Tables: `registry_entries`, `protein_metadata`, `entity_aliases`, `data_sources`, `taxonomy_metadata`

> **Join note**: `protein_metadata.taxonomy_id` is a UUID FK that points **directly** to `taxonomy_metadata.data_source_id` — `data_sources` is not in this join path. Join: `protein_metadata pm JOIN taxonomy_metadata tm ON tm.data_source_id = pm.taxonomy_id`. Fetch `tm.scientific_name` for `organism` and `tm.taxon_id` for `ncbi_taxon_id`.

#### `get_gene_pathways`
```
input:  { id: str, cursor?: str, limit?: int }
output: { items: Pathway[], next_cursor?: str, _meta: Meta }
```
Tables: `protein_pathway_associations` → `pathway_terms`

#### `get_disease`
```
input:  { id: str }
output: {
  mondo_id: str, name: str, definition?: str,
  omim_id?: str, orphanet_id?: str,
  synonyms: [{scope: str, text: str}],
  xrefs: [{source_db: str, source_id: str}],
  _meta: Meta
}
```
Tables: `disease_terms`, `disease_term_synonyms`, `disease_term_xrefs`

#### `get_disease_phenotypes`
```
input:  { id: str, cursor?: str, limit?: int }
output: {
  items: [{
    hpo_id: str, hpo_name: str,
    frequency?: str, onset?: str, evidence: str, reference?: str
  }],
  next_cursor?: str, _meta: Meta
}
```
Tables: `disease_terms`, `disease_phenotype_annotations`, `hpo_term_metadata`

> **Join note**: `disease_phenotype_annotations` uses OMIM/Orphanet IDs (not MONDO IDs). The join from a MONDO disease to its phenotypes must bridge through `disease_terms.omim_id` and `disease_terms.orphanet_id`:
> ```sql
> SELECT dpa.hpo_id, h.name, dpa.frequency, dpa.onset, dpa.evidence, dpa.reference
> FROM disease_terms dt
> JOIN disease_phenotype_annotations dpa ON (
>     (dt.omim_id IS NOT NULL AND dpa.disease_db = 'OMIM' AND dpa.disease_id = dt.omim_id)
>     OR
>     (dt.orphanet_id IS NOT NULL AND dpa.disease_db = 'ORPHA' AND dpa.disease_id = dt.orphanet_id)
> )
> JOIN hpo_term_metadata h ON h.hpo_id = dpa.hpo_id
> WHERE dt.mondo_id = $1
> ```

#### `get_phenotype`
```
input:  { id: str }
output: {
  hpo_id: str, name: str, definition?: str,
  synonyms: str[],   // deserialized from JSONB column hpo_term_metadata.synonyms
  alt_ids: str[],    // deserialized from JSONB column hpo_term_metadata.alt_ids
  _meta: Meta
}
```
Table: `hpo_term_metadata`

> **JSONB note**: `hpo_term_metadata.synonyms` and `alt_ids` are stored as `JSONB` (not native text arrays). Deserialize with `serde_json::from_value::<Vec<String>>()` after fetching.

#### `get_phenotype_diseases`
```
input:  { id: str, cursor?: str, limit?: int }
output: { items: Disease[], next_cursor?: str, _meta: Meta }
```
Tables: `disease_phenotype_annotations`, `disease_terms`

> **Join note**: Reverse bridge — `disease_phenotype_annotations.disease_id` is an OMIM or Orphanet ID. Join back to `disease_terms` via `disease_terms.omim_id` or `disease_terms.orphanet_id`:
> ```sql
> SELECT dt.mondo_id, dt.name, dt.definition
> FROM disease_phenotype_annotations dpa
> JOIN disease_terms dt ON (
>     (dpa.disease_db = 'OMIM' AND dt.omim_id = dpa.disease_id)
>     OR
>     (dpa.disease_db = 'ORPHA' AND dt.orphanet_id = dpa.disease_id)
> )
> WHERE dpa.hpo_id = $1
>   AND dt.is_obsolete = FALSE  -- exclude obsolete MONDO terms (common across releases)
> ```

#### `get_pathway`
```
input:  { id: str }
output: {
  reactome_id: str, name: str,
  species_name: str,   // maps to pathway_terms.species_name column directly
  is_top_level: bool, release: str,
  _meta: Meta
}
```
Table: `pathway_terms`

#### `get_pathway_proteins`
```
input:  { id: str, cursor?: str, limit?: int }
output: {
  items: [{uniprot_acc: str, evidence_type?: str}],
  next_cursor?: str, _meta: Meta
}
```
Table: `protein_pathway_associations`

#### `get_compound`
```
input:  { id: str }
output: {
  chebi_id: str, name: str, formula?: str,
  inchikey?: str, smiles?: str, definition?: str,
  mass_mono?: float, charge?: int,
  _meta: Meta
}
```
Table: `compound_terms`

#### `get_compound_roles`
```
input:  { id: str, cursor?: str, limit?: int }
output: {
  items: [{ chebi_id: str, name: str, relationship_type: str }],
  next_cursor?: str, _meta: Meta
}
```
Tables: `compound_relationships` JOIN `compound_terms`

> **Join note**: `compound_relationships` has no `name` column. `chebi_id` in the output is `object_chebi_id` (the role compound); `name` requires joining `compound_terms ON compound_terms.chebi_id = compound_relationships.object_chebi_id`. Query direction: `WHERE subject_chebi_id = $resolved_id AND relationship_type = 'has_role'`.

> Live now — `compound_relationships` is fully populated by ChEBI pipeline. Enables `compound → biological_role` traversal (e.g., aspirin → anti-inflammatory agent) without waiting for ChEMBL.

#### `traverse`
```
input:  { start_id: str, path: str[] }
// path example: ["disease", "phenotype"]
// path example: ["gene", "pathway", "protein"]
// path example: ["compound", "role"]
output: { nodes: Node[], edges: Edge[], _meta: Meta }
```
Live paths: `gene→pathway`, `gene→pathway→protein`, `disease→phenotype`, `phenotype→disease`, `compound→role`
Stub paths: anything involving `drug_target`, `literature`, `clinical_trial`, `gene_disease`

---

### Stub Tools (return `not_yet_available`)

Stubs use `is_error: false` (not an MCP error code) because "not yet available" is a planned capability, not a failure. Agents should treat it as informational and not retry.

All stubs return:
```json
{
  "status": "not_yet_available",
  "tool": "<tool_name>",
  "reason": "<human explanation>",
  "tracking": "<Linear issue ID>",
  "expected": "2026-Q3"
}
```

| Tool | Blocked by |
|------|------------|
| `get_gene_diseases` | BDP-88 (DisGeNET pipeline) |
| `get_disease_genes` | BDP-88 (DisGeNET pipeline) |
| `get_disease_trials` | BDP-89 (ClinicalTrials.gov pipeline) |
| `get_compound_targets` | BDP-80 (ChEMBL pipeline) |
| `get_compound_trials` | BDP-89 (ClinicalTrials.gov pipeline) |
| `search_literature` | BDP-84 (PubMed pipeline) |
| `get_publication` | BDP-84 (PubMed pipeline) |
| `find_connection` | Requires full graph layer (post-BDP-88/89) |

---

## Error Handling

| Condition | Response |
|-----------|----------|
| Entity not found | `McpError::invalid_params` with "did you mean" suggestion |
| Ambiguous name | `McpError::invalid_params` with top-3 candidates |
| DB unavailable | `McpError::internal_error` — server stays alive |
| Stub tool called | `CallToolResult` with `is_error: false`, `not_yet_available` JSON |
| Pagination cursor invalid | `McpError::invalid_params("Invalid cursor")` |

Error messages must be **agent-actionable**: explain what was wrong and what the agent can try instead.

---

## Audit Logging

Every tool call — live or stub — attempts to write to `agent_query_log`:

```sql
INSERT INTO agent_query_log (agent_id, tool_name, query_params, dataset_versions, result_count, duration_ms)
VALUES ($1, $2, $3, $4, $5, $6)
```

`agent_id` comes from the MCP session context (or `"anonymous"` for stdio without identity).

**Failure policy**: if the audit INSERT fails (transient DB error), log `warn!(tool, error, "audit write failed")` and return the tool result normally. Audit infrastructure failures must never surface to the agent.

---

## Dependencies (Cargo.toml)

```toml
[dependencies]
rmcp = { version = "1.2", features = ["server", "macros", "schemars", "transport-streamable-http-server", "transport-async-rw"] }
axum = { version = "0.7", features = ["tokio"] }
schemars = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "uuid", "chrono"] }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1", features = ["v4", "serde"] }
base64 = "0.22"
anyhow = "1"
clap = { version = "4", features = ["derive", "env"] }
```

---

## Bootstrap Sequence (BDP-91 acceptance criteria)

1. `cargo new --bin crates/bdp-mcp` + add to workspace `Cargo.toml` members
2. Implement `BdpMcpServer` with `#[tool_router]` + `#[tool_handler]`
3. Add one live tool: `get_disease` end-to-end against PostgreSQL
4. stdio transport working with Claude Desktop
5. `cargo build -p bdp-mcp` compiles clean
6. Streamable HTTP transport on `/mcp`
7. All tool stubs returning `not_yet_available`
8. `agent_query_log` writes on every call (failures are swallowed with `warn!`)

---

## Security Checklist

- [ ] stdio: credentials from `DATABASE_URL` env only, never CLI args
- [ ] HTTP: bind `127.0.0.1` for local, configurable for remote
- [ ] HTTP: validate `Origin` header (DNS rebinding protection)
- [ ] HTTP: `Mcp-Session-Id` validated on all non-initialize requests
- [ ] HTTP: CORS — disabled for non-browser clients (Claude Desktop / CLI); if browser access is needed, explicit allowlist required
- [ ] All SQL: parameterized queries only — no string interpolation
- [ ] Entity resolution input: capped at 500 chars before FTS query
- [ ] Result limits: max 200 per page, enforced server-side regardless of client request
- [ ] Production HTTP: OAuth 2.1 + PKCE + RFC 9728 `/.well-known/oauth-protected-resource` metadata endpoint
