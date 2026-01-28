# InterPro Integration Design - CORRECTED

**Date**: 2026-01-28
**Pattern**: Individual Data Sources + Relational Foreign Keys + MAJOR.MINOR Versioning
**Status**: Ready for Implementation

---

## Core Architecture

### Individual Data Sources (40,000 entries)

Each InterPro entry is a separate data source with MAJOR.MINOR versioning:

```
IPR000001 (Kringle domain)        → registry_entry → data_source → versions (1.0, 1.1, 2.0)
IPR000002 (Cation transporter)    → registry_entry → data_source → versions (1.0, 1.1)
... (40,000 total)
```

---

## Database Schema (Fully Relational - NO JSONB)

### 1. Core Metadata Table

```sql
CREATE TABLE interpro_entry_metadata (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id UUID NOT NULL UNIQUE REFERENCES data_sources(id) ON DELETE CASCADE,

    -- Core identifiers
    interpro_id VARCHAR(20) NOT NULL UNIQUE,  -- IPR000001
    entry_type VARCHAR(50) NOT NULL,          -- Family, Domain, Repeat, Site, Homologous_superfamily
    name TEXT NOT NULL,                       -- "Kringle"
    short_name VARCHAR(255),                  -- "Kringle"
    description TEXT,                         -- Full description

    -- Status
    is_obsolete BOOLEAN DEFAULT FALSE,
    replacement_interpro_id VARCHAR(20),      -- If obsoleted, what replaces it (FK added later)

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT fk_interpro_data_source
        FOREIGN KEY (data_source_id) REFERENCES data_sources(id) ON DELETE CASCADE,
    CONSTRAINT fk_replacement
        FOREIGN KEY (replacement_interpro_id) REFERENCES interpro_entry_metadata(interpro_id) DEFERRABLE
);

CREATE INDEX idx_interpro_metadata_ds ON interpro_entry_metadata(data_source_id);
CREATE INDEX idx_interpro_metadata_id ON interpro_entry_metadata(interpro_id);
CREATE INDEX idx_interpro_metadata_type ON interpro_entry_metadata(entry_type);
CREATE INDEX idx_interpro_metadata_obsolete ON interpro_entry_metadata(is_obsolete) WHERE is_obsolete = FALSE;
CREATE INDEX idx_interpro_metadata_replacement ON interpro_entry_metadata(replacement_interpro_id) WHERE replacement_interpro_id IS NOT NULL;

COMMENT ON TABLE interpro_entry_metadata IS
'Core metadata for InterPro entries. Each entry is an individual data source with independent versioning.';
```

---

### 2. Protein Signature Registry (Pfam, SMART, PROSITE, etc.)

**Separate table for signature definitions (reusable across InterPro entries):**

```sql
CREATE TABLE protein_signatures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Signature identification
    database VARCHAR(50) NOT NULL,           -- 'Pfam', 'SMART', 'PROSITE', 'PRINTS', 'PANTHER', etc.
    accession VARCHAR(50) NOT NULL,          -- 'PF00051', 'SM00130', 'PS50070'

    -- Metadata
    name VARCHAR(255),                       -- "7 transmembrane receptor"
    description TEXT,                        -- Full description from member database

    -- Pfam-specific (nullable for other databases)
    clan_accession VARCHAR(50),              -- Pfam clan (e.g., CL0192)
    clan_name VARCHAR(255),                  -- Clan name

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_signature UNIQUE(database, accession)
);

CREATE INDEX idx_signatures_database ON protein_signatures(database);
CREATE INDEX idx_signatures_accession ON protein_signatures(accession);
CREATE INDEX idx_signatures_clan ON protein_signatures(clan_accession) WHERE clan_accession IS NOT NULL;

COMMENT ON TABLE protein_signatures IS
'Registry of protein signatures from member databases (Pfam, SMART, PROSITE, etc.). Reusable across multiple InterPro entries.';
```

---

### 3. InterPro ↔ Member Signatures (Many-to-Many)

