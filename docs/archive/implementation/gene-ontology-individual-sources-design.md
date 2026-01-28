# Gene Ontology Individual Data Sources - Design Document

**Status**: Design Phase
**Date**: 2026-01-28
**Author**: Claude (Assistant)

---

## Executive Summary

This document outlines the refactoring of Gene Ontology (GO) ingestion from an **aggregate pattern** (1 data source for all terms) to an **individual pattern** (1 data source per GO term), matching the architecture used by UniProt, NCBI Taxonomy, and GenBank.

**Current (WRONG)**:
- 1 registry_entry: "Gene Ontology"
- 1 data_source (type: `go_term`)
- ~47,000 GO terms stored in `go_term_metadata` table

**Target (CORRECT)**:
- ~47,000 registry_entries (one per GO term)
- ~47,000 data_sources (one per GO term)
- Each GO term is individually versionable, downloadable, and citable

---

## 1. Current Architecture Analysis

### 1.1 Current Storage Pattern (Aggregate)

The current implementation in `crates/bdp-server/src/ingest/gene_ontology/storage.rs` creates a single aggregate data source:

```rust
// Current approach (WRONG)
async fn create_go_data_source(...) -> Result<Uuid> {
    // Creates ONE registry entry for ALL GO terms
    let entry_id: Uuid = sqlx::query_scalar(
        "INSERT INTO registry_entries (organization_id, source_type, name, description)
         VALUES ($1, 'go_term', 'Gene Ontology', 'Gene Ontology Consortium')
         ON CONFLICT (organization_id, source_type, name) DO UPDATE ..."
    )
    .bind(self.organization_id)
    .fetch_one(&mut **tx)
    .await?;

    // Creates ONE data_source for ALL terms
    let data_source_id: Uuid = sqlx::query_scalar(
        "INSERT INTO data_sources (registry_entry_id, source_type, external_id, metadata)
         VALUES ($1, 'go_term', $2, $3)
         ON CONFLICT (registry_entry_id, external_id) DO UPDATE ..."
    )
    .bind(entry_id)
    .bind(go_release_version)
    .bind(metadata_json)
    .fetch_one(&mut **tx)
    .await?;

    // Stores 47,000 terms in go_term_metadata linked to this ONE data_source_id
    // ...
}
```

### 1.2 Current Schema

**Tables**:
- `go_term_metadata` - Stores all GO terms (linked to single data_source_id)
- `go_relationships` - Stores DAG edges between GO terms
- `go_annotations` - Links proteins/genes to GO terms

**Problems**:
1. Cannot version individual GO terms (only bulk versioning)
2. Cannot download individual GO terms (no version_files per term)
3. Cannot track changes at term granularity
4. No individual citations per term
5. Inconsistent with UniProt/NCBI Taxonomy patterns

---

## 2. Target Architecture (Individual Sources)

### 2.1 Individual Pattern (Like UniProt/NCBI Taxonomy)

Each GO term becomes a separate data source with its own lifecycle:

```rust
// Target approach (CORRECT)
async fn store_go_term_as_data_source(
    &self,
    tx: &mut Transaction<'_, Postgres>,
    term: &GoTerm,
) -> Result<Uuid> {
    // 1. Create registry_entry for this GO term
    let slug = &term.go_id; // e.g., "GO:0008150"
    let name = format!("{} ({})", term.name, term.go_id);
    let description = format!("GO Term: {} - {}", term.go_id, term.namespace.as_str());

    let entry_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
        VALUES ($1, $2, $3, $4, 'data_source')
        ON CONFLICT (slug) DO UPDATE SET
            name = EXCLUDED.name,
            description = EXCLUDED.description,
            updated_at = NOW()
        RETURNING id
        "#
    )
    .bind(self.organization_id)
    .bind(slug)
    .bind(&name)
    .bind(&description)
    .fetch_one(&mut **tx)
    .await?;

    // 2. Create data_source for this GO term
    sqlx::query(
        r#"
        INSERT INTO data_sources (id, source_type)
        VALUES ($1, 'go_term')
        ON CONFLICT (id) DO NOTHING
        "#
    )
    .bind(entry_id)
    .execute(&mut **tx)
    .await?;

    // 3. Create go_term_metadata linked to this data_source
    sqlx::query(
        r#"
        INSERT INTO go_term_metadata (
            data_source_id, go_id, go_accession, name, definition,
            namespace, is_obsolete, synonyms, xrefs, alt_ids, comments,
            go_release_version
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (data_source_id) DO UPDATE SET
            name = EXCLUDED.name,
            definition = EXCLUDED.definition,
            is_obsolete = EXCLUDED.is_obsolete,
            synonyms = EXCLUDED.synonyms,
            xrefs = EXCLUDED.xrefs,
            alt_ids = EXCLUDED.alt_ids,
            comments = EXCLUDED.comments,
            updated_at = NOW()
        "#
    )
    .bind(entry_id)
    .bind(&term.go_id)
    .bind(term.go_accession)
    .bind(&term.name)
    .bind(&term.definition)
    .bind(term.namespace.as_str())
    .bind(term.is_obsolete)
    .bind(serde_json::to_value(&term.synonyms).unwrap())
    .bind(serde_json::to_value(&term.xrefs).unwrap())
    .bind(serde_json::to_value(&term.alt_ids).unwrap())
    .bind(&term.comments)
    .bind(&term.go_release_version)
    .execute(&mut **tx)
    .await?;

    // 4. Create version for this GO term
    let version_id = self.create_version_tx(tx, entry_id).await?;

    // 5. Create version_files (OBO, JSON, TSV)
    self.create_version_files_tx(tx, term, version_id).await?;

    Ok(entry_id)
}
```

### 2.2 Schema Changes

#### Migration File: `migrations/20260128000001_go_individual_sources.sql`

