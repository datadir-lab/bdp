# BDP Schema Refactor & Ingestion System V2

**Date**: 2026-01-18
**Status**: 🚧 Planning Phase
**Type**: Major Schema Refactor + Ingestion Rewrite

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Current Problems](#current-problems)
3. [Proposed Architecture](#proposed-architecture)
4. [Schema Changes](#schema-changes)
5. [Versioning Strategy](#versioning-strategy)
6. [Ingestion Pipeline](#ingestion-pipeline)
7. [Data Source Organizations](#data-source-organizations)
8. [Migration Plan](#migration-plan)
9. [Implementation Phases](#implementation-phases)
10. [Success Criteria](#success-criteria)

---

## Executive Summary

### Goals

1. **Reproducibility**: Pin exact versions of data sources (e.g., `uniprot:p01308@1.2.3`)
2. **Transparency**: Auto-generated changelog documenting all changes
3. **Efficiency**: Sequence deduplication to minimize storage
4. **Extensibility**: Support proteins, genomes, organisms, bundles with shared patterns
5. **Compliance**: License tracking per data source

### Key Changes

- ✅ Semantic versioning (MAJOR.MINOR.PATCH) for all data sources
- ✅ Sequence deduplication via `protein_sequences` table
- ✅ Version-pinned dependencies for reproducible bundles
- ✅ Licenses table with per-entry licensing
- ✅ Organization versioning rules (human-readable documentation)
- ✅ Auto-generated changelog based on change detection
- ✅ Deprecated flag + alias support for accession changes
- ✅ Organism as data source (not just FK to organisms table)

### Impact

- **Storage**: ~15% reduction after 20 releases (sequence deduplication)
- **Reproducibility**: 100% - exact version pinning with lockfiles
- **Scalability**: Supports 571K proteins × multiple releases efficiently
- **Extensibility**: Same patterns for genomes, organisms, bundles

---

## Current Problems

### 1. Wrong Table Structure

**Current ingestion** (INCORRECT):
```rust
INSERT INTO proteins (accession, sequence, ...) VALUES (...);
```

**Problem**: Bypasses registry pattern entirely!

### 2. No Version Strategy

- No semantic versioning
- No change tracking
- No reproducibility guarantee

### 3. Duplicate Sequences

```sql
protein_metadata:
  p01308 version 1.0: "MALWMR..." (500 bytes)
  p01308 version 1.1: "MALWMR..." (500 bytes) ← WASTED!
```

### 4. No License Tracking

- Can't determine if data is CC-BY, CC0, or proprietary
- Legal compliance risk

### 5. Organism Inconsistency

- `data_sources.organism_id` → organisms table
- But organisms should BE data sources themselves!

---

## Proposed Architecture

### Registry Pattern (Correct)

```
registry_entries (base)
    ├── data_sources (inherits)
    │   ├── protein_metadata (source_type="protein")
    │   ├── genome_metadata (source_type="genome")
    │   ├── organism_metadata (source_type="organism")
    │   └── bundle (source_type="bundle")
    └── tools (not covered here)

versions (one-to-many with data_sources)
    └── version_files (FASTA, JSON, etc. in S3)

dependencies (bundle → proteins with version pins)

protein_sequences (deduplicated sequences)
```

### S3 Storage Structure

```
s3://bdp-data/sources/
├── uniprot/                        # organization.slug
│   ├── p01308/                     # registry_entry.slug (protein)
│   │   ├── 1.0.0/
│   │   │   ├── p01308.fasta        # Amino acid sequence
│   │   │   ├── p01308.fai          # FASTA index
│   │   │   └── metadata.json       # Additional metadata
│   │   ├── 1.1.0/
│   │   │   └── p01308.fasta        # Updated sequence
│   │   └── 2.0.0/
│   │       └── p01308.fasta        # Major change
│   ├── p12345/
│   │   └── 1.0.0/
│   └── swissprot/                  # bundle
│       ├── 1.0.0/
│       │   └── manifest.json       # {"dependencies": ["p01308@1.0.0", ...]}
│       └── 2.0.0/
│           └── manifest.json       # Updated dependencies
├── ncbi/
│   ├── homo-sapiens/               # organism
│   │   └── 1.0.0/
│   │       └── metadata.json
│   └── grch38/                     # genome
│       └── 1.0.0/
│           ├── genome.fa.gz
│           └── genome.fa.fai
└── ensembl/
    └── ...
```

---

## Schema Changes

### 1. Organizations Table Enhancement

```sql
ALTER TABLE organizations
ADD COLUMN versioning_rules TEXT;  -- Markdown documentation for researchers

-- Example content
UPDATE organizations
SET versioning_rules = $markdown$
# UniProt Versioning Rules

## Semantic Versioning

UniProt data sources follow semantic versioning: **MAJOR.MINOR.PATCH**

### MAJOR version bump
- Sequence changed (amino acid substitution, insertion, deletion)
- Protein length changed
- Organism changed (reclassification)
- Accession merged or split

### MINOR version bump
- Gene name changed
- Protein name updated
- Functional annotation added/updated
- New cross-references added

### PATCH version bump
- Description typo fixed
- Cross-reference URL updated
- Minor metadata corrections

## Reproducibility

All versions are **immutable**. Version 1.2.3 will always return the exact same data.

## External Version Mapping

| BDP Version | UniProt Release | Release Date |
|-------------|-----------------|--------------|
| 1.0.0       | 2025_01         | 2025-01-15   |
| 1.1.0       | 2025_02         | 2025-02-12   |

Use `version_mappings` table to lookup external versions.
$markdown$
WHERE slug = 'uniprot';
```

### 2. Licenses Table

```sql
-- New licenses table
CREATE TABLE licenses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,          -- "CC-BY-4.0"
    full_name VARCHAR(500) NOT NULL,            -- "Creative Commons Attribution 4.0"
    url TEXT,                                   -- https://creativecommons.org/licenses/by/4.0/
    spdx_identifier VARCHAR(100),               -- "CC-BY-4.0" (standardized)
    requires_attribution BOOLEAN DEFAULT FALSE,
    allows_commercial BOOLEAN DEFAULT TRUE,
    allows_derivatives BOOLEAN DEFAULT TRUE,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Seed common licenses
INSERT INTO licenses (name, full_name, url, spdx_identifier, requires_attribution, allows_commercial, allows_derivatives) VALUES
('CC-BY-4.0', 'Creative Commons Attribution 4.0 International', 'https://creativecommons.org/licenses/by/4.0/', 'CC-BY-4.0', TRUE, TRUE, TRUE),
('CC0-1.0', 'Creative Commons Zero v1.0 Universal', 'https://creativecommons.org/publicdomain/zero/1.0/', 'CC0-1.0', FALSE, TRUE, TRUE),
('ODC-By-1.0', 'Open Data Commons Attribution License v1.0', 'https://opendatacommons.org/licenses/by/1-0/', 'ODC-By-1.0', TRUE, TRUE, TRUE),
('Proprietary', 'Proprietary License', NULL, NULL, NULL, FALSE, FALSE);

-- Update registry_entries to reference licenses
ALTER TABLE registry_entries
ADD COLUMN license_id UUID REFERENCES licenses(id);

-- Remove old VARCHAR field if exists
-- ALTER TABLE registry_entries DROP COLUMN license;
```

### 3. Semantic Versioning for Versions Table

```sql
ALTER TABLE versions
ADD COLUMN version_major INTEGER NOT NULL DEFAULT 1,
ADD COLUMN version_minor INTEGER NOT NULL DEFAULT 0,
ADD COLUMN version_patch INTEGER NOT NULL DEFAULT 0,
ADD COLUMN version_string VARCHAR(50) GENERATED ALWAYS AS (
    version_major || '.' || version_minor || '.' || version_patch
) STORED,
ADD COLUMN changelog TEXT,  -- Auto-generated or manual
ADD COLUMN release_notes TEXT,  -- Human-written summary
ADD COLUMN external_version VARCHAR(100);  -- e.g., "2025_01" for UniProt

-- Update existing 'version' column to match version_string
-- ALTER TABLE versions DROP COLUMN version;  -- Old column

-- Indexes
CREATE INDEX versions_semver_idx ON versions(data_source_id, version_major DESC, version_minor DESC, version_patch DESC);
CREATE INDEX versions_string_idx ON versions(data_source_id, version_string);

-- Function to get latest version
CREATE OR REPLACE FUNCTION get_latest_version(p_data_source_id UUID)
RETURNS TABLE(
    id UUID,
    version_string VARCHAR(50),
    version_major INTEGER,
    version_minor INTEGER,
    version_patch INTEGER
) AS $$
BEGIN
    RETURN QUERY
    SELECT v.id, v.version_string, v.version_major, v.version_minor, v.version_patch
    FROM versions v
    WHERE v.data_source_id = p_data_source_id
    ORDER BY v.version_major DESC, v.version_minor DESC, v.version_patch DESC
    LIMIT 1;
END;
$$ LANGUAGE plpgsql;
```

### 4. Version-Pinned Dependencies

```sql
ALTER TABLE dependencies
ADD COLUMN dependency_version_id UUID REFERENCES versions(id);

-- Add constraint: must specify version
ALTER TABLE dependencies
ADD CONSTRAINT dependency_version_required CHECK (dependency_version_id IS NOT NULL);

-- Index for fast lookups
CREATE INDEX dependencies_version_idx ON dependencies(dependency_version_id);

-- Example usage
INSERT INTO dependencies (dependent_id, dependency_id, dependency_version_id) VALUES (
    (SELECT id FROM data_sources WHERE external_id = 'swissprot'),  -- human:all
    (SELECT id FROM data_sources WHERE external_id = 'P01308'),     -- p01308
    (SELECT id FROM versions WHERE data_source_id = (SELECT id FROM data_sources WHERE external_id = 'P01308') AND version_string = '1.2.3')
);
```

### 5. Sequence Deduplication

```sql
-- New table for deduplicated sequences
CREATE TABLE protein_sequences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sequence TEXT NOT NULL,
    sequence_hash VARCHAR(64) UNIQUE NOT NULL,  -- SHA256 of sequence
    sequence_length INTEGER NOT NULL,
    sequence_md5 VARCHAR(32) NOT NULL,          -- MD5 for compatibility
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Unique index on hash for fast deduplication
CREATE UNIQUE INDEX protein_sequences_hash_idx ON protein_sequences(sequence_hash);

-- Index for length-based queries
CREATE INDEX protein_sequences_length_idx ON protein_sequences(sequence_length);

-- Trigram index for sequence substring search (motif search)
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX protein_sequences_trigram_idx ON protein_sequences USING GIN (sequence gin_trgm_ops);

-- Update protein_metadata
ALTER TABLE protein_metadata
DROP COLUMN sequence,           -- Remove direct sequence storage
DROP COLUMN sequence_checksum,  -- Now in protein_sequences
DROP COLUMN sequence_length,    -- Now in protein_sequences
ADD COLUMN sequence_id UUID REFERENCES protein_sequences(id);

-- Index for fast joins
CREATE INDEX protein_metadata_sequence_idx ON protein_metadata(sequence_id);
```

### 6. Deprecated & Alias Support

```sql
-- Add deprecation support to registry_entries
ALTER TABLE registry_entries
ADD COLUMN deprecated BOOLEAN DEFAULT FALSE,
ADD COLUMN deprecated_at TIMESTAMPTZ,
ADD COLUMN deprecated_reason TEXT,
ADD COLUMN superseded_by_id UUID REFERENCES registry_entries(id);

-- Index for filtering deprecated entries
CREATE INDEX registry_entries_deprecated_idx ON registry_entries(deprecated) WHERE deprecated = FALSE;

-- Aliases table for accession changes
CREATE TABLE data_source_aliases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data_source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    alias VARCHAR(255) NOT NULL,
    alias_type VARCHAR(50) NOT NULL,  -- 'previous_accession', 'synonym', 'legacy'
    valid_from TIMESTAMPTZ,
    valid_until TIMESTAMPTZ,
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(alias, alias_type)
);

-- Index for fast alias lookups
CREATE INDEX aliases_lookup_idx ON data_source_aliases(alias);
CREATE INDEX aliases_data_source_idx ON data_source_aliases(data_source_id);

-- Example: P01308 was previously P12345
INSERT INTO data_source_aliases (data_source_id, alias, alias_type, valid_from, valid_until, notes) VALUES (
    (SELECT id FROM data_sources WHERE external_id = 'P01308'),
    'P12345',
    'previous_accession',
    '2020-01-01',
    '2024-12-31',
    'Accession changed in UniProt 2024_12 release'
);
```

### 7. Organism as Data Source

```sql
-- New organism_metadata table
CREATE TABLE organism_metadata (
    data_source_id UUID PRIMARY KEY REFERENCES data_sources(id) ON DELETE CASCADE,
    taxonomy_id INTEGER UNIQUE NOT NULL,        -- NCBI Taxonomy: 9606
    scientific_name VARCHAR(255) NOT NULL,      -- "Homo sapiens"
    common_name VARCHAR(255),                   -- "Human"
    rank VARCHAR(50),                           -- "species", "genus", etc.
    lineage TEXT,                               -- Full taxonomic lineage
    ncbi_tax_version VARCHAR(50),               -- NCBI Taxonomy version used
    genome_assembly_id UUID REFERENCES data_sources(id)  -- Optional: link to reference genome
);

-- Indexes
CREATE INDEX organism_metadata_taxonomy_idx ON organism_metadata(taxonomy_id);
CREATE INDEX organism_metadata_scientific_name_idx ON organism_metadata(scientific_name);

-- Update protein_metadata to reference organism data_source
ALTER TABLE protein_metadata
DROP COLUMN taxonomy_id,        -- Remove direct taxonomy_id
ADD COLUMN organism_id UUID REFERENCES data_sources(id);  -- FK to organism data_source

-- Example organism entry
INSERT INTO registry_entries (slug, name, entry_type, organization_id) VALUES (
    'ncbi-taxonomy-9606',
    'Homo sapiens',
    'data_source',
    (SELECT id FROM organizations WHERE slug = 'ncbi')
);

INSERT INTO data_sources (id, source_type) VALUES (
    (SELECT id FROM registry_entries WHERE slug = 'ncbi-taxonomy-9606'),
    'organism'
);

INSERT INTO organism_metadata (data_source_id, taxonomy_id, scientific_name, common_name, rank) VALUES (
    (SELECT id FROM data_sources WHERE source_type = 'organism' AND id = (SELECT id FROM registry_entries WHERE slug = 'ncbi-taxonomy-9606')),
    9606,
    'Homo sapiens',
    'Human',
    'species'
);
```

### 8. Update Source Type Constraints

```sql
-- Update data_sources.source_type constraint
ALTER TABLE data_sources DROP CONSTRAINT IF EXISTS source_type_check;

ALTER TABLE data_sources
ADD CONSTRAINT source_type_check CHECK (source_type IN (
    'protein',
    'genome',
    'organism',
    'bundle',
    'transcript',
    'annotation',
    'structure',
    'pathway',
    'other'
));
```

---

## Versioning Strategy

### Semantic Versioning Rules

**Format**: `MAJOR.MINOR.PATCH`

#### For Proteins

| Change Type | Version Bump | Example | Rationale |
|-------------|--------------|---------|-----------|
| Sequence changed | **MAJOR** (1.0.0 → 2.0.0) | MALWMR... → MALWAR... | Breaks downstream analyses (BLAST, alignment) |
| Length changed | **MAJOR** | 110 AA → 115 AA | Different protein isoform |
| Organism changed | **MAJOR** | Human → Mouse | Different biological entity |
| Gene name changed | **MINOR** (1.0.0 → 1.0.1) | INS → INS1 | Metadata refinement |
| Protein name changed | **MINOR** | "Insulin" → "Insulin precursor" | Annotation update |
| Annotation added | **MINOR** | New GO term | Enrichment, non-breaking |
| Description updated | **PATCH** (1.0.0 → 1.0.1) | Typo fix | Minor correction |
| Cross-reference added | **PATCH** | New PDB structure | External link |

#### For Genomes

| Change Type | Version Bump | Example |
|-------------|--------------|---------|
| Assembly changed | **MAJOR** | GRCh37 → GRCh38 |
| Scaffold added/removed | **MAJOR** | Chromosome added |
| Gene annotation updated | **MINOR** | New gene prediction |
| Metadata updated | **PATCH** | Assembly name clarified |

#### For Organisms

| Change Type | Version Bump | Example |
|-------------|--------------|---------|
| Taxonomy reclassification | **MAJOR** | Species moved to different genus |
| Scientific name changed | **MAJOR** | Taxonomic revision |
| Common name updated | **MINOR** | Better translation |
| Metadata added | **PATCH** | Added habitat information |

#### For Bundles

| Change Type | Version Bump | Example |
|-------------|--------------|---------|
| Dependency added/removed | **MAJOR** | New protein in human:all |
| Dependency version updated | **MINOR** | p01308@1.0 → p01308@2.0 |
| Metadata updated | **PATCH** | Description clarified |

### Auto-Generated Changelog

**Format** (Markdown):

```markdown
# Changelog

## 1.2.3 (2025-02-12)

### MAJOR Changes
- Sequence changed: position 47 (A→W)

### MINOR Changes
- Gene name updated: INS → INS1
- New Gene Ontology term: GO:0005615 (extracellular space)

### PATCH Changes
- Fixed typo in protein name: "Insuline" → "Insulin"
- Updated PDB cross-reference: 6XYZ

### External Mapping
- UniProt Release: 2025_02
- Release Date: 2025-02-12
```

**Implementation**:

```rust
fn generate_changelog(old: &ProteinMetadata, new: &ProteinMetadata) -> String {
    let mut major = vec![];
    let mut minor = vec![];
    let mut patch = vec![];

    // Detect sequence changes
    if old.sequence_id != new.sequence_id {
        let old_seq = get_sequence(old.sequence_id)?;
        let new_seq = get_sequence(new.sequence_id)?;
        let diff = sequence_diff(&old_seq, &new_seq);
        major.push(format!("Sequence changed: {}", diff));
    }

    // Detect gene name changes
    if old.gene_name != new.gene_name {
        minor.push(format!("Gene name updated: {} → {}", old.gene_name, new.gene_name));
    }

    // ... more checks ...

    format_changelog(major, minor, patch)
}
```

---

## Ingestion Pipeline

### Phase 1: Download & Version Discovery

```rust
async fn ingest_uniprot_release(external_version: &str) -> Result<()> {
    // 1. Download from FTP
    let dat_data = ftp.download_dat_file(external_version).await?;

    // 2. Upload raw file to S3
    let s3_key = format!("ingest/uniprot/{}/uniprot_sprot.dat.gz", external_version);
    upload_to_s3(&s3_key, &dat_data).await?;

    // 3. Determine internal version mapping
    let internal_version = get_or_create_version_mapping(
        "uniprot",
        external_version,  // "2025_01"
    ).await?;  // Returns "1.0.0" or "1.1.0", etc.

    // 4. Parse and process proteins
    let entries = parse_dat_file(&dat_data)?;
    process_entries(entries, &internal_version).await?;
}
```

### Phase 2: Process Each Protein

```rust
async fn process_protein_entry(
    entry: &UniProtEntry,
    internal_version: &str,  // "1.0.0"
    external_version: &str,  // "2025_01"
) -> Result<()> {
    // 1. Get or create organism
    let organism_id = get_or_create_organism(entry.taxonomy_id).await?;

    // 2. Check if protein already exists
    let existing = find_protein_by_accession(&entry.accession).await?;

    match existing {
        None => create_new_protein(entry, organism_id, internal_version).await?,
        Some(existing_protein) => {
            update_existing_protein(existing_protein, entry, organism_id).await?
        }
    }
}
```

### Phase 3: Create New Protein

```rust
async fn create_new_protein(
    entry: &UniProtEntry,
    organism_id: Uuid,
    internal_version: &str,
) -> Result<()> {
    // 1. Create registry entry
    let registry_id = create_registry_entry(CreateRegistryEntry {
        slug: entry.accession.to_lowercase(),  // "p01308"
        name: entry.protein_name.clone(),
        description: Some(entry.description.clone()),
        entry_type: "data_source",
        organization_id: uniprot_org_id,
        license_id: cc_by_license_id,
    }).await?;

    // 2. Create data source
    create_data_source(CreateDataSource {
        id: registry_id,
        source_type: "protein",
        external_id: entry.accession.clone(),  // "P01308"
    }).await?;

    // 3. Get or create deduplicated sequence
    let sequence_hash = sha256(&entry.sequence);
    let sequence_id = match get_sequence_by_hash(&sequence_hash).await? {
        Some(seq) => seq.id,  // Reuse existing sequence!
        None => {
            create_protein_sequence(CreateProteinSequence {
                sequence: entry.sequence.clone(),
                sequence_hash: sequence_hash.clone(),
                sequence_length: entry.sequence.len() as i32,
                sequence_md5: md5(&entry.sequence),
            }).await?.id
        }
    };

    // 4. Create protein metadata
    create_protein_metadata(CreateProteinMetadata {
        data_source_id: registry_id,
        accession: entry.accession.clone(),
        entry_name: entry.entry_name.clone(),
        protein_name: entry.protein_name.clone(),
        gene_name: entry.gene_name.clone(),
        organism_id: organism_id,
        sequence_id: sequence_id,
        uniprot_version: external_version.to_string(),
    }).await?;

    // 5. Create version 1.0.0
    let (major, minor, patch) = parse_version(internal_version)?;  // "1.0.0" → (1, 0, 0)
    let version_id = create_version(CreateVersion {
        data_source_id: registry_id,
        version_major: major,
        version_minor: minor,
        version_patch: patch,
        changelog: "Initial version".to_string(),
        external_version: external_version.to_string(),
    }).await?.id;

    // 6. Upload FASTA to S3
    let fasta_content = entry.to_fasta();
    let s3_key = format!(
        "sources/uniprot/{}/{}.{}.{}/{}.fasta",
        entry.accession.to_lowercase(),
        major, minor, patch,
        entry.accession.to_lowercase()
    );
    upload_to_s3(&s3_key, &fasta_content).await?;

    // 7. Create version file
    create_version_file(CreateVersionFile {
        version_id: version_id,
        file_type: "fasta",
        file_format: "fasta",
        s3_key: s3_key.clone(),
        size_bytes: fasta_content.len() as i64,
        checksum: md5(&fasta_content),
        compression: None,
    }).await?;

    Ok(())
}
```

### Phase 4: Update Existing Protein

```rust
async fn update_existing_protein(
    existing: DataSource,
    new_entry: &UniProtEntry,
    organism_id: Uuid,
) -> Result<()> {
    // 1. Get current metadata
    let old_metadata = get_protein_metadata(existing.id).await?;

    // 2. Get or create new sequence
    let new_sequence_hash = sha256(&new_entry.sequence);
    let new_sequence_id = match get_sequence_by_hash(&new_sequence_hash).await? {
        Some(seq) => seq.id,
        None => {
            create_protein_sequence(CreateProteinSequence {
                sequence: new_entry.sequence.clone(),
                sequence_hash: new_sequence_hash.clone(),
                sequence_length: new_entry.sequence.len() as i32,
                sequence_md5: md5(&new_entry.sequence),
            }).await?.id
        }
    };

    // 3. Detect changes and determine version bump
    let version_bump = determine_version_bump(&old_metadata, new_entry, new_sequence_id)?;

    if version_bump == VersionBump::None {
        // No changes, skip versioning
        return Ok(());
    }

    // 4. Get latest version
    let latest_version = get_latest_version(existing.id).await?;

    // 5. Calculate new version
    let (new_major, new_minor, new_patch) = match version_bump {
        VersionBump::Major => (latest_version.major + 1, 0, 0),
        VersionBump::Minor => (latest_version.major, latest_version.minor + 1, 0),
        VersionBump::Patch => (latest_version.major, latest_version.minor, latest_version.patch + 1),
        VersionBump::None => unreachable!(),
    };

    // 6. Generate changelog
    let changelog = generate_changelog(&old_metadata, new_entry, new_sequence_id)?;

    // 7. Update protein metadata (in-place, current version)
    update_protein_metadata(UpdateProteinMetadata {
        data_source_id: existing.id,
        protein_name: new_entry.protein_name.clone(),
        gene_name: new_entry.gene_name.clone(),
        organism_id: organism_id,
        sequence_id: new_sequence_id,
        uniprot_version: external_version.to_string(),
    }).await?;

    // 8. Create new version
    let version_id = create_version(CreateVersion {
        data_source_id: existing.id,
        version_major: new_major,
        version_minor: new_minor,
        version_patch: new_patch,
        changelog: changelog,
        external_version: external_version.to_string(),
    }).await?.id;

    // 9. Upload new FASTA to S3
    let fasta_content = new_entry.to_fasta();
    let s3_key = format!(
        "sources/uniprot/{}/{}.{}.{}/{}.fasta",
        new_entry.accession.to_lowercase(),
        new_major, new_minor, new_patch,
        new_entry.accession.to_lowercase()
    );
    upload_to_s3(&s3_key, &fasta_content).await?;

    // 10. Create version file
    create_version_file(CreateVersionFile {
        version_id: version_id,
        file_type: "fasta",
        file_format: "fasta",
        s3_key: s3_key.clone(),
        size_bytes: fasta_content.len() as i64,
        checksum: md5(&fasta_content),
        compression: None,
    }).await?;

    Ok(())
}
```

### Phase 5: Create Bundle

```rust
async fn create_swissprot_bundle(
    protein_ids: Vec<Uuid>,
    internal_version: &str,
) -> Result<()> {
    // 1. Create bundle registry entry
    let bundle_id = create_registry_entry(CreateRegistryEntry {
        slug: "swissprot",
        name: "UniProt Swiss-Prot (Reviewed)",
        description: Some("Manually annotated and reviewed protein sequences"),
        entry_type: "data_source",
        organization_id: uniprot_org_id,
        license_id: cc_by_license_id,
    }).await?;

    // 2. Create data source
    create_data_source(CreateDataSource {
        id: bundle_id,
        source_type: "bundle",
        external_id: "swissprot",
    }).await?;

    // 3. Get latest version for each protein
    let mut dependencies = vec![];
    for protein_id in protein_ids {
        let latest_version = get_latest_version(protein_id).await?;
        dependencies.push((protein_id, latest_version.id));
    }

    // 4. Create bundle version
    let (major, minor, patch) = parse_version(internal_version)?;
    let version_id = create_version(CreateVersion {
        data_source_id: bundle_id,
        version_major: major,
        version_minor: minor,
        version_patch: patch,
        changelog: format!("Bundle of {} proteins from UniProt {}", dependencies.len(), internal_version),
        external_version: internal_version.to_string(),
    }).await?.id;

    // 5. Create dependencies with version pins
    for (protein_id, protein_version_id) in dependencies {
        create_dependency(CreateDependency {
            dependent_id: bundle_id,
            dependency_id: protein_id,
            dependency_version_id: protein_version_id,  // ✅ Version pinned!
        }).await?;
    }

    // 6. Create manifest.json for S3
    let manifest = json!({
        "bundle": "swissprot",
        "version": format!("{}.{}.{}", major, minor, patch),
        "protein_count": dependencies.len(),
        "dependencies": dependencies.iter().map(|(id, ver_id)| {
            json!({
                "protein_id": id,
                "version_id": ver_id
            })
        }).collect::<Vec<_>>()
    });

    let s3_key = format!("sources/uniprot/swissprot/{}.{}.{}/manifest.json", major, minor, patch);
    upload_to_s3(&s3_key, &manifest.to_string()).await?;

    // 7. Create version file
    create_version_file(CreateVersionFile {
        version_id: version_id,
        file_type: "manifest",
        file_format: "json",
        s3_key: s3_key,
        size_bytes: manifest.to_string().len() as i64,
        checksum: md5(&manifest.to_string()),
        compression: None,
    }).await?;

    Ok(())
}
```

---

## Data Source Organizations

### Organizations to Prepare

```rust
// Seed organizations with versioning rules

// 1. UniProt
create_organization(CreateOrganization {
    slug: "uniprot",
    name: "UniProt Consortium",
    description: "Universal Protein Resource",
    url: "https://www.uniprot.org",
    versioning_rules: UNIPROT_VERSIONING_RULES,
    is_system: true,
}).await?;

// 2. NCBI
create_organization(CreateOrganization {
    slug: "ncbi",
    name: "National Center for Biotechnology Information",
    description: "NCBI databases (RefSeq, GenBank, Taxonomy)",
    url: "https://www.ncbi.nlm.nih.gov",
    versioning_rules: NCBI_VERSIONING_RULES,
    is_system: true,
}).await?;

// 3. Ensembl
create_organization(CreateOrganization {
    slug: "ensembl",
    name: "Ensembl",
    description: "Genome annotation and comparative genomics",
    url: "https://www.ensembl.org",
    versioning_rules: ENSEMBL_VERSIONING_RULES,
    is_system: true,
}).await?;

// 4. PDB
create_organization(CreateOrganization {
    slug: "pdb",
    name: "Protein Data Bank",
    description: "3D structural data of proteins and nucleic acids",
    url: "https://www.rcsb.org",
    versioning_rules: PDB_VERSIONING_RULES,
    is_system: true,
}).await?;

// 5. KEGG
create_organization(CreateOrganization {
    slug: "kegg",
    name: "Kyoto Encyclopedia of Genes and Genomes",
    description: "Biological pathways and genomes",
    url: "https://www.genome.jp/kegg",
    versioning_rules: KEGG_VERSIONING_RULES,
    is_system: true,
}).await?;
```

### Versioning Rules Content

#### UniProt Versioning Rules

```markdown
# UniProt Versioning Rules

## Overview

UniProt data sources in BDP follow **semantic versioning** (MAJOR.MINOR.PATCH) to ensure reproducibility.

## Version Numbering

### MAJOR Version Bump (X.0.0)

Breaking changes that affect downstream analysis:

- ✅ Amino acid sequence changed (substitution, insertion, deletion)
- ✅ Protein length changed
- ✅ Organism reclassified (taxonomy change)
- ✅ Protein merged or split into multiple entries
- ✅ Accession number changed

**Example**: p01308@1.0.0 → p01308@2.0.0 (sequence MALWMR... → MALWAR...)

### MINOR Version Bump (x.Y.0)

Non-breaking enhancements and metadata updates:

- ✅ Gene name changed (INS → INS1)
- ✅ Protein name updated
- ✅ Functional annotation added or updated
- ✅ New Gene Ontology (GO) terms
- ✅ New protein features annotated
- ✅ Cross-references added (PDB, KEGG, etc.)

**Example**: p01308@1.0.0 → p01308@1.1.0 (gene name updated)

### PATCH Version Bump (x.y.Z)

Minor corrections and fixes:

- ✅ Typo fixed in description
- ✅ Cross-reference URL updated
- ✅ Formatting improvements
- ✅ Minor metadata corrections

**Example**: p01308@1.0.0 → p01308@1.0.1 (typo fixed)

## Reproducibility Guarantee

**All versions are immutable.** Version 1.2.3 will always return:
- Same amino acid sequence
- Same metadata (as of that version)
- Same FASTA file from S3

## External Version Mapping

BDP internal versions map to UniProt external releases:

| BDP Version | UniProt Release | Date       |
|-------------|-----------------|------------|
| 1.0.0       | 2025_01         | 2025-01-15 |
| 1.1.0       | 2025_02         | 2025-02-12 |
| 2.0.0       | 2025_03         | 2025-03-15 |

Use `version_mappings` table to lookup external versions.

## Changelog

Every version includes an auto-generated changelog:

```
## 1.2.0 (2025-02-12)

MAJOR Changes:
- Sequence changed: position 47 (A→W)

MINOR Changes:
- Gene name updated: INS → INS1

External: UniProt 2025_02
```

## Data Retrieval

### Pin to Exact Version (Recommended)
```bash
bdp source add "uniprot:p01308@1.2.3"
```

### Use Latest Version (Not Recommended for Production)
```bash
bdp source add "uniprot:p01308@latest"
```

## License

All UniProt data is licensed under **CC-BY-4.0** (Creative Commons Attribution 4.0).

**Attribution required**:
> UniProt Consortium. UniProt: the Universal Protein Knowledgebase in 2025.
> Nucleic Acids Res. 2025 Jan; 53(D1):D609-D618.

## Questions?

See [UniProt Release Notes](https://www.uniprot.org/release-notes) for upstream changes.
```

#### NCBI Versioning Rules

```markdown
# NCBI Versioning Rules

## Overview

NCBI data sources (RefSeq, GenBank, Taxonomy) use semantic versioning.

## RefSeq Proteins

### MAJOR Version Bump
- Sequence changed
- Assembly changed (for genomes)

### MINOR Version Bump
- Annotation updated
- Gene name changed

### PATCH Version Bump
- Metadata corrections

## NCBI Taxonomy (Organisms)

### MAJOR Version Bump
- Taxonomic reclassification (species moved)
- Scientific name changed

### MINOR Version Bump
- Common name updated
- Lineage information added

### PATCH Version Bump
- Typo corrections

## GenBank Genomes

### MAJOR Version Bump
- New assembly (GRCh37 → GRCh38)
- Scaffold added/removed

### MINOR Version Bump
- Gene annotation updated

### PATCH Version Bump
- Metadata corrections

## License

NCBI data is **public domain** (no copyright). No attribution required.

## External Mapping

NCBI RefSeq uses internal versioning (NP_000207.1, .2, .3).
BDP maps these to semantic versions.

| BDP Version | RefSeq Version |
|-------------|----------------|
| 1.0.0       | NP_000207.1    |
| 2.0.0       | NP_000207.2    |
```

#### Ensembl Versioning Rules

```markdown
# Ensembl Versioning Rules

## Overview

Ensembl genome annotations use semantic versioning.

## Genes & Transcripts

### MAJOR Version Bump
- Gene model changed significantly
- Transcript structure changed

### MINOR Version Bump
- Annotation updated
- New isoforms added

### PATCH Version Bump
- Metadata corrections

## Genomes

### MAJOR Version Bump
- New assembly

### MINOR Version Bump
- Gene set updated

## License

Ensembl data is available under **Apache 2.0** and **EBI Terms of Use**.

## External Mapping

Ensembl releases are numbered (e.g., Ensembl 110, 111).
BDP maps these to semantic versions.

| BDP Version | Ensembl Release |
|-------------|-----------------|
| 1.0.0       | Ensembl 110     |
| 1.1.0       | Ensembl 111     |
```

---

## Migration Plan

### Step 1: Drop Old Tables

```sql
-- Drop incorrect proteins table
DROP TABLE IF EXISTS proteins CASCADE;

-- Remove migration file
-- File: migrations/20260118000001_create_proteins_table.sql
-- Action: DELETE from filesystem
```

### Step 2: Create New Tables

```bash
# Create new migration
sqlx migrate add schema_refactor_v2

# File: migrations/YYYYMMDD_schema_refactor_v2.sql
# Contains all schema changes from this document
```

### Step 3: Seed Reference Data

```sql
-- Insert licenses
INSERT INTO licenses (...) VALUES (...);

-- Insert organizations with versioning rules
INSERT INTO organizations (...) VALUES (...);

-- Insert version mappings (if any existing)
INSERT INTO version_mappings (...) VALUES (...);
```

### Step 4: Update Ingestion Code

```rust
// Update: crates/bdp-server/src/ingest/uniprot/idempotent_pipeline.rs
// Remove: INSERT INTO proteins
// Add: New ingestion logic from this document
```

### Step 5: Regenerate SQLx Cache

```bash
# After schema changes
cargo sqlx prepare --workspace

# Commit .sqlx directory
git add .sqlx
git commit -m "chore: update sqlx cache for schema refactor"
```

### Step 6: Rebuild Docker Image

```bash
# Rebuild with new schema
docker-compose down
docker-compose build bdp-server
docker-compose up -d
```

### Step 7: Test Ingestion

```bash
# Run ingestion with new schema
cargo run --example run_historical_ingestion 2025_01

# Verify:
# - registry_entries created
# - data_sources created
# - protein_metadata created with sequence_id
# - protein_sequences deduplicated
# - versions with MAJOR.MINOR.PATCH
# - version_files pointing to S3
```

---

## Implementation Phases

### Phase 1: Schema Migration (Week 1)

**Tasks**:
- [x] Create migration file with all schema changes
- [x] Drop `proteins` table
- [x] Create `licenses` table
- [x] Create `protein_sequences` table
- [x] Create `organism_metadata` table
- [x] Create `data_source_aliases` table
- [x] Update `organizations` with `versioning_rules`
- [x] Update `versions` with semantic versioning
- [x] Update `dependencies` with version pins
- [x] Add deprecation fields to `registry_entries`
- [x] Seed licenses and organizations

**Testing**:
```bash
sqlx migrate run
psql -U bdp -d bdp -c "\dt"
# Verify all tables exist
```

### Phase 2: Ingestion Refactor (Week 2)

**Tasks**:
- [ ] Update `idempotent_pipeline.rs` with new logic
- [ ] Implement `create_new_protein()`
- [ ] Implement `update_existing_protein()`
- [ ] Implement `determine_version_bump()`
- [ ] Implement `generate_changelog()`
- [ ] Implement `get_or_create_organism()`
- [ ] Implement sequence deduplication

**Testing**:
```bash
cargo test --test uniprot_ingestion_tests
```

### Phase 3: Bundle Support (Week 3)

**Tasks**:
- [ ] Implement `create_swissprot_bundle()`
- [ ] Implement version-pinned dependencies
- [ ] Create manifest.json for bundles
- [ ] Test dependency resolution

**Testing**:
```bash
# Verify bundle can resolve exact versions
bdp source add "uniprot:swissprot@1.0.0"
# Should download all proteins at their pinned versions
```

### Phase 4: Organism & Genome Support (Week 4)

**Tasks**:
- [ ] Implement organism ingestion
- [ ] Create NCBI Taxonomy parser
- [ ] Implement genome ingestion (placeholder)
- [ ] Test organism as data source

**Testing**:
```bash
cargo run --example ingest_ncbi_taxonomy
# Verify organisms created as data_sources
```

### Phase 5: Documentation & CLI (Week 5)

**Tasks**:
- [ ] Update API documentation
- [ ] Add versioning rules to organization endpoints
- [ ] Update CLI to show versioning rules
- [ ] Create user guides

**Testing**:
```bash
bdp source info uniprot:p01308
# Should show versioning rules and changelog
```

---

## Success Criteria

### Functional Requirements

- [x] Schema migrated successfully
- [ ] Ingestion creates registry_entries, data_sources, protein_metadata
- [ ] Sequences deduplicated in protein_sequences table
- [ ] Versions use semantic versioning (MAJOR.MINOR.PATCH)
- [ ] Dependencies pin to exact versions
- [ ] Changelog auto-generated for each version
- [ ] Organisms created as data_sources
- [ ] Bundles reference exact dependency versions
- [ ] Deprecated flag prevents usage
- [ ] Aliases resolve to current accessions

### Non-Functional Requirements

- [ ] Storage: 15% reduction via sequence deduplication
- [ ] Performance: Version lookup <50ms
- [ ] Reproducibility: 100% - exact version pinning works
- [ ] Scalability: Supports 571K proteins × 20 releases
- [ ] Documentation: All versioning rules documented

### Test Coverage

- [ ] Unit tests for version bump detection
- [ ] Unit tests for changelog generation
- [ ] Integration tests for full ingestion pipeline
- [ ] Integration tests for bundle dependency resolution
- [ ] Integration tests for alias resolution

---

## File Structure

```
crates/bdp-server/
├── src/
│   ├── ingest/
│   │   ├── framework/
│   │   │   ├── coordinator.rs
│   │   │   ├── worker.rs
│   │   │   └── ...
│   │   └── uniprot/
│   │       ├── idempotent_pipeline.rs  ← Major refactor
│   │       ├── versioning.rs           ← NEW: Version detection
│   │       ├── changelog.rs            ← NEW: Changelog generation
│   │       ├── organisms.rs            ← NEW: Organism ingestion
│   │       └── ...
│   ├── db/
│   │   ├── licenses.rs                 ← NEW
│   │   ├── protein_sequences.rs        ← NEW
│   │   ├── organism_metadata.rs        ← NEW
│   │   ├── aliases.rs                  ← NEW
│   │   └── ...
│   └── ...
├── examples/
│   ├── run_historical_ingestion.rs     ← Update
│   ├── ingest_ncbi_taxonomy.rs         ← NEW
│   └── ...
└── tests/
    ├── versioning_tests.rs              ← NEW
    ├── changelog_tests.rs               ← NEW
    └── ...

migrations/
├── 20260118_schema_refactor_v2.sql      ← NEW: All schema changes
└── ...

docs/
├── schema-refactor-and-ingestion-v2.md  ← This file
└── ...
```

---

## Timeline

| Week | Focus | Deliverable |
|------|-------|-------------|
| 1 | Schema Migration | All tables created, seeded |
| 2 | Ingestion Refactor | Protein ingestion working |
| 3 | Bundle Support | Swissprot bundle with version pins |
| 4 | Organism & Genome | Organism ingestion working |
| 5 | Documentation & Polish | User docs, API docs complete |

**Total**: 5 weeks

---

## Open Questions

1. **Changelog Format**: Markdown vs JSON? (Recommendation: Markdown for human-readability)

2. **Patch Version Granularity**: Do we need it, or is MAJOR.MINOR enough?
   - Recommendation: Keep PATCH for typo fixes, cross-ref updates

3. **Bundle Versioning**: Auto-bump when dependency changes?
   - Recommendation: Yes, MINOR bump when dependency version changes

4. **Organism Completeness**: Ingest all NCBI Taxonomy upfront (millions)?
   - Recommendation: On-demand during protein ingestion

5. **Genome Ingestion**: Full genome assemblies (multi-GB)?
   - Recommendation: Phase 2, not critical path

---

## References

- [UniProt 2025 NAR Paper](https://academic.oup.com/nar/article/53/D1/D609/7902999)
- [Scientific Data: Dataset Versioning](https://www.nature.com/articles/s41597-024-03153-y)
- [NCBI RefSeq 2025](https://academic.oup.com/nar/article/53/D1/D243/7889254)
- [Five Pillars of Reproducibility](https://pmc.ncbi.nlm.nih.gov/articles/PMC10591307/)

---

**End of Document**
