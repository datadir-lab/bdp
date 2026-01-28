# Database Design Philosophy - BDP

**Critical Reference for All Database Work**

---

## Core Principle: Relational First, JSONB Last

**BDP uses normalized relational design with foreign keys.** JSONB is a last resort for truly heterogeneous data.

---

## Golden Rules

### 1. NO JSONB for Primary Searchable Data

❌ **WRONG:**
```sql
CREATE TABLE interpro_entry_metadata (
    member_databases JSONB,  -- {"Pfam": ["PF00051"], "SMART": [...]}
    go_mappings JSONB        -- [{"go_id": "GO:0005515", ...}]
);
```

✅ **CORRECT:**
```sql
-- Separate normalized tables with foreign keys
CREATE TABLE interpro_member_signatures (
    interpro_data_source_id UUID REFERENCES data_sources(id),
    database VARCHAR(50) NOT NULL,           -- 'Pfam', 'SMART', 'PROSITE'
    signature_accession VARCHAR(50) NOT NULL -- 'PF00051'
);

CREATE TABLE interpro_go_mappings (
    interpro_data_source_id UUID REFERENCES data_sources(id),
    go_data_source_id UUID REFERENCES data_sources(id),  -- FK to GO term
    evidence_code VARCHAR(10) NOT NULL
);
```

**Why:** Foreign keys enable:
- Cross-database joins
- Referential integrity
- Cascade versioning
- Efficient indexes
- Type safety

---

### 2. Use TEXT[] for Simple Homogeneous Lists

✅ **Acceptable:**
```sql
-- Simple string lists that don't need relationships
alternative_names TEXT[] DEFAULT '{}',
ec_numbers TEXT[] DEFAULT '{}',
keywords TEXT[] DEFAULT '{}'
```

❌ **NOT for structured data:**
```sql
-- NO! This needs a relationship table
authors TEXT[]  -- Each author should be a row with affiliation, orcid, etc.
```

---

### 3. Separate Tables for One-to-Many

✅ **CORRECT:**
```sql
-- Parent
CREATE TABLE interpro_entry_metadata (
    data_source_id UUID PRIMARY KEY
);

-- Children (one-to-many)
CREATE TABLE interpro_member_signatures (
    id UUID PRIMARY KEY,
    interpro_data_source_id UUID REFERENCES data_sources(id) ON DELETE CASCADE,
    database VARCHAR(50),
    signature_accession VARCHAR(50)
);
```

**Pattern:** One parent row, many child rows via foreign key.

---

### 4. Junction Tables for Many-to-Many

✅ **CORRECT:**
```sql
-- InterPro entry references many UniProt proteins
CREATE TABLE protein_interpro_matches (
    id UUID PRIMARY KEY,

    -- Source: InterPro entry
    interpro_data_source_id UUID REFERENCES data_sources(id),
    interpro_version_id UUID REFERENCES versions(id),

    -- Target: UniProt protein
    protein_data_source_id UUID REFERENCES data_sources(id),
    protein_version_id UUID REFERENCES versions(id),

    -- Match metadata
    start_position INTEGER NOT NULL,
    end_position INTEGER NOT NULL,
    e_value DOUBLE PRECISION,
    score DOUBLE PRECISION
);
```

**Pattern:** Bridge table with FKs to both sides + relationship metadata.

---

### 5. Version-Specific Foreign Keys

✅ **CRITICAL:**
```sql
-- Link to SPECIFIC VERSION, not just data source
CREATE TABLE protein_interpro_matches (
    interpro_version_id UUID REFERENCES versions(id),  -- ✅ Version-specific
    protein_version_id UUID REFERENCES versions(id)    -- ✅ Version-specific
);
```

**Why:** When UniProt bumps from v1.0 to v1.1:
- InterPro must create v1.1 that references UniProt v1.1
- Old InterPro v1.0 still references UniProt v1.0
- Time-travel queries work correctly

---

### 6. Cascade Versioning Logic

When a dependency bumps version, cascade to dependents:

```rust
// UniProt v1.0 → v1.1 (proteins added)
if uniprot_version_bumped {
    // Find all InterPro entries that reference UniProt v1.0
    let dependent_interpro_entries = find_dependents(pool, uniprot_version_id).await?;

    for interpro_entry in dependent_interpro_entries {
        // Create new InterPro version pointing to UniProt v1.1
        let new_version = create_version(
            pool,
            interpro_entry.id,
            BumpType::Minor,  // Dependency update = MINOR
        ).await?;

        // Update protein_interpro_matches to reference new versions
        update_matches_to_new_versions(pool, new_version.id, uniprot_v1_1_id).await?;
    }
}
```

**Pattern:** Dependency version bump → cascade MINOR bumps to dependents.

---

## Versioning Rules

### MAJOR.MINOR Only (No Patch!)

✅ **CORRECT:**
```
1.0, 1.1, 1.2, 2.0, 2.1, 3.0
```

❌ **WRONG:**
```
1.0.0, 1.0.1, 1.1.0  -- NO PATCH VERSION!
```

**Database Schema:**
```sql
CREATE TABLE versions (
    version_major INTEGER NOT NULL,
    version_minor INTEGER NOT NULL,
    version_patch INTEGER NOT NULL DEFAULT 0,  -- Always 0

    CHECK (version_patch = 0)  -- Enforce no patch
);
```

**Rust Code:**
```rust
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    // No patch field!
}

impl Display for SemanticVersion {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)  // "1.0", "2.3"
    }
}
```

---

## Real-World Examples

### Example 1: InterPro Member Signatures

❌ **WRONG (JSONB):**
```sql
CREATE TABLE interpro_entry_metadata (
    member_databases JSONB  -- {"Pfam": ["PF00051"], "SMART": ["SM00130"]}
);
```

✅ **CORRECT (Foreign Keys):**
```sql
-- Signature definitions table (reusable across InterPro entries)
CREATE TABLE protein_signatures (
    id UUID PRIMARY KEY,
    database VARCHAR(50) NOT NULL,      -- 'Pfam', 'SMART', 'PROSITE'
    accession VARCHAR(50) NOT NULL,     -- 'PF00051', 'SM00130'
    name VARCHAR(255),
    description TEXT,

    UNIQUE(database, accession)
);

-- Link InterPro entries to signatures (many-to-many)
CREATE TABLE interpro_member_signatures (
    id UUID PRIMARY KEY,
    interpro_data_source_id UUID REFERENCES data_sources(id) ON DELETE CASCADE,
    signature_id UUID REFERENCES protein_signatures(id),

    UNIQUE(interpro_data_source_id, signature_id)
);

-- Index for bidirectional queries
CREATE INDEX idx_interpro_sigs_interpro ON interpro_member_signatures(interpro_data_source_id);
CREATE INDEX idx_interpro_sigs_signature ON interpro_member_signatures(signature_id);
```

**Query Examples:**
```sql
-- Find all Pfam signatures for IPR000001
SELECT ps.accession, ps.name
FROM interpro_member_signatures ims
JOIN protein_signatures ps ON ps.id = ims.signature_id
JOIN interpro_entry_metadata iem ON iem.data_source_id = ims.interpro_data_source_id
WHERE iem.interpro_id = 'IPR000001' AND ps.database = 'Pfam';

-- Find all InterPro entries containing Pfam signature PF00051
SELECT iem.interpro_id, iem.name
FROM interpro_member_signatures ims
JOIN protein_signatures ps ON ps.id = ims.signature_id
JOIN interpro_entry_metadata iem ON iem.data_source_id = ims.interpro_data_source_id
WHERE ps.accession = 'PF00051';
```

---

### Example 2: InterPro → GO Mappings

❌ **WRONG (JSONB):**
```sql
CREATE TABLE interpro_entry_metadata (
    go_mappings JSONB  -- [{"go_id": "GO:0005515", "go_name": "..."}]
);
```