```sql
-- ============================================================================
-- Gene Ontology Individual Sources Refactoring
-- ============================================================================

-- 1. Update go_term_metadata to support individual data sources
--    Change: data_source_id now references individual GO term data sources

ALTER TABLE go_term_metadata
    DROP CONSTRAINT IF EXISTS unique_go_term_per_version;

-- New constraint: one metadata row per data_source (1:1 relationship)
ALTER TABLE go_term_metadata
    ADD CONSTRAINT unique_go_term_per_data_source UNIQUE (data_source_id);

-- Keep version-based uniqueness on go_id for historical queries
CREATE UNIQUE INDEX idx_go_term_version_unique
    ON go_term_metadata(go_id, go_release_version);

COMMENT ON TABLE go_term_metadata IS
'GO term metadata - now linked to individual data_sources (one row per GO term data source)';

-- ============================================================================
-- 2. Update go_relationships to link data_source_ids instead of GO IDs
-- ============================================================================

-- Add new columns for data_source_id references
ALTER TABLE go_relationships
    ADD COLUMN subject_data_source_id UUID,
    ADD COLUMN object_data_source_id UUID;

-- Add foreign key constraints
ALTER TABLE go_relationships
    ADD CONSTRAINT fk_subject_data_source
        FOREIGN KEY (subject_data_source_id)
        REFERENCES data_sources(id) ON DELETE CASCADE;

ALTER TABLE go_relationships
    ADD CONSTRAINT fk_object_data_source
        FOREIGN KEY (object_data_source_id)
        REFERENCES data_sources(id) ON DELETE CASCADE;

-- Create indexes for efficient graph traversal
CREATE INDEX idx_go_rel_subject_ds ON go_relationships(subject_data_source_id);
CREATE INDEX idx_go_rel_object_ds ON go_relationships(object_data_source_id);

-- Add note about dual representation (transition period)
COMMENT ON COLUMN go_relationships.subject_go_id IS
'Legacy: GO ID string (kept for backward compatibility during transition)';
COMMENT ON COLUMN go_relationships.subject_data_source_id IS
'New: Data source ID for subject GO term (preferred for queries)';

-- ============================================================================
-- 3. Update go_annotations to link data_source_id instead of GO ID string
-- ============================================================================

-- Add new column for GO term data_source_id
ALTER TABLE go_annotations
    ADD COLUMN go_term_data_source_id UUID;

-- Add foreign key constraint
ALTER TABLE go_annotations
    ADD CONSTRAINT fk_go_term_data_source
        FOREIGN KEY (go_term_data_source_id)
        REFERENCES data_sources(id) ON DELETE CASCADE;

-- Create index for efficient queries
CREATE INDEX idx_go_ann_term_ds ON go_annotations(go_term_data_source_id);

-- Composite index for protein -> GO term queries
CREATE INDEX idx_go_ann_entity_term_ds
    ON go_annotations(entity_type, entity_id, go_term_data_source_id);

-- Add note about dual representation
COMMENT ON COLUMN go_annotations.go_id IS
'Legacy: GO ID string (kept for backward compatibility)';
COMMENT ON COLUMN go_annotations.go_term_data_source_id IS
'New: Data source ID for GO term (preferred for queries)';

-- ============================================================================
-- 4. Create helper view for backward compatibility
-- ============================================================================

CREATE OR REPLACE VIEW go_terms_view AS
SELECT
    tm.data_source_id,
    tm.go_id,
    tm.go_accession,
    tm.name,
    tm.definition,
    tm.namespace,
    tm.is_obsolete,
    tm.synonyms,
    tm.xrefs,
    tm.alt_ids,
    tm.comments,
    tm.go_release_version,
    re.slug,
    re.organization_id,
    tm.created_at,
    tm.updated_at
FROM go_term_metadata tm
JOIN data_sources ds ON ds.id = tm.data_source_id
JOIN registry_entries re ON re.id = ds.id
WHERE ds.source_type = 'go_term';

COMMENT ON VIEW go_terms_view IS
'Convenient view joining GO term metadata with registry/data_source info';
```

---

## 3. Versioning Strategy

### 3.1 Semantic Versioning for GO Terms

Each GO term follows semantic versioning: `MAJOR.MINOR.PATCH`

**Version Bump Rules** (based on `versioning_strategy`):

| Change Type | Examples | Bump Type | Reason |
|-------------|----------|-----------|--------|
| **Term obsoleted** | `is_obsolete: false → true` | **MAJOR** | Breaking change - term no longer valid |
| **Definition changed** | `definition: "..." → "different text"` | **MINOR** | Semantic refinement |
| **Name changed** | `name: "biological_process" → "biological process"` | **MINOR** | Clarification |
| **Synonyms added** | New synonym added to `synonyms` array | **MINOR** | Enhancement |
| **Xrefs added** | New cross-reference added | **MINOR** | Enhancement |
| **Relationships changed** | `is_a` parent added/removed | **MINOR*** | *Can be MAJOR if breaks inference |

**Initial Version**: All GO terms start at `1.0.0` (first ingestion)

### 3.2 Versioning Strategy Configuration

**Organization**: Gene Ontology Consortium

```json
{
  "major_triggers": [
    {
      "change_type": "obsoleted",
      "category": "go_terms",
      "description": "GO term marked as obsolete"
    },
    {
      "change_type": "removed",
      "category": "go_terms",
      "description": "GO term removed from ontology"
    }
  ],
  "minor_triggers": [
    {
      "change_type": "modified",
      "category": "definition",
      "description": "GO term definition text changed"
    },
    {
      "change_type": "modified",
      "category": "name",
      "description": "GO term name changed"
    },
    {
      "change_type": "added",
      "category": "synonyms",
      "description": "Synonyms added or modified"
    },
    {
      "change_type": "added",
      "category": "xrefs",
      "description": "Cross-references added"
    },
    {
      "change_type": "modified",
      "category": "relationships",
      "description": "Parent/child relationships changed"
    }
  ],
  "default_bump": "minor",
  "cascade_on_major": true,
  "cascade_on_minor": false
}
```

