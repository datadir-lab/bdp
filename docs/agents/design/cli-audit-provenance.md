# CLI Audit & Provenance System Design

Comprehensive design for BDP's local audit trail, provenance tracking, and regulatory compliance reporting.

## Overview

The audit system provides:
1. **Local audit trail** - SQLite database tracking all CLI operations
2. **Tamper detection** - Hash-chain based integrity verification
3. **Regulatory export** - FDA, NIH, EMA compliance reports
4. **Post-pull tracking** - Automatic tool execution and output verification
5. **Report generation** - Data Availability Statements, Methods sections

## Design Principles

1. **Local-First**: Audit data stored in `.bdp/bdp.db` (local SQLite)
2. **Editable**: Users can modify/delete audit logs as needed (clearly documented)
3. **Report-Focused**: Primary purpose is generating research reports, not legal evidence
4. **Simple**: No cryptographic signing, no complex key management
5. **Extensible**: Middleware architecture allows future backend integration
6. **Team-Friendly**: Works across cloned repos without complex setup

## Audit Database Schema

### SQLite Schema (`.bdp/bdp.db`)

```sql
-- Main audit events table (append-preferred, but editable)
CREATE TABLE audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    event_type TEXT NOT NULL,  -- 'download', 'verify', 'post_pull', 'pull_start', 'pull_complete'

    -- What happened
    source_spec TEXT,  -- 'uniprot:P01308-fasta@1.0' or wildcard 'uniprot:*-fasta@1.0'
    details TEXT,  -- JSON: {url, sha256, size_bytes, exit_code, tool, output_files}

    -- Machine context (no authentication)
    machine_id TEXT NOT NULL,  -- Stable machine identifier (hostname or UUID)

    -- Tamper detection (optional - for integrity checking)
    event_hash TEXT,  -- SHA-256 of this event (computed on export)
    previous_hash TEXT,  -- Hash of previous event (for chain verification)

    -- User annotations
    notes TEXT,  -- User can add notes/justifications
    archived BOOLEAN DEFAULT 0  -- Mark for archival
);

-- Files tracked in cache (current state)
CREATE TABLE files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_spec TEXT NOT NULL UNIQUE,  -- 'uniprot:P01308-fasta@1.0'
    file_path TEXT NOT NULL,  -- Relative: '.bdp/cache/sources/uniprot/P01308@1.0/P01308.fasta'
    sha256 TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,

    downloaded_at DATETIME,
    download_event_id INTEGER,  -- References audit_events(id)

    last_verified_at DATETIME,
    verification_status TEXT,  -- 'ok', 'corrupted', 'missing'

    FOREIGN KEY(download_event_id) REFERENCES audit_events(id) ON DELETE SET NULL
);

-- Post-pull generated files (indexes, databases, etc.)
CREATE TABLE generated_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_file_id INTEGER NOT NULL,  -- References files(id)
    file_path TEXT NOT NULL,  -- '.bdp/cache/sources/uniprot/P01308@1.0/P01308.fasta.fai'
    tool TEXT NOT NULL,  -- 'samtools', 'blast', 'custom-hook'
    sha256 TEXT,  -- Checksum (if verifiable)
    size_bytes INTEGER,

    generated_at DATETIME,
    generation_event_id INTEGER,  -- References audit_events(id)

    FOREIGN KEY(source_file_id) REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY(generation_event_id) REFERENCES audit_events(id) ON DELETE SET NULL
);

-- Export snapshots (for archival)
CREATE TABLE audit_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id TEXT NOT NULL UNIQUE,  -- UUID
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    export_format TEXT NOT NULL,  -- 'fda', 'nih', 'ema', 'das'

    event_id_start INTEGER,  -- First event in snapshot
    event_id_end INTEGER,  -- Last event in snapshot
    event_count INTEGER NOT NULL,

    chain_verified BOOLEAN,  -- Was hash chain intact at export?
    output_path TEXT,  -- Where exported file was saved

    FOREIGN KEY(event_id_start) REFERENCES audit_events(id),
    FOREIGN KEY(event_id_end) REFERENCES audit_events(id)
);

-- Indexes for performance
CREATE INDEX idx_events_timestamp ON audit_events(timestamp);
CREATE INDEX idx_events_source ON audit_events(source_spec);
CREATE INDEX idx_events_type ON audit_events(event_type);
CREATE INDEX idx_files_source ON files(source_spec);
CREATE INDEX idx_generated_source ON generated_files(source_file_id);
```

