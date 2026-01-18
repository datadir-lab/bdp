# Cache Strategy

Local caching, team sharing, and file integrity management for BDP.

## Overview

BDP caches downloaded data sources and tools locally to:
1. Avoid redundant downloads
2. Enable offline work
3. Share cache across team members
4. Track storage usage and access patterns
5. Verify file integrity

## Cache Directory Structure

```
.bdp/
├── cache/
│   ├── sources/
│   │   ├── {organization}/
│   │   │   └── {name}@{version}/
│   │   │       ├── {filename}.{format}
│   │   │       └── metadata.json
│   │   └── ...
│   └── tools/
│       ├── {organization}/
│       │   └── {name}@{version}/
│       │       ├── {tool-files}
│       │       └── metadata.json
│       └── ...
├── bdp.db                          # SQLite tracking database
├── resolved-dependencies.json      # Dependency trees
└── audit.log                       # Integrity check log
```

### Example Structure

```
.bdp/
├── cache/
│   ├── sources/
│   │   ├── uniprot/
│   │   │   ├── P01308@1.0/
│   │   │   │   ├── P01308.fasta
│   │   │   │   ├── P01308.xml
│   │   │   │   └── metadata.json
│   │   │   ├── P04637@1.0/
│   │   │   │   ├── P04637.fasta
│   │   │   │   └── metadata.json
│   │   │   └── all@1.0/
│   │   │       └── metadata.json      # Just metadata, no single file
│   │   └── ncbi/
│   │       └── GRCh38@2.0/
│   │           ├── genome.fasta
│   │           └── metadata.json
│   └── tools/
│       ├── ncbi/
│       │   └── blast@2.14.0/
│       │       ├── bin/
│       │       ├── lib/
│       │       └── metadata.json
│       └── broad/
│           └── gatk@4.3.0/
│               ├── gatk.jar
│               └── metadata.json
├── bdp.db
├── resolved-dependencies.json
└── audit.log
```

## Cache Path Format

**Data Sources**:
```
cache/sources/{organization}/{name}@{version}/{filename}.{format}
```

Examples:
- `cache/sources/uniprot/P01308@1.0/P01308.fasta`
- `cache/sources/uniprot/P01308@1.0/P01308.xml`
- `cache/sources/ncbi/GRCh38@2.0/genome.fasta`

**Tools**:
```
cache/tools/{organization}/{name}@{version}/{extracted-files}
```

Examples:
- `cache/tools/ncbi/blast@2.14.0/bin/blastp`
- `cache/tools/broad/gatk@4.3.0/gatk.jar`

## Metadata Files

Each cached version has a `metadata.json` file:

```json
{
  "source": "uniprot:P01308-fasta@1.0",
  "external_version": "2025_01",
  "organization": "uniprot",
  "name": "P01308",
  "version": "1.0",
  "cached_at": "2024-01-16T12:30:45Z",
  "files": [
    {
      "format": "fasta",
      "filename": "P01308.fasta",
      "checksum": "sha256-1a2b3c4d5e6f7890abcdef1234567890abcdef1234567890abcdef1234567890",
      "size": 4096,
      "downloaded_at": "2024-01-16T12:30:50Z"
    },
    {
      "format": "xml",
      "filename": "P01308.xml",
      "checksum": "sha256-9876543210fedcba0987654321fedcba0987654321fedcba0987654321fedcba",
      "size": 16384,
      "downloaded_at": "2024-01-16T12:31:05Z"
    }
  ],
  "description": "Insulin [Homo sapiens]",
  "tags": ["protein", "hormone", "signaling"],
  "last_accessed": "2024-01-16T15:22:10Z",
  "access_count": 5
}
```

## SQLite Database (bdp.db)

Local tracking database using **WAL mode** for concurrency.

### Schema