### 3.3 Version Cascade Behavior

**When a GO term bumps MAJOR**:
- Cascade to all proteins/genes annotated with this term
- Rationale: If `GO:0008150` is obsoleted, all proteins using it need re-annotation

**When a GO term bumps MINOR**:
- No cascade (definition refinements don't break downstream usage)

### 3.4 Example Version Timeline

**GO:0008150 (biological_process)**:

| Date | Version | Change | Trigger |
|------|---------|--------|---------|
| 2026-01-01 | `1.0.0` | Initial ingestion | - |
| 2026-02-15 | `1.1.0` | Definition refined | `MINOR` (definition changed) |
| 2026-03-01 | `1.2.0` | Synonym "biological process" added | `MINOR` (synonym added) |
| 2026-06-01 | `2.0.0` | Term obsoleted, replaced by GO:0099999 | `MAJOR` (obsoleted) |

---

## 4. Code Structure Changes

### 4.1 New Storage Methods

**File**: `crates/bdp-server/src/ingest/gene_ontology/storage.rs`

```rust
impl GoStorage {
    /// Store GO ontology as individual data sources (NEW)
    pub async fn store_ontology_individual(
        &self,
        terms: &[GoTerm],
        relationships: &[GoRelationship],
        go_release_version: &str,
        internal_version: &str,
    ) -> Result<StorageStats> {
        info!(
            "Storing {} GO terms as individual data sources",
            terms.len()
        );

        let mut tx = self.db.begin().await?;

        // Process in batches (like NCBI Taxonomy)
        let total_chunks = (terms.len() + self.term_chunk_size - 1) / self.term_chunk_size;
        let mut stored_count = 0;

        for (chunk_idx, chunk) in terms.chunks(self.term_chunk_size).enumerate() {
            info!(
                "Processing chunk {} / {} ({} terms)",
                chunk_idx + 1,
                total_chunks,
                chunk.len()
            );

            // Batch insert registry entries, data sources, metadata
            self.store_terms_batch(&mut tx, chunk, go_release_version, internal_version).await?;
            stored_count += chunk.len();
        }

        // Store relationships (link data_source_ids)
        self.store_relationships_individual(&mut tx, relationships, go_release_version).await?;

        tx.commit().await?;

        info!("Successfully stored {} GO term data sources", stored_count);

        Ok(StorageStats {
            terms_stored: stored_count,
            relationships_stored: relationships.len(),
            annotations_stored: 0,
        })
    }

    /// Batch insert GO terms as individual data sources
    async fn store_terms_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        terms: &[GoTerm],
        go_release_version: &str,
        internal_version: &str,
    ) -> Result<()> {
        // Similar to NCBI Taxonomy's batch_upsert_registry_entries
        let mut entry_id_map = HashMap::new();

        // 1. Batch upsert registry_entries
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO registry_entries (id, organization_id, slug, name, description, entry_type) "
        );

        query_builder.push_values(terms, |mut b, term| {
            let id = Uuid::new_v4();
            entry_id_map.insert(term.go_id.clone(), id);

            let slug = &term.go_id;
            let name = format!("{} ({})", term.name, term.go_id);
            let description = format!("GO Term: {} - {}", term.go_id, term.namespace.as_str());

            b.push_bind(id)
                .push_bind(self.organization_id)
                .push_bind(slug)
                .push_bind(name)
                .push_bind(description)
                .push_bind("data_source");
        });

        query_builder.push(
            " ON CONFLICT (slug) DO UPDATE SET \
             name = EXCLUDED.name, \
             description = EXCLUDED.description, \
             updated_at = NOW() \
             RETURNING id, slug"
        );

        let rows = query_builder
            .build_query_as::<(Uuid, String)>()
            .fetch_all(&mut **tx)
            .await?;

        // Update map with actual IDs (handles conflicts)
        let mut result_map = HashMap::new();
        for (id, slug) in rows {
            result_map.insert(slug, id);
        }

        // 2. Batch insert data_sources
        // 3. Batch insert go_term_metadata
        // 4. Batch insert versions
        // 5. Batch insert version_files (OBO snippet, JSON, TSV)

        // ... (similar to NCBI Taxonomy pattern)

        Ok(())
    }

    /// Store relationships with data_source_id links
    async fn store_relationships_individual(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        relationships: &[GoRelationship],
        go_release_version: &str,
    ) -> Result<()> {
        // Get data_source_ids for all GO terms in relationships
        let all_go_ids: HashSet<String> = relationships
            .iter()
            .flat_map(|r| vec![r.subject_go_id.clone(), r.object_go_id.clone()])
            .collect();

        // Lookup data_source_ids
        let go_id_to_ds_id = self.lookup_go_data_source_ids(tx, &all_go_ids).await?;

        // Batch insert relationships with both GO IDs and data_source_ids
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO go_relationships \
             (subject_go_id, object_go_id, subject_data_source_id, object_data_source_id, \
              relationship_type, go_release_version) "
        );

        query_builder.push_values(relationships, |mut b, rel| {
            let subject_ds_id = go_id_to_ds_id.get(&rel.subject_go_id);
            let object_ds_id = go_id_to_ds_id.get(&rel.object_go_id);

            b.push_bind(&rel.subject_go_id)
                .push_bind(&rel.object_go_id)
                .push_bind(subject_ds_id)
                .push_bind(object_ds_id)
                .push_bind(rel.relationship_type.as_str())
                .push_bind(go_release_version);
        });

        query_builder.push(" ON CONFLICT DO NOTHING");
        query_builder.build().execute(&mut **tx).await?;

        Ok(())
    }
}
```

### 4.2 Version File Formats

Each GO term data source creates 3 version files:

**1. OBO Format** (`GO:0008150.obo`):
```obo
[Term]
id: GO:0008150
name: biological_process
namespace: biological_process
def: "A biological process is..." [GOC:go_curators]
synonym: "biological process" EXACT []
is_a: GO:0003674 ! root term
```

**2. JSON Format** (`GO:0008150.json`):
```json
{
  "go_id": "GO:0008150",
  "go_accession": 8150,
  "name": "biological_process",
  "definition": "A biological process is...",
  "namespace": "biological_process",
  "is_obsolete": false,
  "synonyms": [
    {"scope": "EXACT", "text": "biological process", "xrefs": []}
  ],
  "xrefs": ["Wikipedia:Biological_process"],
  "alt_ids": [],
  "go_release_version": "2026-01-01"
}
```

**3. TSV Format** (`GO:0008150.tsv`):
```tsv
go_id	go_accession	name	namespace	definition	is_obsolete
GO:0008150	8150	biological_process	biological_process	A biological process is...	false
```

**S3 Storage**:
```
s3://bdp-data/go/terms/GO:0008150/1.0.0/GO:0008150.obo
s3://bdp-data/go/terms/GO:0008150/1.0.0/GO:0008150.json
s3://bdp-data/go/terms/GO:0008150/1.0.0/GO:0008150.tsv
```

---

## 5. Change Detection Logic

### 5.1 Version Bump Detector

**File**: `crates/bdp-server/src/ingest/versioning/detector.rs`

```rust
/// Detect version bumps for Gene Ontology terms
pub struct GeneOntologyBumpDetector {
    organization_id: Uuid,
}

#[async_trait]
impl VersionBumpDetector for GeneOntologyBumpDetector {
    async fn detect_changes(
        &self,
        pool: &PgPool,
        data_source_id: Uuid,
        new_data: &dyn Any,
    ) -> Result<VersionChangelog> {
        let new_term = new_data.downcast_ref::<GoTerm>()
            .ok_or_else(|| anyhow::anyhow!("Invalid data type for GO detector"))?;

        // Get previous version of this GO term
        let previous_term = self.get_previous_go_term(pool, &new_term.go_id).await?;

        match previous_term {
            None => {
                // New GO term
                Ok(VersionChangelog::new(
                    BumpType::Initial,
                    vec![ChangelogEntry {
                        change_type: ChangeType::Added,
                        category: "go_term".to_string(),
                        description: format!("New GO term: {}", new_term.go_id),
                        metadata: serde_json::json!({
                            "go_id": new_term.go_id,
                            "name": new_term.name,
                            "namespace": new_term.namespace.as_str()
                        }),
                    }],
                ))
            }
            Some(prev) => {
                let mut changes = Vec::new();

                // Check for obsolescence (MAJOR bump)
                if !prev.is_obsolete && new_term.is_obsolete {
                    changes.push(ChangelogEntry {
                        change_type: ChangeType::Obsoleted,
                        category: "go_term".to_string(),
                        description: format!("GO term {} marked as obsolete", new_term.go_id),
                        metadata: serde_json::json!({"go_id": new_term.go_id}),
                    });
                }

                // Check for definition change (MINOR bump)
                if prev.definition != new_term.definition {
                    changes.push(ChangelogEntry {
                        change_type: ChangeType::Modified,
                        category: "definition".to_string(),
                        description: format!("Definition changed for {}", new_term.go_id),
                        metadata: serde_json::json!({
                            "go_id": new_term.go_id,
                            "old_definition": prev.definition,
                            "new_definition": new_term.definition
                        }),
                    });
                }

                // Check for name change (MINOR bump)
                if prev.name != new_term.name {
                    changes.push(ChangelogEntry {
                        change_type: ChangeType::Modified,
                        category: "name".to_string(),
                        description: format!("Name changed for {}", new_term.go_id),
                        metadata: serde_json::json!({
                            "go_id": new_term.go_id,
                            "old_name": prev.name,
                            "new_name": new_term.name
                        }),
                    });
                }

                // Determine bump type based on versioning strategy
                let bump_type = self.calculate_bump_type(&changes).await?;

                Ok(VersionChangelog::new(bump_type, changes))
            }
        }
    }
}
```

### 5.2 Change Detection Workflow

**During ingestion**:

```rust
// In pipeline.rs
pub async fn run_ontology(&self, internal_version: &str) -> Result<PipelineStats> {
    // 1. Download and parse OBO
    let parsed = GoParser::parse_obo(&obo_content, &go_release_version, limit)?;

    // 2. For each term, detect changes
    let detector = GeneOntologyBumpDetector::new(self.organization_id);

    for term in &parsed.terms {
        // Get data_source_id for this GO term
        let data_source_id = storage.get_or_create_go_term_data_source(&term).await?;

        // Detect changes
        let changelog = detector.detect_changes(&self.db, data_source_id, term).await?;

        // Calculate next version
        let next_version = calculate_next_version(&self.db, data_source_id, &changelog).await?;

        // Store with new version
        storage.store_go_term_versioned(&term, &next_version).await?;

        // If MAJOR bump, cascade to dependents (proteins annotated with this term)
        if matches!(changelog.bump_type, BumpType::Major) {
            cascade_version_bump(&self.db, data_source_id, &next_version).await?;
        }
    }

    Ok(stats)
}
```

---

## 6. Bundle Creation

### 6.1 Namespace Bundles

Create bundle data sources for each GO namespace:

**Bundles**:
- `go-biological-process` - All BP terms
- `go-molecular-function` - All MF terms
- `go-cellular-component` - All CC terms
- `go-all` - All GO terms

**Implementation**:

```rust
impl GoStorage {
    /// Create namespace bundles after term ingestion
    pub async fn create_namespace_bundles(&self) -> Result<()> {
        info!("Creating GO namespace bundles");

        // Get all GO term data_source_ids by namespace
        let bp_terms = self.get_go_terms_by_namespace("biological_process").await?;
        let mf_terms = self.get_go_terms_by_namespace("molecular_function").await?;
        let cc_terms = self.get_go_terms_by_namespace("cellular_component").await?;

        // Create bundles
        self.create_bundle("go-biological-process", "GO Biological Process Terms", &bp_terms).await?;
        self.create_bundle("go-molecular-function", "GO Molecular Function Terms", &mf_terms).await?;
        self.create_bundle("go-cellular-component", "GO Cellular Component Terms", &cc_terms).await?;

        // Create go-all bundle
        let all_terms: Vec<Uuid> = bp_terms
            .iter()
            .chain(mf_terms.iter())
            .chain(cc_terms.iter())
            .copied()
            .collect();
        self.create_bundle("go-all", "All GO Terms", &all_terms).await?;

        Ok(())
    }
}
```

---

## 7. Migration Strategy

### 7.1 Data Migration

**Migration Script**: `scripts/migrate_go_to_individual.sql`

```sql
-- ============================================================================
-- Step 1: Create individual data sources for existing GO terms
-- ============================================================================

DO $$
DECLARE
    go_org_id UUID;
    term_record RECORD;
    new_entry_id UUID;
    new_version_id UUID;
BEGIN
    -- Get Gene Ontology organization ID
    SELECT id INTO go_org_id
    FROM organizations
    WHERE slug = 'gene-ontology-consortium';

    -- For each existing GO term in go_term_metadata
    FOR term_record IN
        SELECT * FROM go_term_metadata
        ORDER BY go_accession
    LOOP
        -- 1. Create registry_entry
        INSERT INTO registry_entries (
            organization_id, slug, name, description, entry_type
        )
        VALUES (
            go_org_id,
            term_record.go_id,
            format('%s (%s)', term_record.name, term_record.go_id),
            format('GO Term: %s - %s', term_record.go_id, term_record.namespace),
            'data_source'
        )
        ON CONFLICT (slug) DO NOTHING
        RETURNING id INTO new_entry_id;

        -- If conflict (already exists), get the ID
        IF new_entry_id IS NULL THEN
            SELECT id INTO new_entry_id
            FROM registry_entries
            WHERE slug = term_record.go_id;
        END IF;

        -- 2. Create data_source
        INSERT INTO data_sources (id, source_type)
        VALUES (new_entry_id, 'go_term')
        ON CONFLICT (id) DO NOTHING;

        -- 3. Update go_term_metadata to point to new data_source_id
        UPDATE go_term_metadata
        SET data_source_id = new_entry_id
        WHERE id = term_record.id;

        -- 4. Create version (1.0.0 for existing terms)
        INSERT INTO versions (
            registry_entry_id, version_string, status,
            version_major, version_minor, version_patch
        )
        VALUES (
            new_entry_id, '1.0.0', 'published',
            1, 0, 0
        )
        ON CONFLICT (registry_entry_id, version_string) DO NOTHING
        RETURNING id INTO new_version_id;

        -- 5. Create version_files (TODO: generate OBO/JSON/TSV files)
        -- This would happen in Rust code during next ingestion
    END LOOP;

    RAISE NOTICE 'Migration completed';
END $$;

-- ============================================================================
-- Step 2: Update go_relationships to use data_source_ids
-- ============================================================================

UPDATE go_relationships r
SET
    subject_data_source_id = (
        SELECT data_source_id
        FROM go_term_metadata
        WHERE go_id = r.subject_go_id
        LIMIT 1
    ),
    object_data_source_id = (
        SELECT data_source_id
        FROM go_term_metadata
        WHERE go_id = r.object_go_id
        LIMIT 1
    );

-- ============================================================================
-- Step 3: Update go_annotations to use go_term_data_source_id
-- ============================================================================

UPDATE go_annotations a
SET go_term_data_source_id = (
    SELECT data_source_id
    FROM go_term_metadata
    WHERE go_id = a.go_id
    LIMIT 1
);
```

### 7.2 Rollback Plan

If migration fails:

```sql
-- Rollback script
BEGIN;

-- 1. Remove new columns
ALTER TABLE go_relationships
    DROP COLUMN IF EXISTS subject_data_source_id,
    DROP COLUMN IF EXISTS object_data_source_id;

ALTER TABLE go_annotations
    DROP COLUMN IF EXISTS go_term_data_source_id;

-- 2. Delete individual GO term registry_entries/data_sources
DELETE FROM registry_entries
WHERE slug LIKE 'GO:%'
  AND organization_id = (SELECT id FROM organizations WHERE slug = 'gene-ontology-consortium');

-- 3. Restore original unique constraint
ALTER TABLE go_term_metadata
    DROP CONSTRAINT IF EXISTS unique_go_term_per_data_source,
    ADD CONSTRAINT unique_go_term_per_version UNIQUE (go_id, go_release_version);

COMMIT;
```

---

## 8. Performance Considerations

### 8.1 Batch Operations

**Challenge**: Creating ~47,000 individual data sources efficiently.

**Solution**: Use batch operations like NCBI Taxonomy:

```rust
// Process in chunks of 500 terms
const CHUNK_SIZE: usize = 500;

for chunk in terms.chunks(CHUNK_SIZE) {
    // Batch insert registry_entries (500 at a time)
    batch_insert_registry_entries(tx, chunk).await?;

    // Batch insert data_sources (500 at a time)
    batch_insert_data_sources(tx, chunk).await?;

    // Batch insert go_term_metadata (500 at a time)
    batch_insert_metadata(tx, chunk).await?;

    // Batch insert versions (500 at a time)
    batch_insert_versions(tx, chunk).await?;
}
```

**Estimated Performance**:
- Old (N+1 queries): ~6 queries × 47,000 terms = **282,000 queries** (~45 minutes)
- New (batch): ~10 queries × 94 chunks = **940 queries** (~2 minutes)

**Improvement**: ~27x faster

### 8.2 S3 Upload Strategy

**Challenge**: Uploading ~141,000 files (3 formats × 47,000 terms) to S3.

**Solution**: Parallel uploads with batching:

```rust
// Upload in parallel batches of 50
const S3_BATCH_SIZE: usize = 50;

for chunk in terms.chunks(S3_BATCH_SIZE) {
    let upload_futures: Vec<_> = chunk
        .iter()
        .map(|term| async {
            upload_go_term_files(s3, term).await
        })
        .collect();

    futures::future::join_all(upload_futures).await;
}
```

**Estimated Time**:
- Sequential: 141,000 uploads × 100ms = **3.9 hours**
- Parallel (50 concurrent): 141,000 / 50 × 100ms = **4.7 minutes**

**Improvement**: ~50x faster

### 8.3 Database Indexes

**Required indexes** (already exist in migration):
- `go_term_metadata(data_source_id)` - UNIQUE
- `go_term_metadata(go_id, go_release_version)` - UNIQUE
- `go_relationships(subject_data_source_id)`
- `go_relationships(object_data_source_id)`
- `go_annotations(go_term_data_source_id)`

---

## 9. Testing Strategy

### 9.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_go_term_as_data_source() {
        // Setup: Create test GO term
        let term = GoTerm::new(
            "GO:0008150".to_string(),
            "biological_process".to_string(),
            Namespace::BiologicalProcess,
            "2026-01-01".to_string(),
        ).unwrap();

        // Act: Store as data source
        let storage = GoStorage::new(pool, org_id, "1.0.0", "2026-01-01");
        let result = storage.store_go_term_as_data_source(&term).await;

        // Assert
        assert!(result.is_ok());

        // Verify registry_entry exists
        let entry = get_registry_entry_by_slug(&pool, "GO:0008150").await.unwrap();
        assert_eq!(entry.name, "biological_process (GO:0008150)");

        // Verify data_source exists
        let ds = get_data_source_by_id(&pool, entry.id).await.unwrap();
        assert_eq!(ds.source_type, "go_term");

        // Verify go_term_metadata linked
        let metadata = get_go_term_metadata(&pool, ds.id).await.unwrap();
        assert_eq!(metadata.go_id, "GO:0008150");
    }

    #[tokio::test]
    async fn test_version_bump_on_obsolescence() {
        // Setup: Store GO term at v1.0.0
        let term_v1 = GoTerm::new(...).unwrap();
        storage.store_go_term(&term_v1).await.unwrap();

        // Act: Mark as obsolete and re-ingest
        let mut term_v2 = term_v1.clone();
        term_v2.is_obsolete = true;
        storage.store_go_term(&term_v2).await.unwrap();

        // Assert: Version bumped to 2.0.0 (MAJOR)
        let latest_version = get_latest_version(&pool, "GO:0008150").await.unwrap();
        assert_eq!(latest_version.version_string, "2.0.0");
        assert_eq!(latest_version.version_major, 2);
    }
}
```

### 9.2 Integration Tests

```rust
#[tokio::test]
async fn test_full_go_pipeline_individual() {
    // 1. Download real GO OBO file
    let downloader = GoDownloader::new(config).unwrap();
    let obo_content = downloader.download_ontology().await.unwrap();

    // 2. Parse (limit to 100 terms for testing)
    let parsed = GoParser::parse_obo(&obo_content, "2026-01-01", Some(100)).unwrap();

    // 3. Store as individual data sources
    let storage = GoStorage::new(pool, org_id, "1.0.0", "2026-01-01");
    let stats = storage.store_ontology_individual(&parsed.terms, &parsed.relationships).await.unwrap();

    // 4. Verify: 100 registry_entries created
    assert_eq!(stats.terms_stored, 100);

    // 5. Verify: Can query individual terms
    let bp_entry = get_registry_entry_by_slug(&pool, "GO:0008150").await.unwrap();
    assert!(bp_entry.id.is_some());

    // 6. Verify: Relationships linked via data_source_ids
    let relationships = get_go_relationships_for_term(&pool, "GO:0008150").await.unwrap();
    assert!(!relationships.is_empty());
    assert!(relationships[0].subject_data_source_id.is_some());
}
```

---

## 10. User-Facing Changes

### 10.1 CLI Commands

**Old (aggregate)**:
```bash
# Download entire GO ontology as one bundle
bdp source add gene-ontology:go-all@1.0
```

**New (individual)**:
```bash
# Download specific GO terms
bdp source add gene-ontology:GO:0008150@1.0  # biological_process
bdp source add gene-ontology:GO:0003674@1.0  # molecular_function

