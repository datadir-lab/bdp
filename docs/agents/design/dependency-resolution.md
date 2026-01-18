# Dependency Resolution

How BDP resolves and manages dependencies for data sources and tools.

## Overview

Unlike traditional package managers where dependencies are code libraries, BDP dependencies represent:
1. **Data relationships**: Aggregate sources linking to individual proteins
2. **Required datasets**: Tools depending on reference databases
3. **Composite datasets**: Curated collections referencing multiple sources

## Dependency Model

### Types of Dependencies

**1. Aggregate Dependencies**
- One source depends on many others
- Example: `uniprot:all@1.0` → all UniProt proteins
- Example: `uniprot:homo-sapiens@1.0` → all human proteins

**2. Tool Dependencies**
- Tool requires specific data sources
- Example: `blast@2.14.0` → reference databases
- Future feature (MVP focuses on data sources)

**3. Curated Collections**
- User-created collection of specific sources
- Example: `smith-lab:cancer-proteins@1.0` → curated protein set
- Future feature (post-MVP)

### Dependency Declaration

In database:
```sql
-- uniprot:all@1.0 depends on all proteins
INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version)
SELECT
    (SELECT id FROM versions WHERE entry_id = 'all-entry-id' AND version = '1.0'),
    protein_entry_id,
    '1.0'
FROM protein_entries;
```

## Resolution Process

### Step 1: Parse Manifest

```yaml
# bdp.yml
sources:
  - "uniprot:P01308-fasta@1.0"
  - "uniprot:all-fasta@1.0"
```

Parse into structured format:
```rust
struct SourceSpec {
    organization: String,   // "uniprot"
    name: String,           // "P01308" or "all"
    version: String,        // "1.0"
    format: String,         // "fasta"
}
```

### Step 2: Query Registry

For each source spec, query API:
```http
GET /sources/uniprot/P01308/1.0
GET /sources/uniprot/all/1.0
```

Response includes:
- Checksum
- Size
- `has_dependencies: bool`
- `dependency_count: number`

### Step 3: Resolve Dependencies

If `has_dependencies: true`, fetch dependency tree:

```http
GET /sources/uniprot/all/1.0/dependencies?format=fasta&page=1&limit=1000
```

**Pagination handling**:
```rust
async fn fetch_all_dependencies(
    org: &str,
    name: &str,
    version: &str,
    format: &str
) -> Result<Vec<Dependency>> {
    let mut all_deps = Vec::new();
    let mut page = 1;

    loop {
        let resp = api_client
            .get_dependencies(org, name, version, format, page, 1000)
            .await?;

        all_deps.extend(resp.dependencies);

        if page >= resp.pagination.pages {
            break;
        }
        page += 1;
    }

    Ok(all_deps)
}
```

### Step 4: Build Dependency Graph

Create directed acyclic graph (DAG):

```rust
struct DependencyGraph {
    nodes: HashMap<String, Node>,
    edges: Vec<Edge>,
}

struct Node {
    source_spec: String,    // "uniprot:P01308-fasta@1.0"
    checksum: String,
    size: u64,
    resolved: bool,
}

struct Edge {
    from: String,           // "uniprot:all-fasta@1.0"
    to: String,             // "uniprot:P01308-fasta@1.0"
}
```

Example graph:
```
bdp.yml sources:
  ├─ uniprot:P01308-fasta@1.0 (direct)
  └─ uniprot:all-fasta@1.0 (direct)
      ├─ uniprot:P01308-fasta@1.0 (duplicate, dedupe)
      ├─ uniprot:P04637-fasta@1.0 (new)
      ├─ uniprot:P12345-fasta@1.0 (new)
      └─ ... (567,236 more)
```

### Step 5: Deduplication

Remove duplicate dependencies:

