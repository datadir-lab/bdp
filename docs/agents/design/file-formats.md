# File Formats Specification

Detailed specification for BDP configuration and lockfile formats.

## Overview

BDP uses three primary files for project management:

1. **`bdp.yml`**: Human-edited project manifest (committed to git)
2. **`bdl.lock`**: Generated lockfile with resolved versions and checksums (committed to git)
3. **`.bdp/resolved-dependencies.json`**: Machine-generated full dependency tree (gitignored)

## Design Principles

1. **Separate Concerns**: User intent (`bdp.yml`) vs resolved state (`bdl.lock`)
2. **Git-Friendly**: Lockfile must be small and readable
3. **Reproducible**: Lockfile ensures exact same sources across machines
4. **Format Granularity**: Each format is a separate entry for independent checksums
5. **Portable**: No absolute paths, no machine-specific URLs

## bdp.yml (Project Manifest)

Human-edited file declaring project dependencies.

### Structure

```yaml
project:
  name: string              # Project name
  version: string           # Project version (semver)
  description?: string      # Optional project description

sources:
  - string[]               # List of data source specifications

tools:
  - string[]               # List of tool specifications
```

### Source Specification Format

```
{organization}:{source-name}-{format}@{version}
```

**Components**:
- `organization`: Provider slug (e.g., `uniprot`, `ncbi`)
- `source-name`: Data source identifier (e.g., `P01308`, `all`, `GRCh38`)
- `format`: File format (e.g., `fasta`, `xml`, `json`)
- `version`: Our semantic version (e.g., `1.0`, `2.1`)

**Examples**:
```yaml
sources:
  - "uniprot:P01308-fasta@1.0"          # Single protein, FASTA format
  - "uniprot:P01308-xml@1.0"            # Same protein, XML format
  - "uniprot:all-fasta@1.0"             # Aggregate: all UniProt proteins
  - "ncbi:GRCh38-fasta@2.0"             # Human reference genome
  - "ensembl:ENSG00000139618-json@1.0"  # BRCA2 gene annotation
```

### Tool Specification Format

```
{organization}:{tool-name}@{version}
```

**Examples**:
```yaml
tools:
  - "ncbi:blast@2.14.0"
  - "ebi:clustalw@2.1.0"
  - "broad:gatk@4.3.0"
```

### Complete Example

```yaml
project:
  name: "Cancer Genomics Research"
  version: "1.0.0"
  description: "TP53 pathway analysis in breast cancer"

sources:
  # Individual proteins
  - "uniprot:P01308-fasta@1.0"  # Insulin
  - "uniprot:P01308-xml@1.0"    # Insulin (XML for metadata)
  - "uniprot:P04637-fasta@1.0"  # TP53 tumor suppressor

  # Reference genome
  - "ncbi:GRCh38-fasta@2.0"

  # Aggregate data source (all human proteins)
  - "uniprot:homo-sapiens-fasta@1.0"

tools:
  - "ncbi:blast@2.14.0"
  - "broad:gatk@4.3.0"
```

### Validation Rules

1. Project name must be 1-255 characters
2. Version must follow semantic versioning
3. Source/tool strings must match format regex: `^[a-z0-9-]+:[a-zA-Z0-9_-]+-[a-z0-9]+@[0-9]+\.[0-9]+$`
4. No duplicate entries
5. Format must be alphanumeric lowercase

## bdl.lock (Lockfile)

Machine-generated file with resolved versions, checksums, and metadata. Small enough to commit to git.

### Structure

```yaml
lockfileVersion: number      # Lockfile format version (currently 1)
generated: string            # ISO 8601 timestamp

sources:
  "{spec}":                  # Full source specification as key
    resolved: string         # Resolved identifier
    format: string           # File format
    checksum: string         # SHA-256 hash
    size: number             # Size in bytes
    external_version: string # Original provider version
    dependency_count?: number    # If has dependencies
    dependencies_resolved?: bool # If dependencies cached locally

tools:
  "{spec}":                  # Full tool specification as key
    resolved: string         # Resolved identifier
    checksum: string         # SHA-256 hash
    size: number             # Size in bytes
```