## CQRS Audit Middleware Pattern

### Architecture

All CLI commands flow through an audit middleware layer using dependency injection:

```rust
// Trait for audit logging (dependency injection)
#[async_trait]
pub trait AuditLogger: Send + Sync {
    async fn log_event(&self, event: AuditEvent) -> Result<i64>;
    async fn verify_integrity(&self) -> Result<bool>;
    async fn export(&self, format: ExportFormat) -> Result<String>;
}

// Local SQLite implementation (MVP)
pub struct LocalAuditLogger {
    db: Arc<Mutex<Connection>>,
    machine_id: String,
}

// Future: Backend API implementation (post-MVP)
pub struct BackendAuditLogger {
    client: ApiClient,
    local_cache: LocalAuditLogger,  // Fallback
}

// Command context with audit logging
pub struct CommandContext {
    audit: Arc<dyn AuditLogger>,
    cache: CacheManager,
    config: Config,
}

// Middleware wrapper for commands
pub async fn execute_with_audit<F, T>(
    ctx: &CommandContext,
    event_type: &str,
    source_spec: Option<String>,
    command: F,
) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    // Log start event
    let start_event = AuditEvent {
        event_type: format!("{}_start", event_type),
        source_spec: source_spec.clone(),
        details: json!({"started_at": Utc::now()}),
        machine_id: ctx.audit.machine_id(),
        ..Default::default()
    };
    ctx.audit.log_event(start_event).await?;

    // Execute command
    let result = command();

    // Log completion event
    let status = if result.is_ok() { "success" } else { "failure" };
    let complete_event = AuditEvent {
        event_type: format!("{}_{}", event_type, status),
        source_spec,
        details: json!({
            "status": status,
            "error": result.as_ref().err().map(|e| e.to_string())
        }),
        machine_id: ctx.audit.machine_id(),
        ..Default::default()
    };
    ctx.audit.log_event(complete_event).await?;

    result
}
```

### Example Command Implementation

```rust
// bdp pull command with audit middleware
pub async fn cmd_pull(ctx: &CommandContext, sources: Vec<String>) -> Result<()> {
    execute_with_audit(ctx, "pull", None, || {
        for source in &sources {
            // Download with audit
            execute_with_audit(ctx, "download", Some(source.clone()), || {
                download_source(ctx, source)
            }).await?;

            // Verify with audit
            execute_with_audit(ctx, "verify", Some(source.clone()), || {
                verify_checksum(ctx, source)
            }).await?;

            // Post-pull hooks with audit
            if let Some(hooks) = get_post_pull_hooks(source) {
                for hook in hooks {
                    execute_with_audit(ctx, "post_pull", Some(source.clone()), || {
                        run_post_pull_hook(ctx, source, &hook)
                    }).await?;
                }
            }
        }
        Ok(())
    }).await
}
```

## Post-Pull Hooks System

### Pre-Defined Tools (MVP)

Built-in tools recognized by CLI:

```rust
pub fn get_builtin_tool(tool: &str, format: &str, file_path: &Path) -> Option<PostPullTool> {
    match (tool, format) {
        ("samtools", "fasta") => Some(PostPullTool {
            name: "samtools".into(),
            command: "samtools".into(),
            args: vec!["faidx".into(), file_path.to_string_lossy().into()],
            expected_outputs: vec![file_path.with_extension("fasta.fai")],
        }),

        ("blast", "fasta") => Some(PostPullTool {
            name: "blast".into(),
            command: "makeblastdb".into(),
            args: vec![
                "-in".into(),
                file_path.to_string_lossy().into(),
                "-dbtype".into(),
                "prot".into(),
            ],
            expected_outputs: vec![
                file_path.with_extension("phr"),
                file_path.with_extension("pin"),
                file_path.with_extension("psq"),
            ],
        }),

        ("bwa", "fasta") => Some(PostPullTool {
            name: "bwa".into(),
            command: "bwa".into(),
            args: vec!["index".into(), file_path.to_string_lossy().into()],
            expected_outputs: vec![
                file_path.with_extension("amb"),
                file_path.with_extension("ann"),
                file_path.with_extension("bwt"),
                file_path.with_extension("pac"),
                file_path.with_extension("sa"),
            ],
        }),

        _ => None,
    }
}
```