# Download namespace bundles
bdp source add gene-ontology:go-biological-process@1.0
bdp source add gene-ontology:go-molecular-function@1.0
bdp source add gene-ontology:go-cellular-component@1.0

# Download entire ontology (bundle of all terms)
bdp source add gene-ontology:go-all@2.0
```

### 10.2 API Endpoints

**New endpoints**:

```typescript
// Get individual GO term
GET /api/sources/gene-ontology/GO:0008150/1.0

// Download GO term files
GET /api/sources/gene-ontology/GO:0008150/1.0/files/obo
GET /api/sources/gene-ontology/GO:0008150/1.0/files/json
GET /api/sources/gene-ontology/GO:0008150/1.0/files/tsv

// List GO terms by namespace
GET /api/sources/gene-ontology/terms?namespace=biological_process&version=1.0

// Get GO term version history
GET /api/sources/gene-ontology/GO:0008150/versions

// Get GO term changelog
GET /api/sources/gene-ontology/GO:0008150/1.0/changelog
```

### 10.3 Web UI

**GO Term Detail Page** (`/sources/gene-ontology/GO:0008150/1.0`):

```
┌─────────────────────────────────────────────────┐
│ GO:0008150 - biological_process                 │
│ Version 1.2.0                                    │
├─────────────────────────────────────────────────┤
│ Namespace: Biological Process                   │
│ Definition: A biological process is...          │
│ Status: Active                                   │
│                                                  │
│ [Download OBO] [Download JSON] [Download TSV]   │
│                                                  │
│ Version History:                                 │
│ • 1.2.0 (2026-03-01) - Synonym added            │
│ • 1.1.0 (2026-02-15) - Definition refined       │
│ • 1.0.0 (2026-01-01) - Initial version          │
│                                                  │
│ Relationships:                                   │
│ • is_a: GO:0003674 (molecular_function)         │
│                                                  │
│ Annotated Proteins: 12,345                      │
└─────────────────────────────────────────────────┘
```

---

## 11. Documentation Updates

### 11.1 User Documentation

**File**: `docs/gene-ontology-usage.md`

```markdown
# Gene Ontology in BDP