```sql
CREATE TABLE interpro_member_signatures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Source: InterPro entry
    interpro_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,

    -- Target: Member signature
    signature_id UUID NOT NULL REFERENCES protein_signatures(id) ON DELETE CASCADE,

    -- Relationship metadata
    is_primary BOOLEAN DEFAULT FALSE,         -- Is this the primary signature for this entry?
    integration_date DATE,                    -- When was this signature integrated?

    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_interpro_signature UNIQUE(interpro_data_source_id, signature_id)
);

CREATE INDEX idx_ims_interpro ON interpro_member_signatures(interpro_data_source_id);
CREATE INDEX idx_ims_signature ON interpro_member_signatures(signature_id);
CREATE INDEX idx_ims_primary ON interpro_member_signatures(is_primary) WHERE is_primary = TRUE;

COMMENT ON TABLE interpro_member_signatures IS
'Links InterPro entries to their constituent member database signatures (many-to-many). Example: IPR000001 integrates PF00051 (Pfam) + SM00130 (SMART).';
```

---

### 4. InterPro ↔ GO Term Mappings (Many-to-Many with Version FKs)

```sql
CREATE TABLE interpro_go_mappings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Source: InterPro entry (version-specific!)
    interpro_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    interpro_version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,

    -- Target: GO term (version-specific!)
    go_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    go_version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,

    -- Evidence
    evidence_code VARCHAR(10),               -- 'IEA' (Inferred from Electronic Annotation)

    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_interpro_go_mapping
        UNIQUE(interpro_data_source_id, go_data_source_id)
);

CREATE INDEX idx_igm_interpro_ds ON interpro_go_mappings(interpro_data_source_id);
CREATE INDEX idx_igm_interpro_ver ON interpro_go_mappings(interpro_version_id);
CREATE INDEX idx_igm_go_ds ON interpro_go_mappings(go_data_source_id);
CREATE INDEX idx_igm_go_ver ON interpro_go_mappings(go_version_id);

COMMENT ON TABLE interpro_go_mappings IS
'Links InterPro entries to Gene Ontology terms with version-specific foreign keys. Enables cascade versioning when GO terms update.';
```

---

### 5. Protein ↔ InterPro Matches (Many-to-Many with Coordinates)

**This is the main cross-reference table:**

```sql
CREATE TABLE protein_interpro_matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Source: InterPro entry (version-specific!)
    interpro_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    interpro_version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,

    -- Target: UniProt protein (version-specific!)
    protein_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    protein_version_id UUID NOT NULL REFERENCES versions(id) ON DELETE CASCADE,

    -- Denormalized for fast lookups (but still have FK!)
    uniprot_accession VARCHAR(20) NOT NULL,

    -- Match origin: which signature triggered this match
    signature_id UUID NOT NULL REFERENCES protein_signatures(id),

    -- Match coordinates
    start_position INTEGER NOT NULL CHECK (start_position > 0),
    end_position INTEGER NOT NULL CHECK (end_position >= start_position),

    -- Match quality
    e_value DOUBLE PRECISION,
    score DOUBLE PRECISION,

    created_at TIMESTAMPTZ DEFAULT NOW(),

    -- Prevent duplicate matches for same protein-interpro-signature-position
    CONSTRAINT unique_match
        UNIQUE(protein_data_source_id, interpro_data_source_id, signature_id, start_position, end_position)
);

-- Critical indexes for bidirectional queries
CREATE INDEX idx_pim_interpro_ds ON protein_interpro_matches(interpro_data_source_id);
CREATE INDEX idx_pim_interpro_ver ON protein_interpro_matches(interpro_version_id);
CREATE INDEX idx_pim_protein_ds ON protein_interpro_matches(protein_data_source_id);
CREATE INDEX idx_pim_protein_ver ON protein_interpro_matches(protein_version_id);
CREATE INDEX idx_pim_accession ON protein_interpro_matches(uniprot_accession);
CREATE INDEX idx_pim_signature ON protein_interpro_matches(signature_id);
CREATE INDEX idx_pim_positions ON protein_interpro_matches(start_position, end_position);

-- Composite index for common query pattern
CREATE INDEX idx_pim_protein_interpro
ON protein_interpro_matches(protein_data_source_id, interpro_data_source_id);

COMMENT ON TABLE protein_interpro_matches IS
'Links UniProt proteins to InterPro entries with match coordinates. Version-specific foreign keys enable time-travel queries and cascade versioning.';
```