### Wildcard Pattern Support (MVP)

```yaml
# bdp.yml
post_pull:
  uniprot:*-fasta@1.0:  # Matches any UniProt protein FASTA at version 1.0
    - "samtools"

  uniprot:all-*@*:  # Matches all formats of uniprot:all at any version
    - "samtools"
    - "blast"

  ncbi:*@2.0:  # Matches any NCBI source at version 2.0
    - "bwa"
```

**Pattern matching logic**:
```rust
pub fn matches_pattern(source: &str, pattern: &str) -> bool {
    let source_parts: Vec<&str> = source.split(&[':', '-', '@'][..]).collect();
    let pattern_parts: Vec<&str> = pattern.split(&[':', '-', '@'][..]).collect();

    if source_parts.len() != pattern_parts.len() {
        return false;
    }

    source_parts.iter().zip(pattern_parts.iter())
        .all(|(s, p)| p == &"*" || s == p)
}

pub fn get_post_pull_hooks(source: &str, manifest: &Manifest) -> Vec<String> {
    let mut hooks = Vec::new();

    for (pattern, tools) in &manifest.post_pull {
        if matches_pattern(source, pattern) {
            hooks.extend(tools.clone());
        }
    }

    hooks
}
```

### Custom Hooks (Post-MVP)

Users can define custom scripts in `.bdp/hooks/` (committed to git):

```bash
# .bdp/hooks/custom-protein-analysis.sh
#!/bin/bash
INPUT=$1
OUTPUT_DIR=$(dirname "$INPUT")

# Custom analysis
python /path/to/analysis.py "$INPUT" > "$OUTPUT_DIR/analysis.txt"
```

**In bdp.yml**:
```yaml
post_pull:
  uniprot:*-fasta@1.0:
    - "samtools"
    - "custom-protein-analysis"  # Looks for .bdp/hooks/custom-protein-analysis.sh
```

**Discovery logic**:
```rust
pub fn find_hook(name: &str) -> Option<PathBuf> {
    // Check .bdp/hooks/ directory
    let hook_path = Path::new(".bdp/hooks").join(name);

    // Try with common extensions
    for ext in &["", ".sh", ".py", ".R"] {
        let path = hook_path.with_extension(ext);
        if path.exists() && path.is_file() {
            return Some(path);
        }
    }

    None
}
```

## Verification System

### bdp verify Command

Verifies integrity of:
1. **Source files** - checksums match `bdl.lock`
2. **Generated files** - post-pull outputs exist and are valid

