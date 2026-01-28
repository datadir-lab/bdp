# InterPro Individual Data Sources Design

**Date**: 2026-01-28
**Status**: Design Phase - Ready for Implementation
**Pattern**: Individual Data Sources (matching UniProt, NCBI Taxonomy, GenBank)

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [Data Source Structure](#data-source-structure)
4. [Versioning Strategy](#versioning-strategy)
5. [Metadata Structure](#metadata-structure)
6. [Cross-Reference Architecture](#cross-reference-architecture)
7. [Schema Design](#schema-design)
8. [Storage Implementation](#storage-implementation)
9. [Version File Formats](#version-file-formats)
10. [Change Detection](#change-detection)
11. [Performance Optimization](#performance-optimization)
12. [Implementation Plan](#implementation-plan)

---

## Executive Summary

### Design Philosophy

**Each InterPro entry becomes an individual data source**, following the established pattern of UniProt (570K proteins), NCBI Taxonomy (2.4M taxa), and GenBank (millions of sequences).

### Key Numbers

- **~40,000 InterPro entries** → 40,000 individual data sources
- Each entry references **hundreds to thousands** of UniProt proteins
- **~200M protein-domain matches** total across all entries
- Release frequency: Every **8 weeks** (coordinated with UniProt)

### Pattern Consistency

```
UniProt:    1 protein  = 1 registry_entry = 1 data_source + versions
Taxonomy:   1 taxon    = 1 registry_entry = 1 data_source + versions
GenBank:    1 sequence = 1 registry_entry = 1 data_source + versions
GO Term:    1 term     = 1 registry_entry = 1 data_source + versions (PLANNED)
InterPro:   1 entry    = 1 registry_entry = 1 data_source + versions (THIS DESIGN)
```

---

## Architecture Overview

### Individual Data Source Model

Each InterPro entry (e.g., `IPR000001` - "Kringle") becomes a standalone, versioned data source:

```
Registry Entry:
  slug: "IPR000001"
  name: "Kringle"
  organization: interpro
  entry_type: data_source

Data Source:
  id: <uuid>
  registry_entry_id: <uuid>
  source_type: "interpro_entry"

InterPro Metadata:
  interpro_id: "IPR000001"
  entry_type: "Domain"
  name: "Kringle"
  short_name: "Kringle"
  description: "Kringles are autonomous structural domains..."

Version:
  version: "1.0.0" (internal semantic)
  external_version: "103.0" (InterPro release)

Version Files:
  - IPR000001-1.0.0-matches.tsv.gz
  - IPR000001-1.0.0-matches.json.gz
  - IPR000001-1.0.0-metadata.json
```

### Why Individual Sources?

1. **Granular Versioning**: `IPR000001` can bump to v2.0 independently of `IPR000002`
2. **User Experience**: `bdp source add interpro:IPR000001@1.0` (specific domain)
3. **Pattern Consistency**: Matches 3 of 4 existing pipelines
4. **Dependency Tracking**: Each entry tracks its protein dependencies independently
5. **Change Detection**: Detect when specific domains change, not just "InterPro updated"

---

## Data Source Structure

### Registry Entry Pattern

Following `uniprot/storage.rs::create_registry_entry_tx()`:

```rust
// Each InterPro entry gets its own registry entry
async fn create_registry_entry_tx(
    tx: &mut Transaction<'_, Postgres>,
    interpro_entry: &InterProEntry,
    organization_id: Uuid,
) -> Result<Uuid> {
    let slug = &interpro_entry.interpro_id; // "IPR000001"
    let name = &interpro_entry.name;        // "Kringle"
    let description = format!(
        "InterPro {}: {}",
        interpro_entry.entry_type,
        interpro_entry.description
    );

    let entry_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO registry_entries (
            organization_id,
            slug,
            name,
            description,
            entry_type
        )
        VALUES ($1, $2, $3, $4, 'data_source')
        ON CONFLICT (slug)
        DO UPDATE SET
            name = EXCLUDED.name,
            description = EXCLUDED.description
        RETURNING id
        "#
    )
    .bind(organization_id)
    .bind(slug)
    .bind(name)
    .bind(&description)
    .fetch_one(&mut **tx)
    .await?;

    Ok(entry_id)
}
```

### Data Source Creation

```rust
async fn create_data_source_tx(
    tx: &mut Transaction<'_, Postgres>,
    entry_id: Uuid,
    interpro_entry: &InterProEntry,
) -> Result<Uuid> {
    let data_source_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO data_sources (
            registry_entry_id,
            source_type,
            external_id,
            metadata
        )
        VALUES ($1, 'interpro_entry', $2, $3)
        ON CONFLICT (registry_entry_id, external_id)
        DO UPDATE SET metadata = EXCLUDED.metadata
        RETURNING id
        "#
    )
    .bind(entry_id)
    .bind(&interpro_entry.interpro_id)
    .bind(json!({
        "entry_type": interpro_entry.entry_type,
        "short_name": interpro_entry.short_name,
    }))
    .fetch_one(&mut **tx)
    .await?;

    Ok(data_source_id)
}
```

---

## Versioning Strategy

### Semantic Versioning Rules

Each InterPro entry versions **independently** based on its own changes:

#### MAJOR Bumps (Breaking Changes)

| Trigger | Description | Example |
|---------|-------------|---------|
| **Entry Obsoleted** | InterPro entry marked as obsolete | `IPR000001` deprecated → `2.0.0` |
| **Signature Removed** | Major member database signature removed | Pfam signature PF00051 removed → `2.0.0` |
| **Type Changed** | Entry type changed (Domain → Family) | `"Domain"` → `"Family"` → `2.0.0` |
| **Massive Protein Loss** | >50% of proteins lost from matches | 10,000 → 4,000 proteins → `2.0.0` |

#### MINOR Bumps (Non-Breaking Changes)

| Trigger | Description | Example |
|---------|-------------|---------|
| **Proteins Added** | New proteins matched this entry | 1,000 → 1,200 proteins → `1.1.0` |
| **Description Updated** | Entry description/definition changed | Text updated → `1.1.0` |
| **Signature Added** | New member database signature added | SMART signature added → `1.1.0` |
| **Metadata Updated** | GO mappings, names, cross-refs updated | GO:0008150 added → `1.1.0` |
| **Proteins Lost (Minor)** | <10% of proteins lost (not breaking) | 1,000 → 950 proteins → `1.1.0` |

#### External → Internal Version Mapping

```
InterPro Release 103.0 (external)
  ↓
IPR000001: 1.0.0 (internal) - First ingestion
IPR000002: 1.0.0 (internal) - First ingestion
...

InterPro Release 104.0 (external)
  ↓ (change detection)
IPR000001: 1.1.0 (internal) - Minor change (100 proteins added)
IPR000002: 1.0.0 (internal) - No change
IPR000003: 1.0.0 (internal) - New entry
IPR012345: 2.0.0 (internal) - Major change (obsoleted)
```

### Versioning Strategy Definition

```rust
impl VersioningStrategy {
    pub fn interpro() -> Self {
        Self {
            major_triggers: vec![
                VersionTrigger::new(
                    VersionChangeType::Removed,
                    "entry",
                    "InterPro entry marked as obsolete or removed"
                ),
                VersionTrigger::new(
                    VersionChangeType::Removed,
                    "signatures",
                    "Major member database signature removed (e.g., Pfam domain)"
                ),
                VersionTrigger::new(
                    VersionChangeType::Modified,
                    "entry_type",
                    "Entry type changed (Domain → Family, etc.)"
                ),
                VersionTrigger::new(
                    VersionChangeType::Removed,
                    "proteins_major",
                    "Massive protein loss (>50% of matches removed)"
                ),
            ],
            minor_triggers: vec![
                VersionTrigger::new(
                    VersionChangeType::Added,
                    "proteins",
                    "New proteins matched this InterPro entry"
                ),
                VersionTrigger::new(
                    VersionChangeType::Modified,
                    "description",
                    "Entry description or definition updated"
                ),
                VersionTrigger::new(
                    VersionChangeType::Added,
                    "signatures",
                    "New member database signature added (e.g., SMART domain)"
                ),
                VersionTrigger::new(
                    VersionChangeType::Modified,
                    "metadata",
                    "GO mappings, synonyms, or cross-references updated"
                ),
                VersionTrigger::new(
                    VersionChangeType::Removed,
                    "proteins_minor",
                    "Minor protein loss (<10% of matches removed)"
                ),
            ],
            default_bump: BumpType::Minor,
            cascade_on_major: false,  // InterPro changes don't cascade to proteins
            cascade_on_minor: false,  // Proteins depend on InterPro, not vice versa
        }
    }
}
```

### Change Detection Implementation

```rust
pub struct InterProBumpDetector;

#[async_trait]
impl VersionBumpDetector for InterProBumpDetector {
    async fn detect_changes(
        &self,
        pool: &PgPool,
        current_version_id: Option<Uuid>,
        new_data: &serde_json::Value,
    ) -> Result<VersionChangelog> {
        let new_entry: InterProEntry = serde_json::from_value(new_data.clone())?;

        if current_version_id.is_none() {
            // First version - initial ingestion
            return Ok(VersionChangelog::new(
                BumpType::Minor,
                vec![ChangelogEntry::added(
                    "entry",
                    1,
                    format!("Initial ingestion of {}", new_entry.name),
                )],
                ChangelogSummary::initial(1),
                format!("Initial version of {} ({})", new_entry.name, new_entry.interpro_id),
            ));
        }

        // Get previous version data
        let prev_entry = self.get_previous_entry(pool, current_version_id.unwrap()).await?;

        let mut entries = Vec::new();

        // Check for entry obsolescence
        if new_entry.is_obsolete && !prev_entry.is_obsolete {
            entries.push(ChangelogEntry::new(
                ChangeType::Removed,
                "entry",
                format!("Entry {} marked as obsolete", new_entry.interpro_id),
                true, // Breaking change
            ));
        }

        // Check for entry type changes
        if new_entry.entry_type != prev_entry.entry_type {
            entries.push(ChangelogEntry::new(
                ChangeType::Modified,
                "entry_type",
                format!(
                    "Entry type changed from {} to {}",
                    prev_entry.entry_type,
                    new_entry.entry_type
                ),
                true, // Breaking change
            ));
        }

        // Check for protein match changes
        let prev_protein_count = self.get_protein_count(pool, current_version_id.unwrap()).await?;
        let new_protein_count = new_entry.protein_matches.len() as i64;

        let proteins_added = new_protein_count - prev_protein_count;
        let protein_loss_pct = if prev_protein_count > 0 {
            ((prev_protein_count - new_protein_count) as f64 / prev_protein_count as f64) * 100.0
        } else {
            0.0
        };

        if proteins_added > 0 {
            entries.push(ChangelogEntry::added(
                "proteins",
                proteins_added,
                format!("{} new protein matches added", proteins_added),
            ));
        } else if proteins_added < 0 {
            let is_breaking = protein_loss_pct > 50.0;
            entries.push(ChangelogEntry::with_count(
                ChangeType::Removed,
                if is_breaking { "proteins_major" } else { "proteins_minor" },
                proteins_added.abs(),
                format!(
                    "{} protein matches removed ({:.1}% loss)",
                    proteins_added.abs(),
                    protein_loss_pct
                ),
                is_breaking,
            ));
        }

        // Check for description changes
        if new_entry.description != prev_entry.description {
            entries.push(ChangelogEntry::modified(
                "description",
                1,
                "Entry description updated",
                false,
            ));
        }

        // Determine bump type from entries
        let bump_type = VersionChangelog::determine_bump_type(&entries);

        let summary = ChangelogSummary::new(
            prev_protein_count,
            new_protein_count,
            proteins_added.max(0),
            proteins_added.abs().min(prev_protein_count),
            0, // modified count
            TriggerReason::NewRelease,
        );

        let summary_text = format!(
            "InterPro {} updated: {} total proteins ({:+} change)",
            new_entry.interpro_id,
            new_protein_count,
            proteins_added
        );

        Ok(VersionChangelog::new(
            bump_type,
            entries,
            summary,
            summary_text,
        ))
    }
}
```

---

## Metadata Structure

### Primary Metadata Table

```sql
CREATE TABLE interpro_entry_metadata (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,

    -- Core InterPro fields
    interpro_id VARCHAR(20) NOT NULL UNIQUE,  -- IPR000001
    entry_type VARCHAR(50) NOT NULL,          -- Family, Domain, Repeat, Site, etc.
    name TEXT NOT NULL,                       -- "Kringle"
    short_name VARCHAR(255),                  -- "Kringle"
    description TEXT,                         -- Full description

    -- Status
    is_obsolete BOOLEAN DEFAULT FALSE,
    replacement_interpro_id VARCHAR(20),      -- If obsoleted, what replaces it?

    -- Member database integration
    member_databases JSONB,                   -- {"Pfam": ["PF00051"], "SMART": ["SM00130"]}

    -- GO term mappings
    go_mappings JSONB,                        -- [{"go_id": "GO:0005515", "go_name": "protein binding"}]

    -- Cross-references
    cross_references JSONB,                   -- {"PDB": ["1KRI"], "PROSITE": ["PS50070"]}

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT fk_interpro_data_source FOREIGN KEY (data_source_id) REFERENCES data_sources(id) ON DELETE CASCADE
);

CREATE INDEX idx_interpro_entry_metadata_data_source ON interpro_entry_metadata(data_source_id);
CREATE INDEX idx_interpro_entry_metadata_interpro_id ON interpro_entry_metadata(interpro_id);
CREATE INDEX idx_interpro_entry_metadata_entry_type ON interpro_entry_metadata(entry_type);
CREATE INDEX idx_interpro_entry_metadata_obsolete ON interpro_entry_metadata(is_obsolete);
```

### Protein Match Table (Cross-References)

This table links **InterPro entries → UniProt proteins**:

```sql
CREATE TABLE protein_interpro_matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Source: InterPro entry (the domain/family)
    interpro_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    interpro_version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,

    -- Target: UniProt protein
    protein_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    uniprot_accession VARCHAR(20) NOT NULL,  -- Denormalized for fast lookups

    -- Match details
    signature_accession VARCHAR(50),         -- e.g., "PF00051" (Pfam ID)
    signature_database VARCHAR(50),          -- e.g., "Pfam", "SMART", "PROSITE"
    start_position INTEGER NOT NULL,
    end_position INTEGER NOT NULL,
    e_value DOUBLE PRECISION,
    score DOUBLE PRECISION,

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT fk_protein_interpro_interpro_ds FOREIGN KEY (interpro_data_source_id)
        REFERENCES data_sources(id) ON DELETE CASCADE,
    CONSTRAINT fk_protein_interpro_interpro_ver FOREIGN KEY (interpro_version_id)
        REFERENCES versions(id) ON DELETE CASCADE,
    CONSTRAINT fk_protein_interpro_protein_ds FOREIGN KEY (protein_data_source_id)
        REFERENCES data_sources(id) ON DELETE CASCADE
);

-- Critical indexes for performance
CREATE INDEX idx_protein_interpro_interpro_ds ON protein_interpro_matches(interpro_data_source_id);
CREATE INDEX idx_protein_interpro_protein_ds ON protein_interpro_matches(protein_data_source_id);
CREATE INDEX idx_protein_interpro_accession ON protein_interpro_matches(uniprot_accession);
CREATE INDEX idx_protein_interpro_signature ON protein_interpro_matches(signature_accession);
CREATE INDEX idx_protein_interpro_positions ON protein_interpro_matches(start_position, end_position);

-- Composite index for common query patterns
CREATE INDEX idx_protein_interpro_protein_interpro ON protein_interpro_matches(protein_data_source_id, interpro_data_source_id);
```

### JSON Metadata Examples

#### `member_databases` JSONB

```json
{
  "Pfam": ["PF00051"],
  "SMART": ["SM00130"],
  "PROSITE": ["PS50070"],
  "PRINTS": ["PR00008"]
}
```

#### `go_mappings` JSONB

```json
[
  {
    "go_id": "GO:0005515",
    "go_name": "protein binding",
    "evidence": "IEA",
    "namespace": "molecular_function"
  },
  {
    "go_id": "GO:0007596",
    "go_name": "blood coagulation",
    "evidence": "IEA",
    "namespace": "biological_process"
  }
]
```

#### `cross_references` JSONB

```json
{
  "PDB": ["1KRI", "1BUI"],
  "SCOP": ["d1kria_"],
  "CATH": ["2.40.10.10"],
  "Wikipedia": ["Kringle_domain"]
}
```

---

## Cross-Reference Architecture

### Dependency Graph

```
NCBI Taxonomy (organisms)
    ↓ (referenced by)
UniProt Proteins (sequences)
    ↓ (referenced by)
InterPro Entries (domains/families)
    ↓ (referenced by - future)
PDB Structures (3D structures)
```

### UniProt → InterPro Link Pattern

Following your `TaxonomyHelper` pattern (`uniprot/taxonomy_helper.rs`):

```rust
/// Helper for looking up UniProt proteins by accession
///
/// Similar to TaxonomyHelper, this provides cross-reference lookup
/// for InterPro ingestion.
pub struct ProteinLookupHelper {
    db: PgPool,
    cache: HashMap<String, Uuid>, // accession → data_source_id
}

impl ProteinLookupHelper {
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            cache: HashMap::new(),
        }
    }

    /// Look up protein data_source_id by UniProt accession
    ///
    /// Returns None if protein doesn't exist in BDP (not an error)
    pub async fn lookup_protein(&mut self, accession: &str) -> Result<Option<Uuid>> {
        // Check cache first
        if let Some(&id) = self.cache.get(accession) {
            return Ok(Some(id));
        }

        // Query database
        let result = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT ds.id
            FROM data_sources ds
            JOIN protein_metadata pm ON pm.data_source_id = ds.id
            WHERE pm.accession = $1
            "#
        )
        .bind(accession)
        .fetch_optional(&self.db)
        .await?;

        // Cache for future lookups
        if let Some(id) = result {
            self.cache.insert(accession.to_string(), id);
        }

        Ok(result)
    }

    /// Bulk lookup for batch operations (more efficient)
    pub async fn lookup_proteins_bulk(&mut self, accessions: &[String]) -> Result<HashMap<String, Uuid>> {
        // Filter out cached entries
        let uncached: Vec<_> = accessions.iter()
            .filter(|acc| !self.cache.contains_key(*acc))
            .collect();

        if uncached.is_empty() {
            // Everything is cached
            return Ok(accessions.iter()
                .filter_map(|acc| self.cache.get(acc).map(|&id| (acc.clone(), id)))
                .collect());
        }

        // Query database for uncached entries
        let results: Vec<(String, Uuid)> = sqlx::query_as(
            r#"
            SELECT pm.accession, ds.id
            FROM data_sources ds
            JOIN protein_metadata pm ON pm.data_source_id = ds.id
            WHERE pm.accession = ANY($1)
            "#
        )
        .bind(&uncached)
        .fetch_all(&self.db)
        .await?;

        // Update cache
        for (accession, id) in &results {
            self.cache.insert(accession.clone(), *id);
        }

        // Return combined cached + new results
        Ok(accessions.iter()
            .filter_map(|acc| self.cache.get(acc).map(|&id| (acc.clone(), id)))
            .collect())
    }
}
```

### Storage with Cross-References

```rust
impl InterProStorage {
    /// Store matches for an InterPro entry
    async fn store_matches(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        interpro_data_source_id: Uuid,
        interpro_version_id: Uuid,
        matches: &[ProteinMatch],
    ) -> Result<usize> {
        let mut helper = ProteinLookupHelper::new(self.db.clone());

        // Bulk lookup all proteins (efficient)
        let accessions: Vec<String> = matches.iter()
            .map(|m| m.uniprot_accession.clone())
            .collect();

        let protein_map = helper.lookup_proteins_bulk(&accessions).await?;

        let mut stored_count = 0;
        let mut missing_count = 0;

        // Process in batches of 500
        for chunk in matches.chunks(500) {
            let mut values = Vec::new();

            for match_data in chunk {
                // Look up protein
                let protein_ds_id = match protein_map.get(&match_data.uniprot_accession) {
                    Some(id) => *id,
                    None => {
                        warn!(
                            accession = %match_data.uniprot_accession,
                            "Protein not found in BDP - skipping match"
                        );
                        missing_count += 1;
                        continue;
                    }
                };

                values.push((
                    interpro_data_source_id,
                    interpro_version_id,
                    protein_ds_id,
                    &match_data.uniprot_accession,
                    &match_data.signature_accession,
                    &match_data.signature_database,
                    match_data.start_position,
                    match_data.end_position,
                    match_data.e_value,
                    match_data.score,
                ));
            }

            if values.is_empty() {
                continue;
            }

            // Batch insert
            let mut qb = QueryBuilder::new(
                "INSERT INTO protein_interpro_matches (
                    interpro_data_source_id,
                    interpro_version_id,
                    protein_data_source_id,
                    uniprot_accession,
                    signature_accession,
                    signature_database,
                    start_position,
                    end_position,
                    e_value,
                    score
                ) "
            );

            qb.push_values(values, |mut b, match_data| {
                b.push_bind(match_data.0)
                 .push_bind(match_data.1)
                 .push_bind(match_data.2)
                 .push_bind(match_data.3)
                 .push_bind(match_data.4)
                 .push_bind(match_data.5)
                 .push_bind(match_data.6)
                 .push_bind(match_data.7)
                 .push_bind(match_data.8)
                 .push_bind(match_data.9);
            });

            qb.build().execute(&mut **tx).await?;
            stored_count += chunk.len();
        }

        if missing_count > 0 {
            warn!(
                missing = missing_count,
                total = matches.len(),
                "Some proteins not found during InterPro ingestion"
            );
        }

        Ok(stored_count)
    }
}
```

### Dependency Tracking

Create explicit dependency links between InterPro entries and UniProt proteins:

```rust
/// Create dependencies for InterPro entry → UniProt proteins
async fn create_dependencies(
    &self,
    tx: &mut Transaction<'_, Postgres>,
    interpro_version_id: Uuid,
    protein_data_source_ids: &[Uuid],
) -> Result<()> {
    // Get version IDs for all referenced proteins
    let version_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT v.id
        FROM versions v
        WHERE v.registry_entry_id IN (
            SELECT ds.registry_entry_id
            FROM data_sources ds
            WHERE ds.id = ANY($1)
        )
        ORDER BY v.created_at DESC
        "#
    )
    .bind(protein_data_source_ids)
    .fetch_all(&mut **tx)
    .await?;

    // Create dependency links
    for protein_version_id in version_ids.iter().take(1000) {  // Limit to avoid massive links
        sqlx::query(
            r#"
            INSERT INTO dependencies (version_id, depends_on_version_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#
        )
        .bind(interpro_version_id)
        .bind(protein_version_id)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}
```

---

## Schema Design

### Complete Migration SQL

```sql
-- Migration: 20260128000001_add_interpro_tables.sql

-- ============================================================================
-- 1. InterPro Entry Metadata
-- ============================================================================

CREATE TABLE interpro_entry_metadata (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id UUID NOT NULL UNIQUE REFERENCES data_sources(id) ON DELETE CASCADE,

    -- Core InterPro fields
    interpro_id VARCHAR(20) NOT NULL UNIQUE,
    entry_type VARCHAR(50) NOT NULL,
    name TEXT NOT NULL,
    short_name VARCHAR(255),
    description TEXT,

    -- Status
    is_obsolete BOOLEAN DEFAULT FALSE,
    replacement_interpro_id VARCHAR(20),

    -- Metadata (JSONB for flexibility)
    member_databases JSONB DEFAULT '{}',
    go_mappings JSONB DEFAULT '[]',
    cross_references JSONB DEFAULT '{}',

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_interpro_metadata_ds ON interpro_entry_metadata(data_source_id);
CREATE INDEX idx_interpro_metadata_interpro_id ON interpro_entry_metadata(interpro_id);
CREATE INDEX idx_interpro_metadata_entry_type ON interpro_entry_metadata(entry_type);
CREATE INDEX idx_interpro_metadata_obsolete ON interpro_entry_metadata(is_obsolete);

-- GIN indexes for JSONB queries
CREATE INDEX idx_interpro_metadata_member_dbs ON interpro_entry_metadata USING GIN (member_databases);
CREATE INDEX idx_interpro_metadata_go_mappings ON interpro_entry_metadata USING GIN (go_mappings);

COMMENT ON TABLE interpro_entry_metadata IS
'Metadata for individual InterPro entries (domains, families, sites). Each entry is a separate data source.';

-- ============================================================================
-- 2. Protein-InterPro Match Table (Cross-References)
-- ============================================================================

CREATE TABLE protein_interpro_matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Source: InterPro entry
    interpro_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    interpro_version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,

    -- Target: UniProt protein
    protein_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    uniprot_accession VARCHAR(20) NOT NULL,

    -- Match details
    signature_accession VARCHAR(50),
    signature_database VARCHAR(50),
    start_position INTEGER NOT NULL CHECK (start_position > 0),
    end_position INTEGER NOT NULL CHECK (end_position >= start_position),
    e_value DOUBLE PRECISION,
    score DOUBLE PRECISION,

    -- Timestamp
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Performance indexes
CREATE INDEX idx_pim_interpro_ds ON protein_interpro_matches(interpro_data_source_id);
CREATE INDEX idx_pim_interpro_ver ON protein_interpro_matches(interpro_version_id);
CREATE INDEX idx_pim_protein_ds ON protein_interpro_matches(protein_data_source_id);
CREATE INDEX idx_pim_accession ON protein_interpro_matches(uniprot_accession);
CREATE INDEX idx_pim_signature ON protein_interpro_matches(signature_accession);

-- Composite indexes for common queries
CREATE INDEX idx_pim_protein_interpro ON protein_interpro_matches(protein_data_source_id, interpro_data_source_id);
CREATE INDEX idx_pim_positions ON protein_interpro_matches(start_position, end_position);

COMMENT ON TABLE protein_interpro_matches IS
'Links UniProt proteins to InterPro entries with match coordinates. Each match represents a domain/family annotation on a protein.';

-- ============================================================================
-- 3. Statistics Table (Optional - for performance)
-- ============================================================================

CREATE TABLE interpro_entry_stats (
    interpro_data_source_id UUID PRIMARY KEY REFERENCES data_sources(id) ON DELETE CASCADE,
    protein_count INTEGER NOT NULL DEFAULT 0,
    signature_count INTEGER NOT NULL DEFAULT 0,
    last_updated TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE interpro_entry_stats IS
'Cached statistics for InterPro entries to avoid expensive COUNT queries.';

-- ============================================================================
-- 4. Triggers for Statistics Updates
-- ============================================================================

CREATE OR REPLACE FUNCTION update_interpro_stats()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO interpro_entry_stats (interpro_data_source_id, protein_count, signature_count)
        VALUES (NEW.interpro_data_source_id, 1, 1)
        ON CONFLICT (interpro_data_source_id)
        DO UPDATE SET
            protein_count = interpro_entry_stats.protein_count + 1,
            last_updated = NOW();
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE interpro_entry_stats
        SET protein_count = protein_count - 1,
            last_updated = NOW()
        WHERE interpro_data_source_id = OLD.interpro_data_source_id;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_interpro_stats
AFTER INSERT OR DELETE ON protein_interpro_matches
FOR EACH ROW EXECUTE FUNCTION update_interpro_stats();

-- ============================================================================
-- 5. Sample Queries (for testing)
-- ============================================================================

-- Get all proteins with Kringle domains
COMMENT ON TABLE protein_interpro_matches IS
'Sample query: Find all proteins with Kringle domains
SELECT pm.accession, pim.start_position, pim.end_position
FROM protein_interpro_matches pim
JOIN protein_metadata pm ON pm.data_source_id = pim.protein_data_source_id
JOIN interpro_entry_metadata iem ON iem.data_source_id = pim.interpro_data_source_id
WHERE iem.interpro_id = ''IPR000001''
ORDER BY pm.accession;';

-- Get all InterPro domains for a specific protein
COMMENT ON TABLE interpro_entry_metadata IS
'Sample query: Find all domains for protein P01308
SELECT iem.interpro_id, iem.name, iem.entry_type,
       pim.start_position, pim.end_position, pim.e_value
FROM protein_interpro_matches pim
JOIN interpro_entry_metadata iem ON iem.data_source_id = pim.interpro_data_source_id
WHERE pim.uniprot_accession = ''P01308''
ORDER BY pim.start_position;';
```

---

## Storage Implementation

### Batch Processing Pattern

Following NCBI Taxonomy's efficient batch approach:

```rust
pub struct InterProStorage {
    db: PgPool,
    s3: Option<Storage>,
    organization_id: Uuid,
    internal_version: String,
    external_version: String,
    chunk_size: usize,
}

impl InterProStorage {
    pub const DEFAULT_CHUNK_SIZE: usize = 500;

    /// Store InterPro entries in batches
    pub async fn store_entries(&self, entries: &[InterProEntry]) -> Result<usize> {
        info!("Storing {} InterPro entries", entries.len());

        let mut tx = self.db.begin().await?;
        let mut stored_count = 0;

        // Process in chunks for performance
        for chunk in entries.chunks(self.chunk_size) {
            stored_count += self.store_entry_batch(&mut tx, chunk).await?;
        }

        tx.commit().await?;

        info!("Successfully stored {} InterPro entries", stored_count);
        Ok(stored_count)
    }

    /// Store a batch of entries (parallelized)
    async fn store_entry_batch(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        entries: &[InterProEntry],
    ) -> Result<usize> {
        // Step 1: Create registry entries in batch
        let entry_ids = self.create_registry_entries_batch(tx, entries).await?;

        // Step 2: Create data sources in batch
        let data_source_ids = self.create_data_sources_batch(tx, entries, &entry_ids).await?;

        // Step 3: Create metadata in batch
        self.create_metadata_batch(tx, entries, &data_source_ids).await?;

        // Step 4: Create versions in batch
        let version_ids = self.create_versions_batch(tx, &entry_ids).await?;

        // Step 5: Store protein matches in batch
        for (idx, entry) in entries.iter().enumerate() {
            let ds_id = data_source_ids[idx];
            let ver_id = version_ids[idx];

            self.store_matches(tx, ds_id, ver_id, &entry.protein_matches).await?;
        }

        // Step 6: Upload files to S3 (parallel)
        if let Some(ref s3) = self.s3 {
            self.upload_files_parallel(s3, entries, &data_source_ids).await?;
        }

        Ok(entries.len())
    }

    /// Create registry entries in batch (efficient)
    async fn create_registry_entries_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        entries: &[InterProEntry],
    ) -> Result<Vec<Uuid>> {
        let mut qb = QueryBuilder::new(
            "INSERT INTO registry_entries (
                organization_id,
                slug,
                name,
                description,
                entry_type
            ) "
        );

        qb.push_values(entries, |mut b, entry| {
            let description = format!(
                "InterPro {}: {}",
                entry.entry_type,
                entry.description.chars().take(200).collect::<String>()
            );

            b.push_bind(self.organization_id)
             .push_bind(&entry.interpro_id)
             .push_bind(&entry.name)
             .push_bind(description)
             .push_bind("data_source");
        });

        qb.push(" ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name RETURNING id");

        let ids: Vec<Uuid> = qb.build_query_scalar().fetch_all(&mut **tx).await?;
        Ok(ids)
    }
}
```

---

## Version File Formats

### File Structure per Entry

Each InterPro entry produces multiple format files:

```
s3://bdp-storage/data-sources/interpro/IPR000001/1.0.0/
  ├── IPR000001-1.0.0-matches.tsv.gz      # Protein matches (primary)
  ├── IPR000001-1.0.0-matches.json.gz     # JSON format
  ├── IPR000001-1.0.0-metadata.json       # Entry metadata
  └── checksums.sha256                    # Checksums for verification
```

### TSV Format (`IPR000001-1.0.0-matches.tsv.gz`)

```tsv
uniprot_accession	signature_accession	signature_database	start	end	e_value	score
P01308	PF00051	Pfam	120	180	1.2e-45	156.3
P01308	SM00130	SMART	121	179	3.4e-42	148.1
Q96GV9	PF00051	Pfam	45	105	5.6e-38	132.7
...
```

### JSON Format (`IPR000001-1.0.0-matches.json.gz`)

```json
{
  "interpro_id": "IPR000001",
  "name": "Kringle",
  "entry_type": "Domain",
  "version": "1.0.0",
  "external_version": "103.0",
  "protein_count": 1234,
  "matches": [
    {
      "uniprot_accession": "P01308",
      "protein_name": "Insulin",
      "organism": "Homo sapiens",
      "matches": [
        {
          "signature_accession": "PF00051",
          "signature_database": "Pfam",
          "start": 120,
          "end": 180,
          "e_value": 1.2e-45,
          "score": 156.3
        }
      ]
    }
  ]
}
```

### Metadata File (`IPR000001-1.0.0-metadata.json`)

```json
{
  "interpro_id": "IPR000001",
  "name": "Kringle",
  "short_name": "Kringle",
  "entry_type": "Domain",
  "description": "Kringles are autonomous structural domains...",
  "is_obsolete": false,
  "version": "1.0.0",
  "external_version": "103.0",
  "member_databases": {
    "Pfam": ["PF00051"],
    "SMART": ["SM00130"],
    "PROSITE": ["PS50070"]
  },
  "go_mappings": [
    {
      "go_id": "GO:0005515",
      "go_name": "protein binding",
      "evidence": "IEA"
    }
  ],
  "cross_references": {
    "PDB": ["1KRI", "1BUI"],
    "Wikipedia": ["Kringle_domain"]
  },
  "statistics": {
    "protein_count": 1234,
    "species_count": 87,
    "signature_count": 3
  }
}
```

---

## Change Detection

### Differential Ingestion Strategy

When ingesting a new InterPro release (e.g., 104.0 after 103.0):

1. **Download** new `protein2ipr.dat.gz` and `entry.list`
2. **Parse** new entries
3. **For each InterPro entry**:
   - Check if it exists in database
   - If **new**: Create v1.0.0
   - If **exists**: Run change detection
     - Compare protein lists
     - Compare metadata
     - Determine MAJOR vs MINOR bump
     - Create new version if changed

### Change Detection Example

```rust
pub async fn detect_entry_changes(
    pool: &PgPool,
    interpro_id: &str,
    new_entry: &InterProEntry,
) -> Result<Option<VersionChangelog>> {
    // Get current version data
    let current = sqlx::query_as::<_, InterProEntryRow>(
        r#"
        SELECT iem.*, v.id as version_id
        FROM interpro_entry_metadata iem
        JOIN data_sources ds ON ds.id = iem.data_source_id
        JOIN registry_entries re ON re.id = ds.registry_entry_id
        JOIN versions v ON v.registry_entry_id = re.id
        WHERE iem.interpro_id = $1
        ORDER BY v.created_at DESC
        LIMIT 1
        "#
    )
    .bind(interpro_id)
    .fetch_optional(pool)
    .await?;

    let Some(current) = current else {
        // New entry - create v1.0.0
        return Ok(Some(VersionChangelog::new(
            BumpType::Minor,
            vec![ChangelogEntry::added("entry", 1, "Initial ingestion")],
            ChangelogSummary::initial(new_entry.protein_matches.len() as i64),
            format!("Initial version of {} ({})", new_entry.name, interpro_id),
        )));
    };

    // Compare fields
    let mut changes = Vec::new();

    // Check obsolescence
    if new_entry.is_obsolete && !current.is_obsolete {
        changes.push(ChangelogEntry::new(
            ChangeType::Removed,
            "entry",
            format!("Entry {} marked as obsolete", interpro_id),
            true, // MAJOR
        ));
    }

    // Check entry type change
    if new_entry.entry_type != current.entry_type {
        changes.push(ChangelogEntry::new(
            ChangeType::Modified,
            "entry_type",
            format!("Type changed: {} → {}", current.entry_type, new_entry.entry_type),
            true, // MAJOR
        ));
    }

    // Check description change
    if new_entry.description != current.description {
        changes.push(ChangelogEntry::modified(
            "description",
            1,
            "Description updated",
            false, // MINOR
        ));
    }

    // Check protein count changes
    let prev_count = get_protein_count(pool, current.version_id).await?;
    let new_count = new_entry.protein_matches.len() as i64;
    let delta = new_count - prev_count;

    if delta > 0 {
        changes.push(ChangelogEntry::added(
            "proteins",
            delta,
            format!("{} new protein matches", delta),
        ));
    } else if delta < 0 {
        let loss_pct = (delta.abs() as f64 / prev_count as f64) * 100.0;
        let is_major = loss_pct > 50.0;

        changes.push(ChangelogEntry::with_count(
            ChangeType::Removed,
            if is_major { "proteins_major" } else { "proteins_minor" },
            delta.abs(),
            format!("{} protein matches removed ({:.1}% loss)", delta.abs(), loss_pct),
            is_major,
        ));
    }

    if changes.is_empty() {
        // No changes - skip version bump
        return Ok(None);
    }

    // Determine bump type
    let bump_type = VersionChangelog::determine_bump_type(&changes);

    let summary = ChangelogSummary::new(
        prev_count,
        new_count,
        delta.max(0),
        delta.abs().min(prev_count),
        0,
        TriggerReason::NewRelease,
    );

    Ok(Some(VersionChangelog::new(
        bump_type,
        changes,
        summary,
        format!("{} updated: {} proteins ({:+})", interpro_id, new_count, delta),
    )))
}
```

---

## Performance Optimization

### Batch Operations

Following NCBI Taxonomy's pattern:

- **Registry entries**: Batch insert 500 at a time
- **Data sources**: Batch insert 500 at a time
- **Metadata**: Batch insert 500 at a time
- **Protein matches**: Batch insert 500 at a time (per entry)

### Expected Performance

**Full ingestion (40,000 entries, ~200M matches)**:

| Operation | Without Batching | With Batching | Speedup |
|-----------|-----------------|---------------|---------|
| Registry entries | 40K queries | 80 queries | 500x |
| Data sources | 40K queries | 80 queries | 500x |
| Metadata | 40K queries | 80 queries | 500x |
| Protein matches | 200M queries | 400K queries | 500x |
| **Total** | ~200M queries | ~800K queries | **250x faster** |

**Estimated time**:
- Without optimization: ~48 hours
- With batching: ~10-15 minutes (database writes)
- S3 uploads: ~30 minutes (40K files)
- **Total**: ~45-60 minutes for full ingestion

### Parallel S3 Uploads

```rust
async fn upload_files_parallel(
    &self,
    s3: &Storage,
    entries: &[InterProEntry],
    data_source_ids: &[Uuid],
) -> Result<()> {
    use futures::stream::{self, StreamExt};

    const PARALLEL_UPLOADS: usize = 50;

    let upload_futures: Vec<_> = entries.iter()
        .zip(data_source_ids.iter())
        .map(|(entry, &ds_id)| {
            let s3 = s3.clone();
            let entry = entry.clone();

            async move {
                self.upload_entry_files(&s3, &entry, ds_id).await
            }
        })
        .collect();

    stream::iter(upload_futures)
        .buffer_unordered(PARALLEL_UPLOADS)
        .collect::<Vec<_>>()
        .await;

    Ok(())
}
```

---

## Implementation Plan

### Phase 1: Schema & Migrations (2 days)

- [ ] Create `interpro_entry_metadata` table migration
- [ ] Create `protein_interpro_matches` table migration
- [ ] Create `interpro_entry_stats` table migration
- [ ] Add indexes for performance
- [ ] Create triggers for stats updates
- [ ] Test migrations on staging database

### Phase 2: Models & Parsing (3 days)

- [ ] Define `InterProEntry` struct
- [ ] Define `ProteinMatch` struct
- [ ] Implement `Protein2IprParser` for TSV parsing
- [ ] Implement `EntryListParser` for metadata
- [ ] Unit tests for parsing
- [ ] Integration tests with sample data

### Phase 3: Cross-Reference Helper (2 days)

- [ ] Implement `ProteinLookupHelper` (like `TaxonomyHelper`)
- [ ] Add caching for protein lookups
- [ ] Add bulk lookup optimization
- [ ] Unit tests for helper
- [ ] Performance benchmarks

### Phase 4: Storage Layer (4 days)

- [ ] Implement `InterProStorage` struct
- [ ] Implement batch registry entry creation
- [ ] Implement batch data source creation
- [ ] Implement batch metadata creation
- [ ] Implement batch version creation
- [ ] Implement protein match storage with cross-refs
- [ ] Implement S3 file uploads (TSV, JSON, metadata)
- [ ] Integration tests with real InterPro data

### Phase 5: Versioning & Change Detection (3 days)

- [ ] Implement `VersioningStrategy::interpro()`
- [ ] Implement `InterProBumpDetector`
- [ ] Implement change detection logic
- [ ] Test MAJOR bump scenarios (obsolescence, type change)
- [ ] Test MINOR bump scenarios (protein additions, metadata updates)
- [ ] Test NO CHANGE scenarios

### Phase 6: FTP Downloader (2 days)

- [ ] Implement `InterProFtp` downloader (reuse `common/ftp.rs`)
- [ ] Implement version discovery from FTP
- [ ] Implement `protein2ipr.dat.gz` download
- [ ] Implement `entry.list` download
- [ ] Test FTP connection and downloads

### Phase 7: Pipeline Orchestration (3 days)

- [ ] Implement `InterProPipeline` end-to-end flow
- [ ] Integrate with `IngestOrchestrator`
- [ ] Create apalis job definition
- [ ] Implement differential ingestion (skip unchanged entries)
- [ ] Add logging and progress tracking

### Phase 8: Citation & Documentation (1 day)

- [ ] Add `interpro_citation_policy()`
- [ ] Integrate with `setup_citation_policy()`
- [ ] Update ROADMAP.md
- [ ] Write user documentation
- [ ] Write API documentation

### Phase 9: Testing & Validation (3 days)

- [ ] End-to-end integration test with InterPro 103.0
- [ ] Test version bumps with multiple releases
- [ ] Test cross-reference failures (missing proteins)
- [ ] Performance benchmarking
- [ ] Validate data integrity

**Total Estimated Time**: 23 days (~4-5 weeks)

---

## Summary

### Architecture Decisions

| Decision | Choice | Reasoning |
|----------|--------|-----------|
| **Data Source Pattern** | Individual (40K sources) | Matches 3 of 4 existing pipelines |
| **Versioning** | Semantic per entry | Independent version control |
| **Cross-References** | UniProt proteins | Natural dependency hierarchy |
| **Batch Size** | 500 entries/chunk | Proven optimal in NCBI Taxonomy |
| **Storage** | PostgreSQL + S3 | Existing pattern |

### Key Benefits

1. **Granular Versioning**: Each domain can version independently
2. **User Flexibility**: `bdp source add interpro:IPR000001@1.0`
3. **Pattern Consistency**: Matches UniProt/NCBI Taxonomy/GenBank
4. **Performance**: 250x faster with batching
5. **Dependency Tracking**: Explicit InterPro → UniProt links

### Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| 40K registry entries | Medium | Proven with UniProt (570K entries) |
| Missing UniProt proteins | Medium | Skip with warnings, track orphans |
| Large match table (200M rows) | Medium | Indexes + partitioning (future) |

---

**Next Steps**: Review this design, approve architecture, proceed with implementation.