```rust
fn deduplicate_dependencies(graph: &DependencyGraph) -> Vec<String> {
    let mut unique = HashSet::new();

    // Traverse graph depth-first
    for root in &graph.roots {
        visit_node(root, &graph, &mut unique);
    }

    unique.into_iter().collect()
}

fn visit_node(
    node_id: &str,
    graph: &DependencyGraph,
    visited: &mut HashSet<String>
) {
    if visited.contains(node_id) {
        return;  // Already processed
    }

    visited.insert(node_id.to_string());

    // Visit dependencies recursively
    for edge in graph.edges.iter().filter(|e| e.from == node_id) {
        visit_node(&edge.to, graph, visited);
    }
}
```

**Result**: Flat list of unique sources to download

### Step 6: Generate Lockfile

```rust
async fn generate_lockfile(
    manifest: &Manifest,
    resolved: &ResolvedSources
) -> Result<Lockfile> {
    let mut lockfile = Lockfile {
        version: 1,
        generated: Utc::now(),
        sources: HashMap::new(),
        tools: HashMap::new(),
    };

    // Add direct sources from manifest
    for source_spec in &manifest.sources {
        let resolved_source = resolved.get(source_spec)?;

        lockfile.sources.insert(
            source_spec.clone(),
            LockEntry {
                resolved: format!("{}:{}@{}",
                    resolved_source.org,
                    resolved_source.name,
                    resolved_source.version
                ),
                format: resolved_source.format,
                checksum: resolved_source.checksum,
                size: resolved_source.size,
                external_version: resolved_source.external_version,
                dependency_count: resolved_source.dependency_count,
                dependencies_resolved: resolved_source.has_dependencies,
            }
        );
    }

    Ok(lockfile)
}
```

### Step 7: Cache Dependencies

For aggregate sources, save full dependency tree to `.bdp/resolved-dependencies.json`:

```rust
async fn cache_dependency_tree(
    source_spec: &str,
    dependencies: &[Dependency]
) -> Result<()> {
    let tree_checksum = compute_tree_checksum(dependencies);

    let cache_entry = DependencyCache {
        resolved_at: Utc::now(),
        tree_checksum,
        total_count: dependencies.len(),
        total_size: dependencies.iter().map(|d| d.size).sum(),
        dependencies: dependencies.to_vec(),
    };

    let cache_path = Path::new(".bdp/resolved-dependencies.json");
    let mut cache_map: HashMap<String, DependencyCache> = if cache_path.exists() {
        serde_json::from_str(&tokio::fs::read_to_string(cache_path).await?)?
    } else {
        HashMap::new()
    };

    cache_map.insert(source_spec.to_string(), cache_entry);

    tokio::fs::write(
        cache_path,
        serde_json::to_string_pretty(&cache_map)?
    ).await?;

    Ok(())
}
```

## Conflict Resolution

### Version Conflicts

**Scenario**: Two dependencies require different versions of same source

```yaml
sources:
  - "lab-a:collection-a-fasta@1.0"  # depends on uniprot:P01308-fasta@1.0
  - "lab-b:collection-b-fasta@1.0"  # depends on uniprot:P01308-fasta@1.5
```

**Resolution Strategy**:
1. **Exact Match Required**: BDP uses exact versions (no ranges in MVP)
2. **Conflict Detection**: CLI detects version mismatch
3. **User Intervention**: Report error and ask user to resolve

```rust
fn detect_conflicts(graph: &DependencyGraph) -> Vec<Conflict> {
    let mut versions = HashMap::new();
    let mut conflicts = Vec::new();

    for node in &graph.nodes {
        let key = format!("{}:{}", node.org, node.name);

        if let Some(existing) = versions.get(&key) {
            if existing != &node.version {
                conflicts.push(Conflict {
                    source: key,
                    versions: vec![existing.clone(), node.version.clone()],
                });
            }
        } else {
            versions.insert(key, node.version.clone());
        }
    }

    conflicts
}
```

**Error Message**:
```
Error: Version conflict detected

  uniprot:P01308
    - Required as -fasta@1.0 by lab-a:collection-a@1.0
    - Required as -fasta@1.5 by lab-b:collection-b@1.0

Resolution:
  Update one collection to use compatible version, or
  use only one collection.
```