```rust
pub async fn cmd_verify(ctx: &CommandContext) -> Result<()> {
    execute_with_audit(ctx, "verify_all", None, || {
        let lockfile = Lockfile::load("bdl.lock")?;
        let mut all_ok = true;

        println!("Verifying source files...");
        for (spec, entry) in &lockfile.sources {
            let result = verify_source_file(ctx, spec, entry).await?;
            print_verification_result(spec, &result);

            if !result.is_ok() {
                all_ok = false;
            }

            // Verify generated files
            let generated = get_generated_files(ctx, spec).await?;
            for gen_file in generated {
                let gen_result = verify_generated_file(ctx, &gen_file).await?;
                print_verification_result(&gen_file.path, &gen_result);

                if !gen_result.is_ok() {
                    all_ok = false;
                }
            }
        }

        if all_ok {
            println!("\n✓ All files verified successfully");
            Ok(())
        } else {
            bail!("✗ Verification failed for some files");
        }
    }).await
}

async fn verify_source_file(
    ctx: &CommandContext,
    spec: &str,
    entry: &LockfileEntry,
) -> Result<VerificationResult> {
    // Get cached file path
    let file_path = ctx.cache.get_file_path(spec)?;

    if !file_path.exists() {
        return Ok(VerificationResult::Missing);
    }

    // Compute checksum
    let mut file = File::open(&file_path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let checksum = format!("sha256-{}", hex::encode(hasher.finalize()));

    // Compare with lockfile
    if checksum == entry.checksum {
        // Update database
        ctx.audit.log_event(AuditEvent {
            event_type: "verify_ok".into(),
            source_spec: Some(spec.into()),
            details: json!({"checksum": checksum}),
            ..Default::default()
        }).await?;

        Ok(VerificationResult::Ok)
    } else {
        // Log corruption
        ctx.audit.log_event(AuditEvent {
            event_type: "verify_corrupted".into(),
            source_spec: Some(spec.into()),
            details: json!({
                "expected": entry.checksum,
                "actual": checksum
            }),
            ..Default::default()
        }).await?;

        Ok(VerificationResult::Corrupted {
            expected: entry.checksum.clone(),
            actual: checksum,
        })
    }
}

async fn verify_generated_file(
    ctx: &CommandContext,
    gen_file: &GeneratedFile,
) -> Result<VerificationResult> {
    let file_path = Path::new(&gen_file.file_path);

    if !file_path.exists() {
        return Ok(VerificationResult::Missing);
    }

    // For some tools, we can verify checksums
    // For others (like samtools .fai), we just check existence
    match gen_file.tool.as_str() {
        "samtools" => {
            // .fai files are deterministic, can checksum
            // But for MVP, just verify existence
            Ok(VerificationResult::Ok)
        },
        "blast" => {
            // BLAST databases are deterministic
            Ok(VerificationResult::Ok)
        },
        _ => {
            // Custom hooks - just check existence
            Ok(VerificationResult::Ok)
        }
    }
}
```

## Export Formats

### FDA 21 CFR Part 11 Export

```bash
bdp audit export --format fda --output audit-fda-2026-01-18.json
```

**Output format**:
```json
{
  "audit_report": {
    "standard": "FDA 21 CFR Part 11",
    "generated_at": "2026-01-18T14:00:00Z",
    "project": {
      "name": "my-proteomics-project",
      "version": "1.0.0"
    },
    "machine": {
      "machine_id": "lab-workstation-42",
      "hostname": "ws-42.lab.university.edu"
    },
    "period": {
      "start": "2026-01-01T00:00:00Z",
      "end": "2026-01-18T14:00:00Z"
    },
    "event_count": 1523,
    "events": [
      {
        "id": 1,
        "timestamp": "2026-01-15T10:30:00Z",
        "event_type": "download",
        "source": "uniprot:P01308-fasta@1.0",
        "details": {
          "url": "https://api.bdp.dev/files/uniprot/P01308/1.0/P01308.fasta",
          "sha256": "sha256-abc123...",
          "size_bytes": 4096
        },
        "machine_id": "lab-workstation-42"
      },
      {
        "id": 2,
        "timestamp": "2026-01-15T10:30:23Z",
        "event_type": "verify_ok",
        "source": "uniprot:P01308-fasta@1.0",
        "details": {
          "checksum": "sha256-abc123..."
        },
        "machine_id": "lab-workstation-42"
      },
      {
        "id": 3,
        "timestamp": "2026-01-15T10:31:00Z",
        "event_type": "post_pull",
        "source": "uniprot:P01308-fasta@1.0",
        "details": {
          "tool": "samtools",
          "command": "samtools faidx P01308.fasta",
          "exit_code": 0,
          "output_files": ["P01308.fasta.fai"]
        },
        "machine_id": "lab-workstation-42"
      }
    ],
    "verification": {
      "chain_verified": true,
      "no_gaps_in_sequence": true,
      "all_timestamps_valid": true
    },
    "disclaimer": "This audit trail was generated from local records and is editable. It is intended for research documentation purposes, not legal evidence."
  }
}
```

### NIH DMS Export (Data Availability Statement)

```bash
bdp audit export --format das --output data-availability.md
```