```sql
-- Enable WAL mode for concurrent access
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;

-- Cache entries
CREATE TABLE cache_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_spec TEXT NOT NULL UNIQUE,     -- 'uniprot:P01308-fasta@1.0'
    cache_path TEXT NOT NULL,              -- 'sources/uniprot/P01308@1.0/P01308.fasta'
    checksum TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    cached_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_verified TIMESTAMP,
    last_accessed TIMESTAMP,
    access_count INTEGER DEFAULT 0
);

-- Download history
CREATE TABLE download_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_spec TEXT NOT NULL,
    downloaded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    size_bytes INTEGER,
    duration_ms INTEGER,
    download_url TEXT,
    success BOOLEAN DEFAULT TRUE,
    error_message TEXT
);

-- Audit log
CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cache_path TEXT NOT NULL,
    action TEXT NOT NULL,                  -- 'verified', 'modified', 'corrupted', 'missing'
    expected_checksum TEXT,
    actual_checksum TEXT,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    details TEXT
);

-- File locks (for team cache)
CREATE TABLE file_locks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    resource_path TEXT UNIQUE NOT NULL,    -- 'sources/uniprot/P01308@1.0/P01308.fasta'
    locked_by TEXT NOT NULL,               -- '{hostname}:{pid}'
    locked_at INTEGER NOT NULL,            -- Unix timestamp
    lock_ttl INTEGER DEFAULT 300,          -- Seconds (5 minutes)
    operation TEXT                         -- 'download', 'verify', 'cleanup'
);

-- Storage statistics (aggregated)
CREATE TABLE storage_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    recorded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    total_files INTEGER,
    total_size_bytes INTEGER,
    sources_count INTEGER,
    tools_count INTEGER,
    oldest_access TIMESTAMP,
    newest_access TIMESTAMP
);

-- Indexes
CREATE INDEX idx_cache_entries_spec ON cache_entries(source_spec);
CREATE INDEX idx_cache_entries_accessed ON cache_entries(last_accessed);
CREATE INDEX idx_download_history_spec ON download_history(source_spec);
CREATE INDEX idx_download_history_time ON download_history(downloaded_at);
CREATE INDEX idx_audit_log_path ON audit_log(cache_path);
CREATE INDEX idx_audit_log_time ON audit_log(timestamp);
CREATE INDEX idx_file_locks_resource ON file_locks(resource_path);
```

## Cache Configuration

### User-Level Config (~/.bdp/config.toml)

```toml
[cache]
path = ".bdp/cache"           # Default: relative to project
max_size_gb = 100             # Optional: max cache size
auto_cleanup = true           # Auto-remove old files

[registry]
url = "https://api.bdp.dev/v1"

[download]
parallel_downloads = 4        # Concurrent downloads
retry_attempts = 3
timeout_seconds = 300
```

### CLI Commands

```bash
# Get current cache path
bdp config cache get
# Output: /home/user/project/.bdp/cache

# Set team-shared cache
bdp config cache set "/mnt/shared/team-cache"

# Set cache back to default (project-local)
bdp config cache reset

# Show cache statistics
bdp stats
# Output:
# Cache Location: /mnt/shared/team-cache
# Total Files: 1,234
# Total Size: 45.2 GB
# Data Sources: 1,150
# Tools: 84
# Oldest Access: 2023-12-15 (62 days ago)
```

## Team Cache Sharing

### Setup

**1. Team Leader Creates Shared Cache**:
```bash
# Create shared directory
mkdir -p /mnt/shared/team-cache

# Initialize BDP in shared location
cd /mnt/shared/team-cache
bdp init --cache-only

# Set permissions (team-readable/writable)
chmod 775 -R /mnt/shared/team-cache
chgrp research-team -R /mnt/shared/team-cache
```

**2. Team Members Configure**:
```bash
# In their project directory
cd ~/my-research-project

# Point to shared cache
bdp config cache set "/mnt/shared/team-cache"

# Now all downloads go to shared cache
bdp pull
```

### Concurrency Control

**File Locking Mechanism**:

```rust
async fn download_with_lock(
    source_spec: &str,
    cache_path: &Path,
    download_url: &str
) -> Result<()> {
    let db = open_cache_db().await?;
    let lock_id = acquire_lock(&db, cache_path).await?;

    // Check if already downloaded by another process
    if cache_path.exists() && verify_checksum(cache_path, expected_checksum).await? {
        release_lock(&db, lock_id).await?;
        return Ok(());
    }

    // Download file
    let result = download_file(download_url, cache_path).await;

    // Release lock
    release_lock(&db, lock_id).await?;

    result
}

async fn acquire_lock(db: &Connection, resource: &Path) -> Result<i64> {
    let hostname = hostname::get()?.to_string_lossy().to_string();
    let pid = std::process::id();
    let lock_by = format!("{}:{}", hostname, pid);

    // Try to acquire lock
    let result = db.execute(
        "INSERT INTO file_locks (resource_path, locked_by, locked_at, operation)
         VALUES (?1, ?2, unixepoch(), 'download')
         ON CONFLICT(resource_path) DO UPDATE SET
            locked_by = excluded.locked_by,
            locked_at = excluded.locked_at
         WHERE locked_at < unixepoch() - lock_ttl",  // Only if lock expired
        params![resource.to_str().unwrap(), lock_by]
    )?;

    if result == 0 {
        // Lock held by another process
        bail!("Resource locked by another process");
    }

    // Return lock ID
    let lock_id = db.last_insert_rowid();
    Ok(lock_id)
}

async fn release_lock(db: &Connection, lock_id: i64) -> Result<()> {
    db.execute(
        "DELETE FROM file_locks WHERE id = ?1",
        params![lock_id]
    )?;
    Ok(())
}
```