---

### 6. External Cross-References (InterPro → PDB, Wikipedia, etc.)

```sql
CREATE TABLE interpro_external_references (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    interpro_data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,

    -- External database
    database VARCHAR(50) NOT NULL,           -- 'PDB', 'CATH', 'SCOP', 'Wikipedia', 'KEGG', etc.
    database_id VARCHAR(255) NOT NULL,       -- '1KRI', 'Kringle_domain', etc.

    -- Optional metadata
    description TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_interpro_xref
        UNIQUE(interpro_data_source_id, database, database_id)
);

CREATE INDEX idx_ixr_interpro ON interpro_external_references(interpro_data_source_id);
CREATE INDEX idx_ixr_database ON interpro_external_references(database);
CREATE INDEX idx_ixr_db_id ON interpro_external_references(database_id);

COMMENT ON TABLE interpro_external_references IS
'Cross-references from InterPro entries to external databases (PDB structures, Wikipedia articles, KEGG pathways, etc.).';
```

---

### 7. InterPro Entry Statistics (Cached Aggregates)

```sql
CREATE TABLE interpro_entry_stats (
    interpro_data_source_id UUID PRIMARY KEY REFERENCES data_sources(id) ON DELETE CASCADE,

    -- Cached counts
    protein_count INTEGER NOT NULL DEFAULT 0,
    species_count INTEGER NOT NULL DEFAULT 0,
    signature_count INTEGER NOT NULL DEFAULT 0,

    last_updated TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE interpro_entry_stats IS
'Cached statistics for InterPro entries to avoid expensive COUNT queries. Updated by triggers.';

-- Trigger function to update stats
CREATE OR REPLACE FUNCTION update_interpro_stats()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO interpro_entry_stats (interpro_data_source_id, protein_count)
        VALUES (NEW.interpro_data_source_id, 1)
        ON CONFLICT (interpro_data_source_id)
        DO UPDATE SET
            protein_count = interpro_entry_stats.protein_count + 1,
            last_updated = NOW();
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE interpro_entry_stats
        SET protein_count = GREATEST(0, protein_count - 1),
            last_updated = NOW()
        WHERE interpro_data_source_id = OLD.interpro_data_source_id;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_interpro_stats
AFTER INSERT OR DELETE ON protein_interpro_matches
FOR EACH ROW EXECUTE FUNCTION update_interpro_stats();
```

---

## Versioning Strategy (MAJOR.MINOR Only!)

### Semantic Version Structure

```rust
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    // NO PATCH!
}

impl Display for SemanticVersion {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}
```

### Database Storage

```sql
-- Existing versions table already has this
CREATE TABLE versions (
    version_major INTEGER NOT NULL,
    version_minor INTEGER NOT NULL,
    version_patch INTEGER NOT NULL DEFAULT 0,

    -- Constraint: patch must always be 0
    CHECK (version_patch = 0)
);
```

### Version Bump Rules

**MAJOR Bumps (Breaking Changes):**
- Entry obsoleted (`is_obsolete = TRUE`)
- Entry type changed (`entry_type` modified)
- >50% protein loss
- Primary signature removed

**MINOR Bumps (Non-Breaking Changes):**
- Proteins added
- Description updated
- Signature added
- GO mapping added
- <10% protein loss
- Dependency version bumped (cascade)

### Cascade Versioning Logic

**When UniProt v1.0 → v1.1:**