### Format Conflicts

**Scenario**: Same source, different formats

```yaml
sources:
  - "uniprot:P01308-fasta@1.0"
  - "uniprot:P01308-xml@1.0"
```

**Resolution**: No conflict - download both formats

### Circular Dependencies

**Scenario**: A → B → A

**Prevention**: Enforced at database level and publish time
- Backend validates no cycles when adding dependencies
- Graph traversal detects cycles during resolution

```rust
fn detect_cycles(graph: &DependencyGraph) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut stack = Vec::new();

    for root in &graph.roots {
        dfs_cycle_detection(root, graph, &mut visited, &mut stack, &mut cycles);
    }

    cycles
}

fn dfs_cycle_detection(
    node: &str,
    graph: &DependencyGraph,
    visited: &mut HashSet<String>,
    stack: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>
) {
    if stack.contains(&node.to_string()) {
        // Cycle detected
        let cycle_start = stack.iter().position(|n| n == node).unwrap();
        cycles.push(stack[cycle_start..].to_vec());
        return;
    }

    if visited.contains(node) {
        return;
    }

    visited.insert(node.to_string());
    stack.push(node.to_string());

    for edge in graph.edges.iter().filter(|e| e.from == node) {
        dfs_cycle_detection(&edge.to, graph, visited, stack, cycles);
    }

    stack.pop();
}
```

## Download Strategy

### Parallel Downloads

Download multiple files concurrently:

```rust
async fn download_all(
    sources: Vec<SourceSpec>,
    cache_path: &Path
) -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(4));  // Max 4 concurrent
    let mut tasks = Vec::new();

    for source in sources {
        let permit = semaphore.clone().acquire_owned().await?;
        let cache_path = cache_path.to_path_buf();

        let task = tokio::spawn(async move {
            let result = download_source(&source, &cache_path).await;
            drop(permit);
            result
        });

        tasks.push(task);
    }

    // Wait for all downloads
    for task in tasks {
        task.await??;
    }

    Ok(())
}
```

### Smart Download Order

Prioritize:
1. **Small files first**: Quick wins, build confidence
2. **Direct dependencies**: User-specified sources
3. **Transitive dependencies**: Aggregate dependencies

```rust
fn optimize_download_order(sources: &[SourceSpec]) -> Vec<SourceSpec> {
    let mut sorted = sources.to_vec();

    sorted.sort_by_key(|s| {
        let size_priority = s.size / 1_000_000;  // MB
        let type_priority = if s.is_direct { 0 } else { 1000 };
        size_priority + type_priority
    });

    sorted
}
```

### Resume Partial Downloads

Track progress for large files:

```rust
async fn download_with_resume(
    url: &str,
    dest: &Path,
    expected_size: u64
) -> Result<()> {
    let partial_path = dest.with_extension("partial");

    let start_byte = if partial_path.exists() {
        tokio::fs::metadata(&partial_path).await?.len()
    } else {
        0
    };

    if start_byte == expected_size {
        // Already fully downloaded
        tokio::fs::rename(&partial_path, dest).await?;
        return Ok(());
    }

    // Resume from start_byte
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("Range", format!("bytes={}-", start_byte))
        .send()
        .await?;

    // Stream to file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&partial_path)
        .await?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
    }

    // Verify size
    let final_size = tokio::fs::metadata(&partial_path).await?.len();
    if final_size != expected_size {
        bail!("Size mismatch: expected {}, got {}", expected_size, final_size);
    }

    // Move to final location
    tokio::fs::rename(&partial_path, dest).await?;

    Ok(())
}
```

## Dependency Updates

### Updating a Dependency

**Scenario**: User wants to update `uniprot:all` from `1.0` to `1.1`

```bash
# Edit bdp.yml manually or via CLI
bdp source update uniprot:all-fasta@1.0 --version 1.1

# Pull new version
bdp pull
```

