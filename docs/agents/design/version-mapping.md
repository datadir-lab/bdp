# Version Mapping Strategy

How BDP maps external provider versions to internal semantic versions.

## Problem Statement

Different data providers use different versioning schemes:
- **UniProt**: Date-based (2025_01, 2025_02)
- **NCBI**: Mixed (GRCh38.p14, Build 38.1)
- **Ensembl**: Release numbers (Release 111, Release 112)
- **Tools**: Semantic versioning (v2.14.0, 3.1.5)

**Challenges**:
1. Users confused by inconsistent versioning
2. Hard to understand update frequency
3. Difficult to compare versions across providers
4. Semantic meaning varies by provider

## Solution: Dual Versioning

BDP maintains **two versions** for every entry:

1. **Internal Version**: Our opinionated semantic versioning
   - User-facing
   - Predictable, comparable
   - Format: `MAJOR.MINOR.PATCH`

2. **External Version**: Original provider version
   - Preserved for reference
   - Used for provenance tracking
   - Format: Provider-specific

## Version Mapping Rules

### UniProt Mapping

**External Format**: `YYYY_MM` (e.g., `2025_01`)

**Mapping Rules**:
```
Release Date    → Internal Version
2020-01         → 1.0  (first release we sync)
2020-02         → 1.1
2020-03         → 1.2
...
2020-12         → 1.11
2021-01         → 2.0  (new year = major bump)
2021-02         → 2.1
2022-01         → 3.0
```

**Logic**:
- **First release**: `1.0` (configurable oldest_version)
- **Same year**: Increment minor version
- **New year**: Increment major version, reset minor to 0
- **No patches**: UniProt doesn't patch releases

**Implementation**:
```rust
fn map_uniprot_version(external: &str, mappings: &[Mapping]) -> Result<String> {
    // Check if mapping exists
    if let Some(mapping) = mappings.iter().find(|m| m.external == external) {
        return Ok(mapping.internal.clone());
    }

    // Parse external version: "2025_01"
    let parts: Vec<&str> = external.split('_').collect();
    let year: i32 = parts[0].parse()?;
    let month: i32 = parts[1].parse()?;

    // Get latest internal version
    let latest = get_latest_internal_version("uniprot", mappings)?;
    let (latest_major, latest_minor) = parse_version(&latest)?;

    // Get latest year from mappings
    let latest_year = get_latest_year("uniprot", mappings)?;

    // Compute new version
    let new_version = if year > latest_year {
        // New year: bump major
        format!("{}.0", latest_major + 1)
    } else {
        // Same year: bump minor
        format!("{}.{}", latest_major, latest_minor + 1)
    };

    Ok(new_version)
}
```

### NCBI Genome Mapping

**External Format**: `GRCh38.p14` (assembly.patch)

**Mapping Rules**:
```
External       → Internal
GRCh38         → 1.0  (initial release)
GRCh38.p1      → 1.1  (patch 1)
GRCh38.p2      → 1.2  (patch 2)
...
GRCh38.p14     → 1.14 (patch 14)
GRCh39         → 2.0  (new major assembly)
```

**Logic**:
- **New assembly**: Major bump
- **Patch release**: Minor bump
- **No month-based releases**: NCBI patches irregularly

**Implementation**:
```rust
fn map_ncbi_genome_version(external: &str) -> Result<String> {
    // Parse: "GRCh38.p14" or "GRCh38"
    let re = Regex::new(r"^GRCh(\d+)(\.p(\d+))?$")?;
    let caps = re.captures(external)
        .ok_or_else(|| anyhow!("Invalid NCBI genome version"))?;

    let assembly: i32 = caps[1].parse()?;
    let patch: i32 = caps.get(3)
        .map(|m| m.as_str().parse().unwrap())
        .unwrap_or(0);

    // First assembly is 1.0, patches increment minor
    let major = assembly - 37;  // GRCh38 → major 1
    let minor = patch;

    Ok(format!("{}.{}", major, minor))
}
```

### Ensembl Mapping

**External Format**: `Release 111`, `Release 112`

**Mapping Rules**:
```
External       → Internal
Release 100    → 1.0  (first we sync)
Release 101    → 1.1
Release 102    → 1.2
...
Release 110    → 1.10
Release 111    → 1.11
```

**Logic**:
- **Sequential releases**: Increment minor
- **Major version**: Every 100 releases (unlikely)

**Implementation**:
```rust
fn map_ensembl_version(external: &str, base_release: i32) -> Result<String> {
    // Parse: "Release 111"
    let release: i32 = external
        .strip_prefix("Release ")
        .ok_or_else(|| anyhow!("Invalid Ensembl version"))?
        .parse()?;

    // Compute major/minor relative to base release
    let diff = release - base_release;
    let major = (diff / 100) + 1;
    let minor = diff % 100;

    Ok(format!("{}.{}", major, minor))
}
```

### Tool Semantic Versioning

**External Format**: `v2.14.0`, `3.1.5`