## Overview

BDP treats each Gene Ontology (GO) term as an individual data source, allowing:
- Version tracking for each term
- Change detection (obsolescence, definition updates)
- Individual downloads
- Citation tracking

## Downloading GO Terms

### Individual Terms

```bash
# Download specific GO term
bdp source add gene-ontology:GO:0008150@1.0
```

### Namespace Bundles

```bash
# Download all Biological Process terms
bdp source add gene-ontology:go-biological-process@1.0

# Download all Molecular Function terms
bdp source add gene-ontology:go-molecular-function@1.0
```

### Complete Ontology

```bash
# Download entire GO (all ~47,000 terms)
bdp source add gene-ontology:go-all@1.0
```

## Versioning

GO terms follow semantic versioning:

| Change | Bump Type | Example |
|--------|-----------|---------|
| Term obsoleted | MAJOR | 1.0.0 → 2.0.0 |
| Definition changed | MINOR | 1.0.0 → 1.1.0 |
| Synonym added | MINOR | 1.0.0 → 1.1.0 |

## File Formats

Each GO term provides 3 formats:

- **OBO**: Standard GO format
- **JSON**: Structured data
- **TSV**: Tab-separated values

```bash
# Extract specific format
bdp pull gene-ontology:GO:0008150@1.0 --format obo
bdp pull gene-ontology:GO:0008150@1.0 --format json
```
```

### 11.2 Developer Documentation

**File**: `docs/agents/go-implementation.md`

```markdown
# Gene Ontology Implementation