```rust
async fn cascade_uniprot_version_to_interpro(
    pool: &PgPool,
    uniprot_version_id: Uuid,
    new_uniprot_version_id: Uuid,
) -> Result<Vec<CascadeResult>> {
    // 1. Find all InterPro entries that reference the old UniProt version
    let dependent_interpro = sqlx::query_as::<_, DependentEntry>(
        r#"
        SELECT DISTINCT
            iem.interpro_id,
            ds.id as data_source_id,
            re.id as registry_entry_id
        FROM protein_interpro_matches pim
        JOIN interpro_entry_metadata iem ON iem.data_source_id = pim.interpro_data_source_id
        JOIN data_sources ds ON ds.id = iem.data_source_id
        JOIN registry_entries re ON re.id = ds.registry_entry_id
        WHERE pim.protein_version_id = $1
        "#
    )
    .bind(uniprot_version_id)
    .fetch_all(pool)
    .await?;

    let mut results = Vec::new();

    for entry in dependent_interpro {
        // 2. Create new MINOR version for each affected InterPro entry
        let new_version = create_version(
            pool,
            entry.registry_entry_id,
            BumpType::Minor,
            format!("Dependency update: UniProt version bumped"),
        ).await?;

        // 3. Copy matches from old version to new version, updating protein_version_id
        sqlx::query(
            r#"
            INSERT INTO protein_interpro_matches (
                interpro_data_source_id,
                interpro_version_id,
                protein_data_source_id,
                protein_version_id,
                uniprot_accession,
                signature_id,
                start_position,
                end_position,
                e_value,
                score
            )
            SELECT
                interpro_data_source_id,
                $1,  -- New InterPro version
                protein_data_source_id,
                $2,  -- New UniProt version
                uniprot_accession,
                signature_id,
                start_position,
                end_position,
                e_value,
                score
            FROM protein_interpro_matches
            WHERE interpro_data_source_id = $3
              AND interpro_version_id = $4
            "#
        )
        .bind(new_version.id)
        .bind(new_uniprot_version_id)
        .bind(entry.data_source_id)
        .bind(entry.current_version_id)
        .execute(pool)
        .await?;

        results.push(CascadeResult {
            entry_id: entry.registry_entry_id,
            new_version_id: new_version.id,
            trigger: "UniProt dependency update".to_string(),
        });
    }

    Ok(results)
}
```

---

## Cross-Reference Helpers

### ProteinLookupHelper (Like TaxonomyHelper)

```rust
pub struct ProteinLookupHelper {
    db: PgPool,
    cache: HashMap<String, Uuid>,  // accession → data_source_id
}

impl ProteinLookupHelper {
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            cache: HashMap::new(),
        }
    }

    /// Bulk lookup proteins by accessions (efficient batch query)
    pub async fn lookup_proteins_bulk(
        &mut self,
        accessions: &[String],
    ) -> Result<HashMap<String, Uuid>> {
        // Filter out cached
        let uncached: Vec<_> = accessions.iter()
            .filter(|acc| !self.cache.contains_key(*acc))
            .collect();

        if !uncached.is_empty() {
            // Batch query
            let results: Vec<(String, Uuid)> = sqlx::query_as(
                r#"
                SELECT pm.accession, pm.data_source_id
                FROM protein_metadata pm
                WHERE pm.accession = ANY($1)
                "#
            )
            .bind(&uncached)
            .fetch_all(&self.db)
            .await?;

            // Update cache
            for (accession, ds_id) in results {
                self.cache.insert(accession, ds_id);
            }
        }

        // Return combined results
        Ok(accessions.iter()
            .filter_map(|acc| self.cache.get(acc).map(|&id| (acc.clone(), id)))
            .collect())
    }

    /// Get current version ID for a protein data source
    pub async fn get_current_version(
        &self,
        protein_ds_id: Uuid,
    ) -> Result<Uuid> {
        let version_id = sqlx::query_scalar(
            r#"
            SELECT v.id
            FROM versions v
            JOIN registry_entries re ON re.id = v.registry_entry_id
            JOIN data_sources ds ON ds.registry_entry_id = re.id
            WHERE ds.id = $1
            ORDER BY v.created_at DESC
            LIMIT 1
            "#
        )
        .bind(protein_ds_id)
        .fetch_one(&self.db)
        .await?;

        Ok(version_id)
    }
}
```

---

## Storage Implementation

### Batch Operations (Following NCBI Taxonomy Pattern)