**Lock Cleanup** (background task):
```rust
async fn cleanup_expired_locks(db: &Connection) -> Result<()> {
    let deleted = db.execute(
        "DELETE FROM file_locks
         WHERE locked_at < unixepoch() - lock_ttl",
        []
    )?;

    if deleted > 0 {
        tracing::info!("Cleaned up {} expired locks", deleted);
    }

    Ok(())
}
```

### Conflict Resolution

**Scenario**: Two researchers pull same file simultaneously

```
Time    Process A                Process B
----    ---------                ---------
T0      acquire_lock(P01308)
T1      start download           acquire_lock(P01308) → WAIT
T2      download complete
T3      verify checksum
T4      release_lock             acquire_lock → SUCCESS
T5                               file exists, checksum valid
T6                               skip download, release_lock
```

## Cache Invalidation

### Manual Cleanup

```bash
# Remove specific source
bdp cache remove uniprot:P01308-fasta@1.0

# Remove all sources for a version
bdp cache remove uniprot:P01308@1.0

# Remove unused sources (not in bdp.yml or bdl.lock)
bdp cache clean

# Remove sources not accessed in 30 days
bdp cache clean --older-than 30d

# Force remove everything
bdp cache clear --force
```

### Automatic Cleanup

```rust
async fn auto_cleanup(config: &Config, db: &Connection) -> Result<()> {
    if !config.cache.auto_cleanup {
        return Ok(());
    }

    // Get current cache size
    let total_size = get_cache_size(&config.cache.path).await?;
    let max_size = config.cache.max_size_gb * 1024 * 1024 * 1024;

    if total_size < max_size {
        return Ok(());
    }

    // Remove least recently accessed files until under limit
    let entries = db.query(
        "SELECT cache_path, size_bytes FROM cache_entries
         ORDER BY last_accessed ASC",
        []
    )?;

    let mut freed = 0;
    for entry in entries {
        remove_file(&entry.cache_path).await?;
        freed += entry.size_bytes;

        if total_size - freed < max_size {
            break;
        }
    }

    tracing::info!("Auto-cleanup freed {} bytes", freed);
    Ok(())
}
```

## Integrity Verification

### Audit Command

```bash
$ bdp audit
```

**Output**:
```
Auditing cache integrity...

✓ uniprot:P01308-fasta@1.0
  - File: .bdp/cache/sources/uniprot/P01308@1.0/P01308.fasta
  - Expected: sha256-1a2b3c4d...
  - Actual:   sha256-1a2b3c4d...
  - Status: OK

✓ uniprot:P01308-xml@1.0
  - File: .bdp/cache/sources/uniprot/P01308@1.0/P01308.xml
  - Expected: sha256-9876543...
  - Actual:   sha256-9876543...
  - Status: OK

✗ uniprot:P04637-fasta@1.0
  - File: .bdp/cache/sources/uniprot/P04637@1.0/P04637.fasta
  - Expected: sha256-abcd1234...
  - Actual:   sha256-xxxx9999...
  - Status: CORRUPTED

⚠ ncbi:GRCh38-fasta@2.0
  - File: .bdp/cache/sources/ncbi/GRCh38@2.0/genome.fasta
  - Status: MISSING

Summary:
  Verified: 2
  Corrupted: 1
  Missing: 1

Run 'bdp pull' to re-download corrupted/missing files.
```

### Implementation

```rust
async fn audit_cache(lockfile: &Lockfile, cache_path: &Path) -> Result<AuditReport> {
    let mut report = AuditReport::default();

    for (source_spec, entry) in &lockfile.sources {
        let file_path = resolve_cache_path(cache_path, source_spec)?;

        if !file_path.exists() {
            report.missing.push(source_spec.clone());
            log_audit(&db, &file_path, "missing", &entry.checksum, None).await?;
            continue;
        }

        let actual_checksum = compute_file_checksum(&file_path).await?;

        if actual_checksum == entry.checksum {
            report.verified.push(source_spec.clone());
            log_audit(&db, &file_path, "verified", &entry.checksum, Some(&actual_checksum)).await?;
        } else {
            report.corrupted.push(source_spec.clone());
            log_audit(&db, &file_path, "corrupted", &entry.checksum, Some(&actual_checksum)).await?;
        }
    }

    Ok(report)
}

async fn compute_file_checksum(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 8192];

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("sha256-{}", hex::encode(hasher.finalize())))
}
```

## Cache Statistics