✅ **CORRECT (Foreign Keys to GO Terms):**
```sql
CREATE TABLE interpro_go_mappings (
    id UUID PRIMARY KEY,

    -- Source: InterPro entry
    interpro_data_source_id UUID REFERENCES data_sources(id) ON DELETE CASCADE,
    interpro_version_id UUID REFERENCES versions(id),

    -- Target: GO term (also a data source!)
    go_data_source_id UUID REFERENCES data_sources(id),
    go_version_id UUID REFERENCES versions(id),

    -- Evidence
    evidence_code VARCHAR(10),  -- 'IEA', 'IDA', etc.

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(interpro_data_source_id, go_data_source_id)
);

-- Indexes
CREATE INDEX idx_interpro_go_interpro ON interpro_go_mappings(interpro_data_source_id);
CREATE INDEX idx_interpro_go_go ON interpro_go_mappings(go_data_source_id);
```

**Benefits:**
- Can join to `go_term_metadata` to get current GO term name
- Referential integrity: can't link to non-existent GO term
- When GO term version bumps, can cascade to InterPro
- Efficient bidirectional queries

---

### Example 3: Protein-InterPro Matches (Cross-References)

✅ **CORRECT Pattern:**
```sql
CREATE TABLE protein_interpro_matches (
    id UUID PRIMARY KEY,

    -- Version-specific foreign keys
    interpro_data_source_id UUID NOT NULL REFERENCES data_sources(id),
    interpro_version_id UUID NOT NULL REFERENCES versions(id),

    protein_data_source_id UUID NOT NULL REFERENCES data_sources(id),
    protein_version_id UUID NOT NULL REFERENCES versions(id),

    -- Denormalized for fast lookups (but still have FK!)
    uniprot_accession VARCHAR(20) NOT NULL,

    -- Match details (from specific signature)
    signature_id UUID REFERENCES protein_signatures(id),  -- FK to signature table!
    start_position INTEGER NOT NULL CHECK (start_position > 0),
    end_position INTEGER NOT NULL CHECK (end_position >= start_position),
    e_value DOUBLE PRECISION,
    score DOUBLE PRECISION,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Critical indexes
CREATE INDEX idx_pim_interpro_ver ON protein_interpro_matches(interpro_version_id);
CREATE INDEX idx_pim_protein_ver ON protein_interpro_matches(protein_version_id);
CREATE INDEX idx_pim_accession ON protein_interpro_matches(uniprot_accession);
CREATE INDEX idx_pim_signature ON protein_interpro_matches(signature_id);
CREATE INDEX idx_pim_positions ON protein_interpro_matches(start_position, end_position);
```

**Version Cascade Example:**

When UniProt P01308 bumps from v1.0 to v1.1:
1. InterPro entries that reference P01308 must create new versions
2. Old matches link to old versions, new matches link to new versions

```sql
-- Old version (still exists for time-travel)
protein_version_id = 'uuid-for-P01308-v1.0'

-- New version
protein_version_id = 'uuid-for-P01308-v1.1'
```

---

## When JSONB is Acceptable

### Exception 1: Truly Heterogeneous Extension Data

✅ **Acceptable:**
```sql
-- Optional flexible metadata where structure varies wildly
CREATE TABLE data_sources (
    metadata JSONB  -- Optional extension fields
);
```

**Rules:**
- Not primary data
- Not searchable as primary query
- Supplementary/optional information
- No foreign key relationships needed

### Exception 2: Complex Nested Structures (Rare)

✅ **Acceptable:**
```sql
CREATE TABLE go_term_metadata (
    -- Synonyms have type discriminators
    synonyms JSONB  -- [{"type": "EXACT", "text": "..."}, {"type": "RELATED", ...}]
);
```

**Why:** Each synonym is a mini-object with 2+ fields. Could normalize but overhead not worth it.

---

## Migration Checklist

Before writing any migration with JSONB, ask:

- [ ] Can this be a separate table with FK?
- [ ] Can this be TEXT[] array?
- [ ] Can this be normalized into columns?
- [ ] Is this truly heterogeneous data?
- [ ] Will we need to query/join on this data?
- [ ] Will this have foreign key relationships?