### Complete Example

```yaml
lockfileVersion: 1
generated: "2024-01-16T12:30:45Z"

sources:
  "uniprot:P01308-fasta@1.0":
    resolved: "uniprot:P01308@1.0"
    format: fasta
    checksum: "sha256-1a2b3c4d5e6f7890abcdef1234567890abcdef1234567890abcdef1234567890"
    size: 4096
    external_version: "2025_01"

  "uniprot:P01308-xml@1.0":
    resolved: "uniprot:P01308@1.0"
    format: xml
    checksum: "sha256-9876543210fedcba0987654321fedcba0987654321fedcba0987654321fedcba"
    size: 16384
    external_version: "2025_01"

  "uniprot:P04637-fasta@1.0":
    resolved: "uniprot:P04637@1.0"
    format: fasta
    checksum: "sha256-abcd1234efgh5678ijkl9012mnop3456qrst7890uvwx1234yz567890abcdef12"
    size: 8192
    external_version: "2025_01"

  "uniprot:all-fasta@1.0":
    resolved: "uniprot:all@1.0"
    format: fasta
    checksum: "sha256-aggregate1234567890abcdef1234567890abcdef1234567890abcdef12345678"
    size: 4294967296
    external_version: "2025_01"
    dependency_count: 567239
    dependencies_resolved: true

  "ncbi:GRCh38-fasta@2.0":
    resolved: "ncbi:GRCh38@2.0"
    format: fasta
    checksum: "sha256-genome1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
    size: 3200000000
    external_version: "GRCh38.p14"

tools:
  "ncbi:blast@2.14.0":
    resolved: "ncbi:blast@2.14.0"
    checksum: "sha256-blast1234567890abcdef1234567890abcdef1234567890abcdef12345678901"
    size: 104857600

  "broad:gatk@4.3.0":
    resolved: "broad:gatk@4.3.0"
    checksum: "sha256-gatk1234567890abcdef1234567890abcdef1234567890abcdef1234567890a"
    size: 524288000
```

### Key Design Decisions

**1. No URLs in Lockfile**
- URLs can change (server migration, CDN updates)
- CLI resolves download URL from registry at pull time
- Uses: `{organization}:{name}@{version}` + checksum

**2. No Cache Paths**
- Teams may use shared cache or individual caches
- Cache location is configurable per user/machine
- Lockfile is portable across environments

**3. Aggregate Dependency Handling**
- For sources with many dependencies (e.g., 567k proteins):
  - Lockfile stores count and summary checksum
  - Full dependency tree in `.bdp/resolved-dependencies.json`
  - `dependencies_resolved: true` indicates full tree is cached

**4. Format Separation**
- Each format is a separate entry with own checksum
- Allows partial downloads (FASTA only, skip XML)
- Independent integrity verification

### Checksum Format

Always `sha256-{hex}` format:
```
sha256-1a2b3c4d5e6f7890abcdef1234567890abcdef1234567890abcdef1234567890
```

### Lockfile Generation

Generated by `bdp pull` or `bdp install`:

```rust
async fn generate_lockfile(manifest: &Manifest) -> Result<Lockfile> {
    let mut lockfile = Lockfile {
        version: 1,
        generated: Utc::now(),
        sources: HashMap::new(),
        tools: HashMap::new(),
    };

    // Resolve each source
    for source_spec in &manifest.sources {
        let resolved = api_client.resolve_source(source_spec).await?;

        lockfile.sources.insert(
            source_spec.clone(),
            SourceEntry {
                resolved: format!("{}:{}@{}", resolved.org, resolved.name, resolved.version),
                format: resolved.format,
                checksum: resolved.checksum,
                size: resolved.size,
                external_version: resolved.external_version,
                dependency_count: resolved.dependency_count,
                dependencies_resolved: resolved.has_dependencies,
            }
        );

        // If has dependencies, fetch and cache full tree
        if resolved.has_dependencies {
            fetch_and_cache_dependencies(source_spec, &resolved).await?;
        }
    }

    Ok(lockfile)
}
```