```rust
impl InterProStorage {
    const DEFAULT_CHUNK_SIZE: usize = 500;

    pub async fn store_entries(&self, entries: &[InterProEntry]) -> Result<usize> {
        let mut stored = 0;

        for chunk in entries.chunks(Self::DEFAULT_CHUNK_SIZE) {
            stored += self.store_chunk(chunk).await?;
        }

        Ok(stored)
    }

    async fn store_chunk(&self, entries: &[InterProEntry]) -> Result<usize> {
        let mut tx = self.db.begin().await?;

        // 1. Batch create registry entries
        let entry_ids = self.create_registry_entries_batch(&mut tx, entries).await?;

        // 2. Batch create data sources
        let ds_ids = self.create_data_sources_batch(&mut tx, &entry_ids, entries).await?;

        // 3. Batch create metadata
        self.create_metadata_batch(&mut tx, &ds_ids, entries).await?;

        // 4. Batch create versions
        let version_ids = self.create_versions_batch(&mut tx, &entry_ids).await?;

        // 5. Batch insert member signatures
        self.insert_member_signatures_batch(&mut tx, &ds_ids, entries).await?;

        // 6. Batch insert GO mappings
        self.insert_go_mappings_batch(&mut tx, &ds_ids, &version_ids, entries).await?;

        // 7. Batch insert protein matches
        self.insert_protein_matches_batch(&mut tx, &ds_ids, &version_ids, entries).await?;

        tx.commit().await?;

        Ok(entries.len())
    }

    async fn insert_protein_matches_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ds_ids: &[Uuid],
        version_ids: &[Uuid],
        entries: &[InterProEntry],
    ) -> Result<()> {
        let mut helper = ProteinLookupHelper::new(self.db.clone());

        for (idx, entry) in entries.iter().enumerate() {
            let interpro_ds_id = ds_ids[idx];
            let interpro_ver_id = version_ids[idx];

            // Bulk lookup all proteins for this entry
            let accessions: Vec<String> = entry.protein_matches.iter()
                .map(|m| m.uniprot_accession.clone())
                .collect();

            let protein_map = helper.lookup_proteins_bulk(&accessions).await?;

            // Batch insert matches
            let mut qb = QueryBuilder::new(
                "INSERT INTO protein_interpro_matches (
                    interpro_data_source_id,
                    interpro_version_id,
                    protein_data_source_id,
                    protein_version_id,
                    uniprot_accession,
                    signature_id,
                    start_position,
                    end_position,
                    e_value,
                    score
                ) "
            );

            let mut values_added = 0;

            for match_data in &entry.protein_matches {
                // Look up protein
                let protein_ds_id = match protein_map.get(&match_data.uniprot_accession) {
                    Some(id) => *id,
                    None => {
                        warn!("Protein {} not found - skipping", match_data.uniprot_accession);
                        continue;
                    }
                };

                // Get protein version
                let protein_ver_id = helper.get_current_version(protein_ds_id).await?;

                // Look up signature ID
                let signature_id = self.get_or_create_signature(
                    tx,
                    &match_data.signature_database,
                    &match_data.signature_accession,
                ).await?;

                if values_added > 0 {
                    qb.push(", ");
                }

                qb.push("(");
                qb.push_bind(interpro_ds_id);
                qb.push(", ");
                qb.push_bind(interpro_ver_id);
                qb.push(", ");
                qb.push_bind(protein_ds_id);
                qb.push(", ");
                qb.push_bind(protein_ver_id);
                qb.push(", ");
                qb.push_bind(&match_data.uniprot_accession);
                qb.push(", ");
                qb.push_bind(signature_id);
                qb.push(", ");
                qb.push_bind(match_data.start);
                qb.push(", ");
                qb.push_bind(match_data.end);
                qb.push(", ");
                qb.push_bind(match_data.e_value);
                qb.push(", ");
                qb.push_bind(match_data.score);
                qb.push(")");

                values_added += 1;
            }

            if values_added > 0 {
                qb.push(" ON CONFLICT DO NOTHING");
                qb.build().execute(&mut **tx).await?;
            }
        }

        Ok(())
    }
}
```

---