**Output format** (Markdown):
```markdown
## Data Availability Statement

All data used in this study are publicly available and were acquired using the BDP (Bioinformatics Data Package Manager) CLI tool version 0.1.0.

### Protein Sequences

**UniProt Swiss-Prot Insulin (P01308)**
- Source: UniProt Knowledgebase
- Version: Release 2026_01 (accessed 2026-01-15)
- Format: FASTA
- File: P01308.fasta (4,096 bytes)
- Checksum: sha256-abc123...
- BDP Specification: `uniprot:P01308-fasta@1.0`

**UniProt Swiss-Prot TP53 (P04637)**
- Source: UniProt Knowledgebase
- Version: Release 2026_01 (accessed 2026-01-15)
- Format: FASTA
- File: P04637.fasta (8,192 bytes)
- Checksum: sha256-def456...
- BDP Specification: `uniprot:P04637-fasta@1.0`

### Reference Genome

**Human Reference Genome GRCh38**
- Source: NCBI Genome Reference Consortium
- Version: GRCh38.p14 (accessed 2026-01-10)
- Format: FASTA
- File: GRCh38.fasta (3.2 GB)
- Checksum: sha256-genome123...
- BDP Specification: `ncbi:GRCh38-fasta@2.0`

### Post-Processing

All FASTA files were indexed using samtools v1.18 for random access:
```bash
samtools faidx P01308.fasta
samtools faidx P04637.fasta
samtools faidx GRCh38.fasta
```

### Reproducibility

The complete data environment can be reproduced using:
```bash
git clone https://github.com/user/my-proteomics-project.git
cd my-proteomics-project
bdp pull
```

All data sources, versions, and checksums are specified in the project manifest (`bdp.yml`) and lockfile (`bdl.lock`), both committed to version control.

### Software

- BDP CLI version 0.1.0 (https://github.com/bdp/bdp)
- samtools version 1.18 (http://www.htslib.org/)

Generated automatically by `bdp audit export --format das` on 2026-01-18.
```

### EMA ALCOA++ Export

```bash
bdp audit export --format ema --output audit-ema-2026-01-18.yaml
```

**Output format** (YAML):
```yaml
alcoa_plus_compliance_report:
  generated_at: "2026-01-18T14:00:00Z"
  project: "my-proteomics-project"

  # Attributable
  attributable:
    status: "compliant"
    evidence: "All audit events include machine_id and timestamp"
    machine_id: "lab-workstation-42"
    hostname: "ws-42.lab.university.edu"

  # Legible
  legible:
    status: "compliant"
    evidence: "Human-readable JSON/YAML export, machine-processable SQLite database"
    formats: ["JSON", "YAML", "Markdown", "SQLite"]

  # Contemporaneous
  contemporaneous:
    status: "compliant"
    evidence: "Events timestamped at occurrence (ISO 8601 format)"
    timestamp_format: "ISO 8601"

  # Original
  original:
    status: "compliant"
    evidence: "Source URLs recorded, checksums verify original data"
    verification: "SHA-256 checksums in lockfile"

  # Accurate
  accurate:
    status: "compliant"
    evidence: "Cryptographic checksums, automated integrity verification"
    checksum_algorithm: "SHA-256"

  # Complete
  complete:
    status: "compliant"
    evidence: "All data operations logged (download, verify, post-pull)"
    total_events: 1523

  # Consistent
  consistent:
    status: "compliant"
    evidence: "Chronological event ordering enforced by database"
    ordering: "Ascending by ID and timestamp"

  # Enduring
  enduring:
    status: "compliant"
    evidence: "SQLite database for long-term storage, archival exports"
    storage: "SQLite + JSON/YAML archives"

  # Available
  available:
    status: "compliant"
    evidence: "Multiple export formats (JSON, YAML, Markdown)"
    export_commands:
      - "bdp audit export --format fda"
      - "bdp audit export --format das"
      - "bdp audit export --format ema"

  # Traceable
  traceable:
    status: "compliant"
    evidence: "Full provenance from source to derived files, post-pull tracking"
    provenance: "Source files → Post-pull outputs (samtools, BLAST, etc.)"

  disclaimer: |
    This audit trail is stored locally in SQLite and is editable by the user.
    It is intended for research documentation and regulatory reporting,
    not for legal evidence or forensic purposes.
```

## Archive System (Post-MVP)

### bdp audit archive

Archive old events to reduce database size:

```bash
# Archive events older than 6 months
bdp audit archive --before "2025-07-01"

# Archive to specific file
bdp audit archive --before "2025-07-01" --output archive-2025-h1.json
```