## .bdp/resolved-dependencies.json

Machine-generated file containing full dependency trees for aggregate sources. **Gitignored**.

### Purpose

For sources like `uniprot:all@1.0` with 567,239 dependencies:
- Too large for lockfile (would be 50MB+)
- Needed for actual downloads
- Cached locally after first resolution
- Can be regenerated from server

### Structure

```json
{
  "{source-spec}": {
    "resolved_at": "ISO 8601 timestamp",
    "tree_checksum": "sha256-...",
    "total_count": number,
    "total_size": number,
    "dependencies": [
      {
        "source": "string",
        "checksum": "string",
        "size": number
      }
    ]
  }
}
```

### Example

```json
{
  "uniprot:all-fasta@1.0": {
    "resolved_at": "2024-01-16T12:30:45Z",
    "tree_checksum": "sha256-aggregate1234567890abcdef1234567890abcdef1234567890abcdef12345678",
    "total_count": 567239,
    "total_size": 4294967296,
    "dependencies": [
      {
        "source": "uniprot:P01308-fasta@1.0",
        "checksum": "sha256-1a2b3c4d...",
        "size": 4096
      },
      {
        "source": "uniprot:P04637-fasta@1.0",
        "checksum": "sha256-9876543...",
        "size": 8192
      },
      {
        "source": "uniprot:P12345-fasta@1.0",
        "checksum": "sha256-abcd1234...",
        "size": 6144
      }
      // ... 567,236 more entries
    ]
  },

  "uniprot:homo-sapiens-fasta@1.0": {
    "resolved_at": "2024-01-16T12:35:20Z",
    "tree_checksum": "sha256-human1234567890abcdef1234567890abcdef1234567890abcdef123456789",
    "total_count": 20438,
    "total_size": 167772160,
    "dependencies": [
      {
        "source": "uniprot:P01308-fasta@1.0",
        "checksum": "sha256-1a2b3c4d...",
        "size": 4096
      }
      // ... 20,437 more human proteins
    ]
  }
}
```

### Generation

```rust
async fn fetch_and_cache_dependencies(
    source_spec: &str,
    resolved: &ResolvedSource
) -> Result<()> {
    let mut all_deps = Vec::new();
    let mut page = 1;

    // Fetch paginated dependencies from server
    loop {
        let resp = api_client.get_dependencies(
            &resolved.org,
            &resolved.name,
            &resolved.version,
            page,
            1000  // 1000 per page
        ).await?;

        all_deps.extend(resp.dependencies);

        if page >= resp.pagination.total_pages {
            break;
        }
        page += 1;
    }

    // Compute aggregate checksum
    let tree_checksum = compute_tree_checksum(&all_deps);

    // Verify against lockfile
    if tree_checksum != resolved.checksum {
        bail!("Dependency tree checksum mismatch!");
    }

    // Save to .bdp/resolved-dependencies.json
    let cache_entry = DependencyCache {
        resolved_at: Utc::now(),
        tree_checksum,
        total_count: all_deps.len(),
        total_size: all_deps.iter().map(|d| d.size).sum(),
        dependencies: all_deps,
    };

    save_dependency_cache(source_spec, &cache_entry).await?;

    Ok(())
}
```

### Tree Checksum Computation

```rust
fn compute_tree_checksum(deps: &[Dependency]) -> String {
    let mut hasher = Sha256::new();

    // Sort dependencies for deterministic hash
    let mut sorted = deps.to_vec();
    sorted.sort_by(|a, b| a.source.cmp(&b.source));

    // Hash each dependency's checksum
    for dep in sorted {
        hasher.update(dep.checksum.as_bytes());
    }

    format!("sha256-{}", hex::encode(hasher.finalize()))
}
```