**Mapping Rules**: Use as-is (already semantic)

```
External       → Internal
v2.14.0        → 2.14.0  (strip 'v' prefix)
3.1.5          → 3.1.5   (use directly)
```

**Implementation**:
```rust
fn map_tool_version(external: &str) -> Result<String> {
    // Strip common prefixes
    let version = external
        .trim_start_matches('v')
        .trim_start_matches('V');

    // Validate semantic version
    Version::parse(version)?;

    Ok(version.to_string())
}
```

## Database Storage

### Version Mappings Table

```sql
CREATE TABLE version_mappings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_slug VARCHAR(100) NOT NULL,
    external_version VARCHAR(64) NOT NULL,
    internal_version VARCHAR(64) NOT NULL,
    release_date DATE,
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(organization_slug, external_version),
    UNIQUE(organization_slug, internal_version)
);

-- Indexes
CREATE INDEX version_mappings_org_idx ON version_mappings(organization_slug);
CREATE INDEX version_mappings_external_idx ON version_mappings(external_version);
CREATE INDEX version_mappings_internal_idx ON version_mappings(internal_version);
CREATE INDEX version_mappings_date_idx ON version_mappings(release_date);
```

### Example Data

```sql
INSERT INTO version_mappings (organization_slug, external_version, internal_version, release_date) VALUES
    -- UniProt
    ('uniprot', '2020_01', '1.0', '2020-01-15'),
    ('uniprot', '2020_02', '1.1', '2020-02-12'),
    ('uniprot', '2020_03', '1.2', '2020-03-11'),
    ('uniprot', '2021_01', '2.0', '2021-01-13'),
    ('uniprot', '2025_01', '6.0', '2025-01-15'),

    -- NCBI
    ('ncbi', 'GRCh38', '1.0', '2013-12-17'),
    ('ncbi', 'GRCh38.p1', '1.1', '2014-06-25'),
    ('ncbi', 'GRCh38.p14', '1.14', '2022-02-03'),

    -- Ensembl
    ('ensembl', 'Release 100', '1.0', '2020-04-24'),
    ('ensembl', 'Release 111', '1.11', '2024-01-19'),

    -- NCBI BLAST
    ('ncbi', 'v2.13.0', '2.13.0', '2022-05-01'),
    ('ncbi', 'v2.14.0', '2.14.0', '2023-11-15');
```

## API Integration

### Query Mapping

```http
GET /version-mappings?org=uniprot&external=2025_01
```

Response:
```json
{
  "success": true,
  "data": {
    "organization": "uniprot",
    "external_version": "2025_01",
    "internal_version": "6.0",
    "release_date": "2025-01-15"
  }
}
```

### Reverse Lookup

```http
GET /version-mappings?org=uniprot&internal=6.0
```

Response:
```json
{
  "success": true,
  "data": {
    "organization": "uniprot",
    "external_version": "2025_01",
    "internal_version": "6.0",
    "release_date": "2025-01-15"
  }
}
```

## User Experience

### In CLI

Users work with internal versions:

```bash
# Add source using internal version
bdp source add uniprot:P01308-fasta@1.0

# Pull shows both versions
bdp pull
# Downloading uniprot:P01308-fasta@1.0 (external: 2025_01)...
```

### In Web Interface

```
UniProt: P01308 (Insulin)

Version: 1.0
External Version: 2025_01
Release Date: 2025-01-15
```

### In API Responses

```json
{
  "version": "1.0",
  "external_version": "2025_01",
  "release_date": "2025-01-15"
}
```

## Cron Job Integration

### Auto-Mapping on Ingest

When cron job discovers new UniProt release:

```rust
async fn ingest_uniprot_release(external_version: &str) -> Result<()> {
    // Check if already mapped
    if let Some(mapping) = get_mapping("uniprot", external_version).await? {
        tracing::info!("Version already mapped: {} → {}",
            external_version, mapping.internal);
        return Ok(());
    }

    // Generate new internal version
    let internal_version = map_uniprot_version(
        external_version,
        &get_all_mappings("uniprot").await?
    )?;

    // Store mapping
    insert_mapping(
        "uniprot",
        external_version,
        &internal_version,
        parse_release_date(external_version)?
    ).await?;

    tracing::info!("Created version mapping: {} → {}",
        external_version, internal_version);

    // Continue with protein ingestion
    ingest_proteins(external_version, &internal_version).await?;

    Ok(())
}
```

## Configuration

### Server Config

```toml
# config/server.toml

[versioning]
# Base versions for each organization
[versioning.uniprot]
oldest_external = "2020_01"  # Don't sync older
first_internal = "1.0"       # Starting version

[versioning.ncbi]
oldest_external = "GRCh38"
first_internal = "1.0"

[versioning.ensembl]
oldest_external = "Release 100"
first_internal = "1.0"
base_release = 100  # For computing offsets
```

## Version Comparison

### Semantic Ordering

Internal versions can be compared semantically:

```rust
use semver::Version;

fn compare_versions(v1: &str, v2: &str) -> Ordering {
    let ver1 = Version::parse(v1).unwrap();
    let ver2 = Version::parse(v2).unwrap();
    ver1.cmp(&ver2)
}

// Example
assert!(compare_versions("1.0", "1.5") == Ordering::Less);
assert!(compare_versions("2.0", "1.5") == Ordering::Greater);
```

### Latest Version Query

```sql
-- Get latest version for an entry
SELECT version, external_version, release_date
FROM versions
WHERE entry_id = ?
ORDER BY
    CAST(SUBSTRING(version, 1, POSITION('.' IN version) - 1) AS INTEGER) DESC,
    CAST(SUBSTRING(version, POSITION('.' IN version) + 1) AS INTEGER) DESC
LIMIT 1;
```

## Future Enhancements

### Version Ranges (Post-MVP)

```yaml
sources:
  - "uniprot:P01308-fasta@^1.0"   # >=1.0, <2.0
  - "uniprot:P01308-fasta@~1.5"   # >=1.5, <1.6
  - "uniprot:P01308-fasta@>=1.0"  # >=1.0
```

Resolves to highest matching version:
```
^1.0 matches: 1.0, 1.1, 1.5, 1.11 (but not 2.0)
→ Resolves to 1.11
```

### Named Versions

```yaml
sources:
  - "uniprot:P01308-fasta@latest"
  - "ncbi:GRCh38-fasta@stable"
```

Aliases map to specific versions:
```sql
CREATE TABLE version_aliases (
    id UUID PRIMARY KEY,
    organization_slug VARCHAR(100),
    alias VARCHAR(50),  -- 'latest', 'stable', 'lts'
    internal_version VARCHAR(64),
    created_at TIMESTAMPTZ,
    UNIQUE(organization_slug, alias)
);
```

### Version Deprecation

```sql
ALTER TABLE versions ADD COLUMN deprecated BOOLEAN DEFAULT FALSE;
ALTER TABLE versions ADD COLUMN deprecation_reason TEXT;
ALTER TABLE versions ADD COLUMN superseded_by VARCHAR(64);
```

Warning when using deprecated version:
```
⚠ Warning: uniprot:P01308@1.0 is deprecated
  Reason: Superseded by newer annotation
  Recommended: Use @1.5 instead
```

## Edge Cases

### Same External, Different Context

**Problem**: Same external version, different meaning
- NCBI: `v2.14.0` for BLAST tool
- NCBI: `v2.14.0` for different tool

**Solution**: Scoped by entry
```sql
-- Mapping is unique per (organization, entry, external_version)
-- Not just (organization, external_version)
```

### Missing Release Dates

**Problem**: Historical data without known release dates

**Solution**: Estimate or mark as unknown
```sql
INSERT INTO version_mappings
    (organization_slug, external_version, internal_version, release_date)
VALUES
    ('uniprot', '2015_01', '0.1', NULL);  -- NULL = unknown
```

### Out-of-Order Ingestion

**Problem**: Discover older release after newer ones ingested

**Solution**: Version mapping is independent of ingestion order
```rust
// Always compute based on external version, not insertion order
let internal = map_version(external);  // Deterministic
```

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_uniprot_version_mapping() {
    let mappings = vec![
        Mapping { external: "2020_01".into(), internal: "1.0".into() },
        Mapping { external: "2020_02".into(), internal: "1.1".into() },
    ];

    assert_eq!(
        map_uniprot_version("2020_03", &mappings).unwrap(),
        "1.2"
    );

    assert_eq!(
        map_uniprot_version("2021_01", &mappings).unwrap(),
        "2.0"
    );
}

#[test]
fn test_ncbi_genome_mapping() {
    assert_eq!(map_ncbi_genome_version("GRCh38").unwrap(), "1.0");
    assert_eq!(map_ncbi_genome_version("GRCh38.p1").unwrap(), "1.1");
    assert_eq!(map_ncbi_genome_version("GRCh38.p14").unwrap(), "1.14");
    assert_eq!(map_ncbi_genome_version("GRCh39").unwrap(), "2.0");
}
```

### Integration Tests

```rust
#[sqlx::test]
async fn test_version_mapping_persistence(pool: PgPool) {
    let mapping = VersionMapping {
        organization_slug: "uniprot".into(),
        external_version: "2025_01".into(),
        internal_version: "6.0".into(),
        release_date: Some(NaiveDate::from_ymd(2025, 1, 15)),
    };

    insert_mapping(&pool, &mapping).await.unwrap();

    let retrieved = get_mapping(&pool, "uniprot", "2025_01")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(retrieved.internal_version, "6.0");
}
```

## Related Documents

- [Database Schema](./database-schema.md) - version_mappings table
- [API Design](./api-design.md) - Version endpoints
- [UniProt Ingestion](./uniprot-ingestion.md) - Auto-mapping during ingest
- [File Formats](./file-formats.md) - Version in lockfile