**What happens**:
1. Exports events to JSON file
2. Marks events as `archived: true` in database
3. Optionally removes from active table (move to `audit_events_archived` table)
4. Verification still works (checks both active + archived)

**Schema addition**:
```sql
CREATE TABLE audit_events_archived (
    -- Same schema as audit_events
    -- ...
    archive_date DATETIME NOT NULL,
    archive_file TEXT NOT NULL  -- Path to JSON archive
);
```

## Machine ID Generation

Stable machine identifier without personal info:

```rust
pub fn get_machine_id() -> Result<String> {
    // Try to read existing ID
    let id_file = Path::new(".bdp/machine-id");
    if id_file.exists() {
        return Ok(std::fs::read_to_string(id_file)?.trim().to_string());
    }

    // Generate new ID based on machine info
    let hostname = hostname::get()?
        .to_string_lossy()
        .to_string();

    // Use hostname + random suffix (not MAC address for privacy)
    let suffix = Uuid::new_v4().to_string()[..8].to_string();
    let machine_id = format!("{}-{}", hostname, suffix);

    // Save for future use
    std::fs::write(id_file, &machine_id)?;

    Ok(machine_id)
}
```

## Configuration

### .bdp/config.yml (Gitignored)

```yaml
# Cache location (machine-specific)
cache:
  location: "/mnt/lab-storage/bdp-cache"  # or "local" for default ~/.bdp/cache

# Audit settings
audit:
  enabled: true  # Can disable for testing
  auto_archive_days: 180  # Auto-archive events older than 6 months (0 = never)
```

### bdp.yml (Committed)

```yaml
project:
  name: "my-proteomics-project"
  version: "1.0.0"
  description: "TP53 pathway analysis"

sources:
  - "uniprot:P01308-fasta@1.0"
  - "uniprot:P04637-fasta@1.0"
  - "ncbi:GRCh38-fasta@2.0"

# Post-pull hooks (committed)
post_pull:
  # Wildcard patterns
  uniprot:*-fasta@1.0:
    - "samtools"

  ncbi:*-fasta@*:
    - "samtools"
    - "bwa"

  # Specific overrides
  uniprot:all-fasta@1.0:
    - "samtools"
    - "blast"
```

## Help Text Examples

### bdp audit --help

```
Audit trail management and export

IMPORTANT: The local audit trail (.bdp/bdp.db) is EDITABLE and intended
for research documentation and report generation, NOT legal evidence.

For regulatory compliance reporting (FDA, NIH, EMA), use the export commands
to generate standardized reports from your audit trail.

USAGE:
    bdp audit <SUBCOMMAND>

SUBCOMMANDS:
    list        List recent audit events
    export      Export audit trail in various formats (FDA, NIH, EMA)
    verify      Verify audit chain integrity
    archive     Archive old events (post-MVP)
    annotate    Add notes to an event (post-MVP)

EXAMPLES:
    # List last 20 events
    bdp audit list --limit 20

    # Export FDA compliance report
    bdp audit export --format fda --output audit-fda-2026.json

    # Generate Data Availability Statement
    bdp audit export --format das --output data-availability.md

    # Verify audit chain integrity
    bdp audit verify
```

### bdp audit export --help

```
Export audit trail for regulatory compliance or research documentation

FORMATS:
    fda     FDA 21 CFR Part 11 compliant JSON report
    nih     NIH Data Management & Sharing (DMS) markdown report
    ema     EMA ALCOA++ compliance YAML report
    das     Data Availability Statement (markdown)
    json    Raw JSON export of all events

IMPORTANT: Exported reports include a disclaimer that the audit trail is
locally stored, editable, and intended for research documentation.

USAGE:
    bdp audit export [OPTIONS] --format <FORMAT>

OPTIONS:
    -f, --format <FORMAT>    Export format (fda, nih, ema, das, json)
    -o, --output <FILE>      Output file path
        --from <DATE>        Start date (ISO 8601)
        --to <DATE>          End date (ISO 8601)

EXAMPLES:
    # FDA compliance report
    bdp audit export --format fda --output audit-fda.json

    # Data Availability Statement for publication
    bdp audit export --format das --output methods/data-availability.md

    # Export events from specific date range
    bdp audit export --format json --from 2026-01-01 --to 2026-01-31
```