## File Locations

```
project/
├── bdp.yml                              # Project manifest (committed)
├── bdl.lock                             # Lockfile (committed)
└── .bdp/
    ├── cache/                           # Downloaded files (gitignored)
    ├── bdp.db                           # SQLite tracking (gitignored)
    ├── resolved-dependencies.json       # Dependency trees (gitignored)
    └── audit.log                        # Integrity checks (gitignored)
```

## .gitignore

```gitignore
# BDP cache and runtime files
.bdp/cache/
.bdp/bdp.db
.bdp/bdp.db-shm
.bdp/bdp.db-wal
.bdp/resolved-dependencies.json
.bdp/audit.log

# Commit these:
# bdp.yml
# bdl.lock
```

## CLI Workflow

### Adding a Source

```bash
$ bdp source add uniprot:P01308-fasta@1.0
```

**Actions**:
1. Parse specification
2. Validate format
3. Check if already in `bdp.yml`
4. Append to `sources` array
5. Save `bdp.yml`

**bdp.yml after**:
```yaml
project:
  name: "my-project"
  version: "0.1.0"

sources:
  - "uniprot:P01308-fasta@1.0"
```

### Pulling Sources

```bash
$ bdp pull
```

**Actions**:
1. Read `bdp.yml`
2. For each source:
   a. Query registry API for metadata
   b. Resolve version if needed (e.g., `@latest` → `@1.0`)
   c. Fetch checksums and sizes
   d. If has dependencies, fetch dependency tree
   e. Save to `.bdp/resolved-dependencies.json`
3. Generate/update `bdl.lock`
4. Download files to cache
5. Verify checksums
6. Update `.bdp/bdp.db` tracking

### Verifying Integrity

```bash
$ bdp audit
```

**Actions**:
1. Read `bdl.lock`
2. For each source:
   a. Check if cached
   b. Compute checksum of cached file
   c. Compare with `bdl.lock` checksum
   d. Log results to `.bdp/audit.log`
3. Report any mismatches

## Version Constraints (Future)

Currently using exact versions (`@1.0`). Future support for ranges:

```yaml
sources:
  - "uniprot:P01308-fasta@^1.0"   # >=1.0.0, <2.0.0
  - "uniprot:P04637-fasta@~1.5"   # >=1.5.0, <1.6.0
  - "ncbi:GRCh38-fasta@>=2.0"     # >=2.0.0
```

Lockfile always resolves to exact version:
```yaml
"uniprot:P01308-fasta@^1.0":
  resolved: "uniprot:P01308-fasta@1.5"  # Highest matching version
  # ...
```

## Format Migration

If lockfile format changes, include version field:

```yaml
lockfileVersion: 2  # New version
generated: "2025-01-16T12:30:45Z"

# ... new format fields
```

CLI handles migration automatically:
```rust
match lockfile.version {
    1 => migrate_v1_to_v2(lockfile),
    2 => Ok(lockfile),
    _ => bail!("Unsupported lockfile version"),
}
```

## Comparison with Other Package Managers

| Feature | npm | cargo | conda | BDP |
|---------|-----|-------|-------|-----|
| Manifest | package.json | Cargo.toml | environment.yml | bdp.yml |
| Lockfile | package-lock.json | Cargo.lock | conda-lock.yml | bdl.lock |
| URLs in lock | No | No | Yes | No |
| Paths in lock | No | No | No | No |
| Dependency tree | Nested | Flat | Flat | Hybrid (flat + cached) |
| File size | Small | Small | Medium | Small (lock) + Large (cache) |

**Key Difference**: BDP separates small lockfile (git-friendly) from large dependency cache (local only).

## Related Documents

- [Database Schema](./database-schema.md) - Backend data model
- [API Design](./api-design.md) - REST endpoints for resolution
- [Cache Strategy](./cache-strategy.md) - Local caching and sharing
- [Dependency Resolution](./dependency-resolution.md) - Resolution algorithm