If you answered YES to any of the first 3 or last 2: **Don't use JSONB.**

---

## Index Strategy

### Always Index Foreign Keys
```sql
CREATE INDEX idx_table_parent_id ON child_table(parent_id);
```

### Composite Indexes for Common Queries
```sql
-- Query pattern: WHERE protein_id = ? AND interpro_id = ?
CREATE INDEX idx_matches_protein_interpro
ON protein_interpro_matches(protein_data_source_id, interpro_data_source_id);
```

### Partial Indexes for Filtered Queries
```sql
-- Only index non-obsolete entries
CREATE INDEX idx_interpro_active
ON interpro_entry_metadata(interpro_id)
WHERE is_obsolete = FALSE;
```

### GIN Indexes for Arrays (Not JSONB)
```sql
CREATE INDEX idx_keywords ON protein_metadata USING GIN(keywords);
-- Enables: WHERE keywords @> ARRAY['enzyme']
```

---

## Summary Table

| Data Type | Storage | Queryability | Use When |
|-----------|---------|--------------|----------|
| **Foreign Key** | Separate table | ✅ Full JOIN support | Related entities |
| **TEXT[]** | Array column | ✅ Array operators | Simple string lists |
| **VARCHAR/INT** | Scalar column | ✅ Full index support | Enumerated values |
| **JSONB** | JSON column | ⚠️ Limited (GIN) | Truly heterogeneous |

---

## Examples from Existing Code

### ✅ Good: protein_features (normalized)

```sql
CREATE TABLE protein_features (
    protein_id UUID REFERENCES protein_metadata(data_source_id),
    feature_type VARCHAR(50),  -- NOT JSONB!
    start_pos INT,
    end_pos INT,
    description TEXT
);
```

### ✅ Good: protein_publications (array for simple lists)

```sql
CREATE TABLE protein_publications (
    protein_id UUID,
    authors TEXT[],     -- Simple list ✅
    comments TEXT[]     -- Simple list ✅
);
```

### ✅ Good: protein_cross_references (FK to protein, not JSONB)

```sql
CREATE TABLE protein_cross_references (
    protein_id UUID REFERENCES protein_metadata(data_source_id),
    database VARCHAR(50),
    database_id VARCHAR(255)
);
```

### ⚠️ Acceptable Exception: go_term_metadata.synonyms

```sql
CREATE TABLE go_term_metadata (
    synonyms JSONB  -- [{"type": "EXACT", "text": "..."}]
    -- Each synonym is a complex object, low query priority
);
```

---

## Enforcement

### Code Review Checklist

Before merging any PR with new tables:

- [ ] No JSONB for primary data
- [ ] All relationships use foreign keys
- [ ] All foreign keys have indexes
- [ ] Version-specific FKs use `version_id` not just `data_source_id`
- [ ] Cascade logic implemented for dependency bumps
- [ ] MAJOR.MINOR versioning only (no patch)

### Database Migration Review

- [ ] Migration includes FK constraints
- [ ] Migration includes indexes
- [ ] Migration includes CHECK constraints
- [ ] JSONB usage justified in comments

---

## Reference Implementations

Study these files for correct patterns:

- `migrations/20260116000010_protein_metadata.sql` - Core metadata table
- `migrations/20260123000001_add_protein_publications.sql` - One-to-many with arrays
- `migrations/20260123000002_create_search_materialized_view.sql` - Cross-table queries
- `crates/bdp-server/src/ingest/uniprot/storage.rs` - Foreign key creation
- `crates/bdp-server/src/ingest/uniprot/taxonomy_helper.rs` - Cross-reference helper

---

## Questions?

When in doubt:

1. Check existing migrations for similar patterns
2. Prefer relational over JSONB
3. Ask: "Will I need to JOIN on this?" → If yes, use FK
4. Ask: "Will this change independently?" → If yes, separate table

**Remember:** We're building for **cross-database queries**. Foreign keys enable this. JSONB prevents it.