## Architecture

GO terms are stored as individual data sources following the UniProt/NCBI pattern:

```
registry_entry (GO:0008150)
  └─ data_source (source_type: go_term)
      └─ go_term_metadata (name, definition, namespace, ...)
      └─ version (1.0.0, 1.1.0, 2.0.0)
          └─ version_files (OBO, JSON, TSV)
```

## Storage Flow

1. **Parse OBO** → GoTerm structs
2. **Detect Changes** → Version changelog
3. **Calculate Version** → Semantic version bump
4. **Store** → registry_entry + data_source + metadata + version
5. **Upload Files** → S3 (OBO, JSON, TSV)
6. **Create Bundles** → Namespace bundles + go-all

## Change Detection

See: `crates/bdp-server/src/ingest/versioning/detector.rs`

```rust
impl VersionBumpDetector for GeneOntologyBumpDetector {
    async fn detect_changes(...) -> Result<VersionChangelog> {
        // Compare old vs new term
        // Return MAJOR if obsoleted
        // Return MINOR if definition/name changed
    }
}
```
```

---

## 12. Implementation Checklist

### Phase 1: Schema Migration
- [ ] Write migration `20260128000001_go_individual_sources.sql`
- [ ] Add `subject_data_source_id`, `object_data_source_id` to `go_relationships`
- [ ] Add `go_term_data_source_id` to `go_annotations`
- [ ] Update constraints and indexes
- [ ] Test migration on dev database

### Phase 2: Storage Refactoring
- [ ] Implement `store_ontology_individual()` in `storage.rs`
- [ ] Implement batch operations (like NCBI Taxonomy)
  - [ ] `batch_upsert_registry_entries()`
  - [ ] `batch_insert_data_sources()`
  - [ ] `batch_upsert_go_term_metadata()`
  - [ ] `batch_insert_versions()`
  - [ ] `batch_insert_version_files()`
- [ ] Implement `store_relationships_individual()`
- [ ] Add data_source_id lookup methods
- [ ] Update `create_version_files_tx()` for GO formats

### Phase 3: Version Detection
- [ ] Implement `GeneOntologyBumpDetector` in `versioning/detector.rs`
- [ ] Add change detection for:
  - [ ] Obsolescence (MAJOR)
  - [ ] Definition changes (MINOR)
  - [ ] Name changes (MINOR)
  - [ ] Synonym changes (MINOR)
  - [ ] Relationship changes (MINOR/MAJOR)
- [ ] Add versioning strategy to Gene Ontology organization

### Phase 4: Bundle Creation
- [ ] Implement `create_namespace_bundles()`
- [ ] Create `go-biological-process` bundle
- [ ] Create `go-molecular-function` bundle
- [ ] Create `go-cellular-component` bundle
- [ ] Create `go-all` bundle

### Phase 5: Pipeline Updates
- [ ] Update `pipeline.rs::run_ontology()` to use individual storage
- [ ] Add version detection loop
- [ ] Add cascade logic for MAJOR bumps
- [ ] Test on small OBO subset (100 terms)

### Phase 6: Data Migration
- [ ] Write `scripts/migrate_go_to_individual.sql`
- [ ] Run migration on staging database
- [ ] Verify data integrity
- [ ] Update `go_relationships` with data_source_ids
- [ ] Update `go_annotations` with go_term_data_source_id

### Phase 7: Testing
- [ ] Unit tests for storage methods
- [ ] Unit tests for version detection
- [ ] Integration test: full pipeline (100 terms)
- [ ] Integration test: relationship linking
- [ ] Integration test: bundle creation
- [ ] Performance test: 47,000 terms (full ontology)

### Phase 8: Documentation
- [ ] Write user guide: `docs/gene-ontology-usage.md`
- [ ] Write developer guide: `docs/agents/go-implementation.md`
- [ ] Update ROADMAP.md with GO individual sources
- [ ] Add changelog entry

### Phase 9: Deployment
- [ ] Deploy schema migration to production
- [ ] Run data migration script
- [ ] Re-ingest GO ontology with new pipeline
- [ ] Verify web UI displays GO terms correctly
- [ ] Test CLI download commands

---

## 13. Risk Assessment

### 13.1 High Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Data migration failure** | Data loss, downtime | Full backup before migration, rollback script tested |
| **Performance degradation** | Slow queries, timeouts | Use batch operations, add indexes, test on staging |
| **Breaking API changes** | Frontend breaks | Keep legacy `go_id` columns during transition |

### 13.2 Medium Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| **S3 upload failures** | Missing files | Retry logic, parallel uploads |
| **Version cascade issues** | Incorrect version bumps | Comprehensive tests for cascade logic |
| **Relationship linking errors** | Broken GO graph | Validate data_source_id lookups |

### 13.3 Low Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Bundle creation fails** | Missing bundles | Create bundles as separate step |
| **OBO format generation** | Invalid files | Validate against GO spec |

---

## 14. Success Criteria

### 14.1 Functional Requirements

- [ ] Each GO term is a separate data source with unique slug (e.g., `GO:0008150`)
- [ ] Each GO term has independent versioning (1.0.0, 1.1.0, 2.0.0, ...)
- [ ] Version bumps follow semantic versioning rules
- [ ] Obsolescence triggers MAJOR bump and cascades to dependent proteins
- [ ] Definition/name changes trigger MINOR bump
- [ ] All GO terms downloadable individually (OBO, JSON, TSV formats)
- [ ] Namespace bundles created (BP, MF, CC, ALL)

### 14.2 Performance Requirements

- [ ] Full ontology ingestion completes in <5 minutes (47,000 terms)
- [ ] S3 upload completes in <10 minutes (141,000 files)
- [ ] Individual GO term queries return in <100ms
- [ ] Relationship traversal queries return in <200ms

### 14.3 Data Integrity Requirements

- [ ] Zero data loss during migration
- [ ] All GO term metadata preserved (definitions, synonyms, xrefs)
- [ ] All relationships preserved (is_a, part_of, regulates)
- [ ] All protein annotations preserved (GO ID → protein mappings)

---

## 15. Timeline Estimate

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| **Phase 1**: Schema Migration | 1 day | None |
| **Phase 2**: Storage Refactoring | 3 days | Phase 1 |
| **Phase 3**: Version Detection | 2 days | Phase 2 |
| **Phase 4**: Bundle Creation | 1 day | Phase 2 |
| **Phase 5**: Pipeline Updates | 2 days | Phases 2-4 |
| **Phase 6**: Data Migration | 1 day | Phases 1-5 |
| **Phase 7**: Testing | 3 days | Phases 1-6 |
| **Phase 8**: Documentation | 1 day | Phases 1-7 |
| **Phase 9**: Deployment | 1 day | Phases 1-8 |
| **Total** | **15 days** | - |

---

## 16. Appendix

### 16.1 References

- **GO Citation Policy**: https://geneontology.org/docs/go-citation-policy/
- **GO OBO Format Spec**: https://owlcollab.github.io/oboformat/doc/GO.format.obo-1_4.html
- **UniProt Storage Pattern**: `crates/bdp-server/src/ingest/uniprot/storage.rs`
- **NCBI Taxonomy Pattern**: `crates/bdp-server/src/ingest/ncbi_taxonomy/storage.rs`
- **Versioning Module**: `crates/bdp-server/src/ingest/versioning/`

### 16.2 Sample Queries

**Get all GO terms in Biological Process namespace**:
```sql
SELECT re.slug, tm.name, tm.definition
FROM registry_entries re
JOIN data_sources ds ON ds.id = re.id
JOIN go_term_metadata tm ON tm.data_source_id = ds.id
WHERE ds.source_type = 'go_term'
  AND tm.namespace = 'biological_process'
ORDER BY tm.go_accession;
```

**Get all proteins annotated with GO:0008150**:
```sql
SELECT pm.accession, pm.protein_name
FROM go_annotations ga
JOIN protein_metadata pm ON pm.data_source_id = ga.entity_id
WHERE ga.go_term_data_source_id = (
    SELECT data_source_id
    FROM go_term_metadata
    WHERE go_id = 'GO:0008150'
)
  AND ga.entity_type = 'protein';
```

**Get GO term version history**:
```sql
SELECT v.version_string, v.status, v.created_at, cl.summary
FROM versions v
JOIN registry_entries re ON re.id = v.registry_entry_id
LEFT JOIN version_changelogs cl ON cl.version_id = v.id
WHERE re.slug = 'GO:0008150'
ORDER BY v.version_major DESC, v.version_minor DESC, v.version_patch DESC;
```

### 16.3 GO Statistics

| Metric | Value |
|--------|-------|
| Total GO terms (2026-01-01) | ~47,000 |
| Biological Process terms | ~30,000 |
| Molecular Function terms | ~12,000 |
| Cellular Component terms | ~5,000 |
| Total relationships | ~100,000 |
| Total protein annotations (UniProt) | ~700 million |

---

**End of Design Document**