**Process**:
1. Fetch new dependency tree for `all@1.1`
2. Compare with cached tree for `all@1.0`
3. Compute diff (added, removed, updated proteins)
4. Download only changed dependencies
5. Update lockfile and cache

```rust
async fn incremental_update(
    old_version: &str,
    new_version: &str
) -> Result<UpdatePlan> {
    let old_deps = load_cached_dependencies(old_version)?;
    let new_deps = fetch_dependencies(new_version).await?;

    let old_set: HashSet<_> = old_deps.iter()
        .map(|d| &d.source)
        .collect();
    let new_set: HashSet<_> = new_deps.iter()
        .map(|d| &d.source)
        .collect();

    let added: Vec<_> = new_set.difference(&old_set)
        .map(|s| (*s).clone())
        .collect();

    let removed: Vec<_> = old_set.difference(&new_set)
        .map(|s| (*s).clone())
        .collect();

    let updated: Vec<_> = new_deps.iter()
        .filter(|new_dep| {
            old_deps.iter().any(|old_dep| {
                old_dep.source == new_dep.source &&
                old_dep.checksum != new_dep.checksum
            })
        })
        .map(|d| d.source.clone())
        .collect();

    Ok(UpdatePlan { added, removed, updated })
}
```

## Dependency Visualization

### CLI Command

```bash
$ bdp deps tree uniprot:all-fasta@1.0
```

**Output**:
```
uniprot:all-fasta@1.0 (4.0 GB, 567,239 dependencies)
├─ uniprot:P01308-fasta@1.0 (4 KB)
├─ uniprot:P04637-fasta@1.0 (8 KB)
├─ uniprot:P12345-fasta@1.0 (6 KB)
└─ ... (567,236 more)

Total size: 4.0 GB
Total files: 567,239
```

### Dependency Statistics

```bash
$ bdp deps stats
```

**Output**:
```
Dependency Statistics
=====================

Direct dependencies:     3
Transitive dependencies: 567,239
Total unique sources:    567,240

Size breakdown:
  Direct:      24 KB
  Transitive:  4.0 GB
  Total:       4.0 GB

Largest dependencies:
  1. uniprot:all-fasta@1.0          4.0 GB
  2. ncbi:GRCh38-fasta@2.0          3.2 GB
  3. ensembl:homo-sapiens-fasta@1.0 1.5 GB

Deepest dependency chain: 2 levels
```

## Future Enhancements

### Version Ranges (Post-MVP)

Support semantic versioning ranges:

```yaml
sources:
  - "uniprot:P01308-fasta@^1.0"   # >=1.0.0, <2.0.0
  - "uniprot:P04637-fasta@~1.5"   # >=1.5.0, <1.6.0
```

**Resolution**:
```rust
async fn resolve_version_range(
    org: &str,
    name: &str,
    range: &str
) -> Result<String> {
    let versions = api_client.get_versions(org, name).await?;

    let req = VersionReq::parse(range)?;
    let matching = versions.iter()
        .map(|v| Version::parse(&v.version))
        .filter_map(Result::ok)
        .filter(|v| req.matches(v))
        .max();

    matching
        .map(|v| v.to_string())
        .ok_or_else(|| anyhow!("No matching version found"))
}
```

### Dependency Constraints

Specify optional dependencies:

```yaml
sources:
  - "uniprot:all-fasta@1.0"
    optional_dependencies:
      - "uniprot:reviewed-only"  # Only reviewed proteins
```

### Peer Dependencies

Tool declares required data sources:

```yaml
tools:
  - "ncbi:blast@2.14.0"
    peer_dependencies:
      - "ncbi:nr-database-fasta@latest"
```

## Related Documents

- [Database Schema](./database-schema.md) - Dependencies table structure
- [File Formats](./file-formats.md) - Lockfile format
- [API Design](./api-design.md) - Dependency endpoints
- [Cache Strategy](./cache-strategy.md) - Download and caching