### Storage Usage

```bash
$ bdp stats
```

**Output**:
```
BDP Cache Statistics
====================

Location: /mnt/shared/team-cache

Storage:
  Total Size:        45.2 GB
  Total Files:       1,234
  Data Sources:      1,150 (43.8 GB)
  Tools:             84 (1.4 GB)

Activity:
  Total Downloads:   1,567
  Last Download:     2024-01-16 15:30:45
  Last Access:       2024-01-16 15:45:12

Top Data Sources by Size:
  1. ncbi:GRCh38-fasta@2.0          3.2 GB
  2. uniprot:all-fasta@1.0          4.1 GB
  3. ensembl:homo-sapiens-fasta@1.0 2.8 GB

Oldest Files (by access):
  1. uniprot:P99999-fasta@1.0       62 days ago
  2. ncbi:legacy-genome-fasta@1.0   45 days ago

Recommendations:
  - Consider cleaning files older than 30 days: bdp cache clean --older-than 30d
  - Current usage: 45% of max (100 GB)
```

### Implementation

```rust
async fn compute_statistics(db: &Connection, cache_path: &Path) -> Result<CacheStats> {
    let stats = db.query_row(
        "SELECT
            COUNT(*) as total_files,
            SUM(size_bytes) as total_size,
            MIN(last_accessed) as oldest_access,
            MAX(last_accessed) as newest_access
         FROM cache_entries",
        [],
        |row| Ok(CacheStats {
            total_files: row.get(0)?,
            total_size: row.get(1)?,
            oldest_access: row.get(2)?,
            newest_access: row.get(3)?,
        })
    )?;

    // Get breakdown by type
    let sources_count = db.query_row(
        "SELECT COUNT(*), SUM(size_bytes)
         FROM cache_entries
         WHERE source_spec LIKE '%-fasta@%'",  // Data sources have format suffix
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    )?;

    // ... more queries

    Ok(stats)
}
```

## Performance Optimization

### Parallel Downloads

```rust
async fn download_multiple(sources: Vec<SourceSpec>) -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(4));  // Max 4 concurrent
    let mut tasks = Vec::new();

    for source in sources {
        let permit = semaphore.clone().acquire_owned().await?;
        let task = tokio::spawn(async move {
            let result = download_source(&source).await;
            drop(permit);  // Release semaphore
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

### Progressive Verification

Verify checksums during download (streaming):

```rust
async fn download_and_verify(
    url: &str,
    dest: &Path,
    expected_checksum: &str
) -> Result<()> {
    let response = reqwest::get(url).await?;
    let mut file = File::create(dest).await?;
    let mut hasher = Sha256::new();

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }

    let actual = format!("sha256-{}", hex::encode(hasher.finalize()));
    if actual != expected_checksum {
        tokio::fs::remove_file(dest).await?;
        bail!("Checksum mismatch: expected {}, got {}", expected_checksum, actual);
    }

    Ok(())
}
```

## Migration Scenarios

### Moving Cache Location

```bash
# Old cache: .bdp/cache
# New cache: /mnt/shared/team-cache

# Option 1: Move files
mv .bdp/cache/* /mnt/shared/team-cache/
bdp config cache set "/mnt/shared/team-cache"

# Option 2: Re-download
bdp config cache set "/mnt/shared/team-cache"
bdp pull  # Re-downloads to new location
bdp cache clear --path .bdp/cache  # Remove old cache
```

### Team Member Joins

```bash
# New team member clones repo
git clone https://github.com/lab/cancer-research.git
cd cancer-research

# Configure shared cache
bdp config cache set "/mnt/shared/team-cache"

# Pull sources (uses shared cache, no download needed if files exist)
bdp pull

# bdp.yml and bdl.lock already in repo from git
# Shared cache already has files from team
# Just updates local bdp.db tracking
```

## Security Considerations

1. **Checksum Verification**: Always verify SHA-256 before use
2. **HTTPS Only**: Download URLs must be HTTPS
3. **File Permissions**: Shared cache should have appropriate group permissions
4. **Lock Timeout**: Prevent indefinite locks (5min TTL)
5. **Path Traversal**: Validate all paths to prevent directory traversal attacks

```rust
fn validate_cache_path(path: &Path, base: &Path) -> Result<()> {
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(base) {
        bail!("Invalid cache path: attempted directory traversal");
    }
    Ok(())
}
```

## Related Documents

- [File Formats](./file-formats.md) - bdp.yml and bdl.lock structure
- [Database Schema](./database-schema.md) - Server-side storage
- [API Design](./api-design.md) - Download endpoints
- [Dependency Resolution](./dependency-resolution.md) - Resolving dependencies