## Query Examples

### Find all proteins with Kringle domain

```sql
SELECT
    pm.accession,
    pm.protein_name,
    pim.start_position,
    pim.end_position,
    pim.e_value
FROM protein_interpro_matches pim
JOIN protein_metadata pm ON pm.data_source_id = pim.protein_data_source_id
JOIN interpro_entry_metadata iem ON iem.data_source_id = pim.interpro_data_source_id
WHERE iem.interpro_id = 'IPR000001'
ORDER BY pm.accession, pim.start_position;
```

### Find all InterPro domains for protein P01308

```sql
SELECT
    iem.interpro_id,
    iem.name,
    iem.entry_type,
    pim.start_position,
    pim.end_position,
    ps.database,
    ps.accession as signature_accession
FROM protein_interpro_matches pim
JOIN interpro_entry_metadata iem ON iem.data_source_id = pim.interpro_data_source_id
JOIN protein_signatures ps ON ps.id = pim.signature_id
WHERE pim.uniprot_accession = 'P01308'
ORDER BY pim.start_position;
```

### Find all InterPro entries containing Pfam signature PF00051

```sql
SELECT
    iem.interpro_id,
    iem.name,
    iem.entry_type,
    COUNT(DISTINCT pim.protein_data_source_id) as protein_count
FROM interpro_member_signatures ims
JOIN protein_signatures ps ON ps.id = ims.signature_id
JOIN interpro_entry_metadata iem ON iem.data_source_id = ims.interpro_data_source_id
LEFT JOIN protein_interpro_matches pim ON pim.interpro_data_source_id = iem.data_source_id
WHERE ps.accession = 'PF00051'
GROUP BY iem.interpro_id, iem.name, iem.entry_type;
```

---

## File Formats

### TSV (Primary Distribution)

```tsv
uniprot_accession	signature_database	signature_accession	start	end	e_value	score
P01308	Pfam	PF00051	120	180	1.2e-45	156.3
P01308	SMART	SM00130	121	179	3.4e-42	148.1
Q96GV9	Pfam	PF00051	45	105	5.6e-38	132.7
```

### JSON (Structured)

```json
{
  "interpro_id": "IPR000001",
  "name": "Kringle",
  "entry_type": "Domain",
  "version": "1.0",
  "external_version": "103.0",
  "statistics": {
    "protein_count": 1234,
    "species_count": 87
  },
  "member_signatures": [
    {"database": "Pfam", "accession": "PF00051", "name": "Kringle"},
    {"database": "SMART", "accession": "SM00130", "name": "Kringle"}
  ],
  "protein_matches": [
    {
      "uniprot_accession": "P01308",
      "protein_name": "Insulin",
      "matches": [
        {
          "signature": "PF00051",
          "database": "Pfam",
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

---

## Implementation Checklist

- [ ] Create migrations (7 tables)
- [ ] Implement models (InterProEntry, ProteinMatch, Signature)
- [ ] Implement parsers (protein2ipr.dat.gz, entry.list)
- [ ] Implement ProteinLookupHelper
- [ ] Implement InterProStorage with batch operations
- [ ] Implement cascade versioning logic
- [ ] Implement FTP downloader
- [ ] Implement change detection
- [ ] Implement InterProPipeline
- [ ] Add tests
- [ ] Add documentation

---

## Summary of Corrections

| Issue | Old (Wrong) | New (Correct) |
|-------|-------------|---------------|
| **JSONB** | `member_databases JSONB` | `interpro_member_signatures` table with FK |
| **JSONB** | `go_mappings JSONB` | `interpro_go_mappings` table with FK to GO |
| **JSONB** | `cross_references JSONB` | `interpro_external_references` table |
| **Versioning** | 1.0.0, 1.0.1 (patch) | 1.0, 1.1 (no patch) |
| **Version FKs** | Missing version FKs | `version_id` FK everywhere |
| **Cascade** | Not specified | Explicit cascade versioning logic |

**Design Philosophy**: Fully relational with foreign keys. JSONB eliminated for all primary data. Version-specific FKs enable cascade versioning and time-travel queries.