## Implementation Phases

### Phase 1: Core Audit (MVP)

**Deliverables**:
- [ ] SQLite schema (audit_events, files, generated_files)
- [ ] Machine ID generation
- [ ] CQRS audit middleware trait
- [ ] LocalAuditLogger implementation
- [ ] Dependency injection in CLI commands
- [ ] Basic event logging (download, verify, post_pull)
- [ ] Hash chain computation (for verification)

**Commands**:
- [ ] `bdp audit list` - List recent events
- [ ] `bdp audit verify` - Verify chain integrity

**Testing**:
- [ ] Unit tests for middleware pattern
- [ ] Integration tests for audit logging
- [ ] Verification tests

**Estimated**: 3-5 days

### Phase 2: Post-Pull Hooks (MVP)

**Deliverables**:
- [ ] Pre-defined tool registry (samtools, blast, bwa)
- [ ] Wildcard pattern matching
- [ ] Post-pull execution with audit logging
- [ ] Generated file tracking
- [ ] Verification of generated files

**Commands**:
- [ ] `bdp verify` - Include generated files

**Testing**:
- [ ] Pattern matching tests
- [ ] Tool execution tests
- [ ] Generated file verification

**Estimated**: 2-3 days

### Phase 3: Export Formats (MVP)

**Deliverables**:
- [ ] FDA 21 CFR Part 11 JSON export
- [ ] NIH DMS markdown export
- [ ] EMA ALCOA++ YAML export
- [ ] Data Availability Statement generator
- [ ] Export snapshot tracking

**Commands**:
- [ ] `bdp audit export --format <fda|nih|ema|das|json>`

**Testing**:
- [ ] Export format validation
- [ ] Template tests

**Estimated**: 2-3 days

### Phase 4: Custom Hooks (Post-MVP)

**Deliverables**:
- [ ] `.bdp/hooks/` directory support
- [ ] Custom script discovery
- [ ] Hook execution with audit logging
- [ ] Hook validation

**Configuration**:
- [ ] Hook registry in `.bdp/hooks/registry.yml`

**Testing**:
- [ ] Custom hook execution
- [ ] Security validation

**Estimated**: 2-3 days

### Phase 5: Archive System (Post-MVP)

**Deliverables**:
- [ ] Archive old events to JSON
- [ ] `audit_events_archived` table
- [ ] Auto-archive configuration
- [ ] Verification with archived events

**Commands**:
- [ ] `bdp audit archive --before <date>`

**Testing**:
- [ ] Archive and restore
- [ ] Verification with archives

**Estimated**: 1-2 days

### Phase 6: Backend Integration (Post-MVP)

**Deliverables**:
- [ ] BackendAuditLogger implementation
- [ ] API client for audit endpoints
- [ ] Fallback to local when offline
- [ ] Sync local → backend

**Configuration**:
- [ ] Backend URL in config
- [ ] Authentication token

**Testing**:
- [ ] Backend integration tests
- [ ] Offline fallback tests

**Estimated**: 3-5 days

## Related Documents

- [File Formats](./file-formats.md) - bdp.yml, bdl.lock structure
- [Cache Strategy](./cache-strategy.md) - Local cache management
- [CLI Development](../cli-development.md) - CLI command patterns
- [Backend Architecture](../backend-architecture.md) - CQRS pattern (server-side reference)

## Open Questions

1. **Checksum verification for generated files**: Should we compute checksums for all post-pull outputs, or just verify existence?
   - **Proposal**: For deterministic tools (samtools, BLAST), verify checksums. For non-deterministic, just check existence.

2. **Archive retention policy**: How long should archived events be kept?
   - **Proposal**: User-configurable, default = never delete (storage is cheap)

3. **Backend sync strategy**: Should local audit auto-sync to backend when available?
   - **Proposal**: Post-MVP, opt-in sync with `bdp audit sync --backend`

4. **Custom hook security**: Should we validate/sandbox custom hooks?
   - **Proposal**: Post-MVP, add hook validation (checksum verification, allow-list)

---

**Version**: 1.0
**Last Updated**: 2026-01-18
**Status**: Draft - Ready for Implementation
