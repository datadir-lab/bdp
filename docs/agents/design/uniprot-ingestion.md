# UniProt Ingestion Strategy

Automated scraping and ingestion of UniProt protein database releases.

## Overview

UniProt releases new versions monthly with:
- **SwissProt**: ~570k manually curated proteins
- **TremBL**: ~250M computationally annotated proteins

BDP ingests these releases automatically via cron jobs, creating:
- Individual protein data sources
- Multiple file formats (FASTA, XML, DAT, JSON)
- Aggregate sources (`uniprot:all@version`)
- Version mappings (external → internal)
- Full-text search indexes

## UniProt Release Structure

### FTP Directory

```
ftp://ftp.uniprot.org/pub/databases/uniprot/
├── current_release/
│   ├── knowledgebase/
│   │   ├── complete/
│   │   │   ├── uniprot_sprot.dat.gz       # SwissProt DAT (metadata)
│   │   │   ├── uniprot_sprot.fasta.gz     # SwissProt FASTA
│   │   │   ├── uniprot_sprot.xml.gz       # SwissProt XML
│   │   │   ├── uniprot_trembl.dat.gz      # TremBL DAT
│   │   │   └── ...
│   │   └── taxonomic_divisions/
│   │       ├── uniprot_sprot_human.dat.gz  # Human proteins only
│   │       └── ...
│   └── relnotes.txt                        # Release notes
└── previous_releases/
    ├── release-2024_06/
    ├── release-2025_01/
    └── ...
```

### Release Metadata

**relnotes.txt** contains:
- Release name: `2025_01`
- Release date: `15-Jan-2025`
- Entry counts: SwissProt (570,000), TremBL (250,000,000)
- Changes: New entries, updated entries, deleted entries

## Ingestion Architecture

### Cron Job Schedule

```
# Run daily at 2 AM UTC
0 2 * * * /usr/local/bin/bdp-ingest uniprot
```

Or programmatically:
```rust
use tokio_cron_scheduler::{JobScheduler, Job};

#[tokio::main]
async fn main() -> Result<()> {
    let sched = JobScheduler::new().await?;

    sched.add(
        Job::new_async("0 0 2 * * *", |_uuid, _l| {
            Box::pin(async {
                if let Err(e) = ingest_uniprot().await {
                    error!("UniProt ingestion failed: {}", e);
                }
            })
        })?
    ).await?;

    sched.start().await?;

    // Keep running
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

### High-Level Workflow

```
1. Check for new release
2. Download release files
3. Parse DAT files (protein metadata)
4. Extract per-protein data
5. Upload files to S3
6. Insert into database
7. Create version mapping
8. Build aggregate source
9. Update search indexes
10. Send notifications
```

## Implementation

### Step 1: Check for New Release

```rust
async fn check_new_release() -> Result<Option<ReleaseInfo>> {
    let ftp_client = FtpStream::connect("ftp.uniprot.org:21")?;
    ftp_client.login("anonymous", "anonymous")?;

    // Read release notes
    let notes = ftp_client.simple_retr(
        "/pub/databases/uniprot/current_release/relnotes.txt"
    )?;

    let release_info = parse_release_notes(&notes)?;

    // Check if already ingested
    if version_exists("uniprot", &release_info.external_version).await? {
        return Ok(None);
    }

    Ok(Some(release_info))
}

struct ReleaseInfo {
    external_version: String,    // "2025_01"
    release_date: NaiveDate,     // 2025-01-15
    swissprot_count: u64,        // 570000
    trembl_count: u64,           // 250000000
}

fn parse_release_notes(content: &str) -> Result<ReleaseInfo> {
    let version_re = Regex::new(r"Release (\d{4}_\d{2})")?;
    let date_re = Regex::new(r"(\d{1,2}-[A-Za-z]{3}-\d{4})")?;
    let sprot_re = Regex::new(r"UniProtKB/Swiss-Prot.*?(\d+) sequence entries")?;

    let external_version = version_re.captures(content)
        .ok_or_else(|| anyhow!("Version not found"))?[1]
        .to_string();

    let date_str = date_re.captures(content)
        .ok_or_else(|| anyhow!("Date not found"))?[1]
        .to_string();
    let release_date = NaiveDate::parse_from_str(&date_str, "%d-%b-%Y")?;

    let swissprot_count = sprot_re.captures(content)
        .ok_or_else(|| anyhow!("SwissProt count not found"))?[1]
        .parse()?;

    Ok(ReleaseInfo {
        external_version,
        release_date,
        swissprot_count,
        trembl_count: 0,  // Parse similarly
    })
}
```

### Step 2: Download Release Files

```rust
async fn download_release_files(
    release: &ReleaseInfo,
    temp_dir: &Path
) -> Result<DownloadedFiles> {
    let ftp_client = FtpStream::connect("ftp.uniprot.org:21")?;
    ftp_client.login("anonymous", "anonymous")?;

    let base_path = "/pub/databases/uniprot/current_release/knowledgebase/complete";

    // Download files
    let files = vec![
        "uniprot_sprot.dat.gz",
        "uniprot_sprot.fasta.gz",
        "uniprot_sprot.xml.gz",
    ];

    let mut downloaded = Vec::new();

    for file in files {
        let remote_path = format!("{}/{}", base_path, file);
        let local_path = temp_dir.join(file);

        info!("Downloading {} ({:.1} GB)", file, estimate_size(file));

        let mut reader = ftp_client.retr_as_stream(&remote_path)?;
        let mut writer = File::create(&local_path)?;

        std::io::copy(&mut reader, &mut writer)?;

        info!("Downloaded {}", file);
        downloaded.push(local_path);
    }

    Ok(DownloadedFiles {
        dat: temp_dir.join("uniprot_sprot.dat.gz"),
        fasta: temp_dir.join("uniprot_sprot.fasta.gz"),
        xml: temp_dir.join("uniprot_sprot.xml.gz"),
    })
}
```

### Step 3: Parse DAT File

DAT format is UniProt's flat file format containing all metadata.

**Example Entry**:
```
ID   INS_HUMAN               Reviewed;         110 AA.
AC   P01308;
DT   21-JUL-1986, integrated into UniProtKB/Swiss-Prot.
DE   RecName: Full=Insulin;
GN   Name=INS;
OS   Homo sapiens (Human).
OC   Eukaryota; Metazoa; Chordata; Craniata; Vertebrata; Euteleostomi;
OC   Mammalia; Eutheria; Euarchontoglires; Primates; Haplorrhini;
OC   Catarrhini; Hominidae; Homo.
OX   NCBI_TaxID=9606;
RN   [1]
RP   NUCLEOTIDE SEQUENCE.
RX   PubMed=6265056; DOI=10.1038/292086a0;
RA   Bell G.I., Pictet R.L., Rutter W.J., Cordell B., Tischer E., Goodman H.M.;
RT   "Sequence of the human insulin gene.";
RL   Nature 284:26-32(1980).
SQ   SEQUENCE   110 AA;  11937 MW;  6F2B89D7AAAC28AC CRC64;
     MALWMRLLPL LALLALWGPD PAAAFVNQHL CGSHLVEALY LVCGERGFFY TPKTRREAED
     LQVGQVELGG GPGAGSLQPL ALEGSLQKRG IVEQCCTSIC SLYQLENYCN
//
```

**Parser**:
```rust
struct ProteinEntry {
    accession: String,           // P01308
    entry_name: String,          // INS_HUMAN
    protein_name: String,        // Insulin
    gene_name: Option<String>,   // INS
    organism_name: String,       // Homo sapiens
    taxonomy_id: i32,            // 9606
    sequence: String,            // MALWMRLLPL...
    sequence_length: i32,        // 110
    mass_da: i64,                // 11937
    citations: Vec<Citation>,
    // ... more fields
}

fn parse_dat_file(path: &Path) -> Result<Vec<ProteinEntry>> {
    let file = File::open(path)?;
    let reader = GzDecoder::new(file);
    let buffered = BufReader::new(reader);

    let mut entries = Vec::new();
    let mut current_entry = ProteinEntry::default();
    let mut in_sequence = false;

    for line in buffered.lines() {
        let line = line?;

        if line.starts_with("ID   ") {
            // New entry
            current_entry = ProteinEntry::default();
            let parts: Vec<&str> = line.split_whitespace().collect();
            current_entry.entry_name = parts[1].trim_end_matches(';').to_string();
        }
        else if line.starts_with("AC   ") {
            // Accession
            current_entry.accession = line[5..]
                .split(';')
                .next()
                .unwrap()
                .trim()
                .to_string();
        }
        else if line.starts_with("DE   RecName: Full=") {
            // Protein name
            current_entry.protein_name = line
                .split("Full=")
                .nth(1)
                .unwrap()
                .trim_end_matches(';')
                .to_string();
        }
        else if line.starts_with("GN   Name=") {
            // Gene name
            current_entry.gene_name = Some(
                line.split("Name=")
                    .nth(1)
                    .unwrap()
                    .split(';')
                    .next()
                    .unwrap()
                    .trim()
                    .to_string()
            );
        }
        else if line.starts_with("OX   NCBI_TaxID=") {
            // Taxonomy ID
            current_entry.taxonomy_id = line
                .split('=')
                .nth(1)
                .unwrap()
                .trim_end_matches(';')
                .parse()?;
        }
        else if line.starts_with("SQ   SEQUENCE") {
            // Sequence metadata
            let parts: Vec<&str> = line.split_whitespace().collect();
            current_entry.sequence_length = parts[2].parse()?;
            current_entry.mass_da = parts[4].parse()?;
            in_sequence = true;
        }
        else if in_sequence && !line.starts_with("//") {
            // Sequence data
            current_entry.sequence.push_str(
                &line.trim().replace(' ', "")
            );
        }
        else if line == "//" {
            // End of entry
            entries.push(current_entry.clone());
            in_sequence = false;
        }
    }

    Ok(entries)
}
```

### Step 4: Extract Per-Protein Files

```rust
async fn extract_per_protein_files(
    proteins: &[ProteinEntry],
    fasta_path: &Path,
    xml_path: &Path,
    temp_dir: &Path
) -> Result<HashMap<String, ProteinFiles>> {
    let mut files_map = HashMap::new();

    // Parse FASTA
    let fasta_records = parse_fasta(fasta_path)?;

    // Parse XML
    let xml_records = parse_xml(xml_path)?;

    for protein in proteins {
        let accession = &protein.accession;

        // Create directory for this protein
        let protein_dir = temp_dir.join(accession);
        fs::create_dir_all(&protein_dir)?;

        // Write FASTA
        let fasta_file = protein_dir.join(format!("{}.fasta", accession));
        let fasta_record = fasta_records.get(accession)
            .ok_or_else(|| anyhow!("FASTA not found for {}", accession))?;
        fs::write(&fasta_file, fasta_record)?;

        // Write XML
        let xml_file = protein_dir.join(format!("{}.xml", accession));
        let xml_record = xml_records.get(accession)
            .ok_or_else(|| anyhow!("XML not found for {}", accession))?;
        fs::write(&xml_file, xml_record)?;

        // Generate JSON from parsed data
        let json_file = protein_dir.join(format!("{}.json", accession));
        let json_data = serde_json::to_string_pretty(protein)?;
        fs::write(&json_file, json_data)?;

        files_map.insert(accession.clone(), ProteinFiles {
            fasta: fasta_file,
            xml: xml_file,
            json: json_file,
        });
    }

    Ok(files_map)
}
```

### Step 5: Upload to S3

```rust
async fn upload_protein_files(
    accession: &str,
    internal_version: &str,
    files: &ProteinFiles,
    s3_client: &S3Client
) -> Result<UploadedFiles> {
    let bucket = "bdp-data";
    let base_key = format!("proteins/uniprot/{}/{}", accession, internal_version);

    let mut uploaded = UploadedFiles::default();

    // Upload FASTA
    let fasta_key = format!("{}/{}.fasta", base_key, accession);
    upload_file(s3_client, bucket, &fasta_key, &files.fasta).await?;
    uploaded.fasta_s3_key = fasta_key;
    uploaded.fasta_checksum = compute_checksum(&files.fasta).await?;
    uploaded.fasta_size = fs::metadata(&files.fasta)?.len();

    // Upload XML
    let xml_key = format!("{}/{}.xml.gz", base_key, accession);
    let compressed_xml = compress_file(&files.xml).await?;
    upload_file(s3_client, bucket, &xml_key, &compressed_xml).await?;
    uploaded.xml_s3_key = xml_key;
    uploaded.xml_checksum = compute_checksum(&compressed_xml).await?;
    uploaded.xml_size = fs::metadata(&compressed_xml)?.len();

    // Upload JSON
    let json_key = format!("{}/{}.json", base_key, accession);
    upload_file(s3_client, bucket, &json_key, &files.json).await?;
    uploaded.json_s3_key = json_key;
    uploaded.json_checksum = compute_checksum(&files.json).await?;
    uploaded.json_size = fs::metadata(&files.json)?.len();

    Ok(uploaded)
}

async fn upload_file(
    s3_client: &S3Client,
    bucket: &str,
    key: &str,
    file_path: &Path
) -> Result<()> {
    let body = ByteStream::from_path(file_path).await?;

    s3_client.put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .send()
        .await?;

    info!("Uploaded {}", key);
    Ok(())
}
```

### Step 6: Insert into Database

```rust
async fn ingest_protein(
    pool: &PgPool,
    protein: &ProteinEntry,
    internal_version: &str,
    external_version: &str,
    uploaded_files: &UploadedFiles
) -> Result<Uuid> {
    // Get or create organization
    let org_id = get_or_create_organization(pool, "uniprot").await?;

    // Get or create organism
    let organism_id = get_or_create_organism(
        pool,
        protein.taxonomy_id,
        &protein.organism_name
    ).await?;

    // Create registry entry
    let entry_id = sqlx::query_scalar!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
        VALUES ($1, $2, $3, $4, 'data_source')
        ON CONFLICT (slug) DO UPDATE SET updated_at = NOW()
        RETURNING id
        "#,
        org_id,
        protein.accession,
        format!("{} [{}]", protein.protein_name, protein.organism_name),
        format!("UniProt protein: {}", protein.protein_name),
    )
    .fetch_one(pool)
    .await?;

    // Create data source
    sqlx::query!(
        r#"
        INSERT INTO data_sources (id, source_type, external_id, organism_id)
        VALUES ($1, 'protein', $2, $3)
        ON CONFLICT (id) DO NOTHING
        "#,
        entry_id,
        protein.accession,
        organism_id,
    )
    .execute(pool)
    .await?;

    // Create protein metadata
    sqlx::query!(
        r#"
        INSERT INTO protein_metadata
            (data_source_id, accession, entry_name, protein_name, gene_name,
             sequence_length, mass_da, sequence_checksum)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (accession) DO UPDATE SET
            entry_name = $3,
            protein_name = $4,
            gene_name = $5
        "#,
        entry_id,
        protein.accession,
        protein.entry_name,
        protein.protein_name,
        protein.gene_name,
        protein.sequence_length,
        protein.mass_da,
        compute_sequence_checksum(&protein.sequence),
    )
    .execute(pool)
    .await?;

    // Create version
    let version_id = sqlx::query_scalar!(
        r#"
        INSERT INTO versions
            (entry_id, version, external_version, release_date, size_bytes)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (entry_id, version) DO NOTHING
        RETURNING id
        "#,
        entry_id,
        internal_version,
        external_version,
        protein.release_date,
        uploaded_files.total_size() as i64,
    )
    .fetch_one(pool)
    .await?;

    // Insert version files
    for (format, s3_key, checksum, size) in uploaded_files.iter() {
        sqlx::query!(
            r#"
            INSERT INTO version_files
                (version_id, format, s3_key, checksum, size_bytes, compression)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            version_id,
            format,
            s3_key,
            checksum,
            size as i64,
            if format == "xml" { Some("gzip") } else { None },
        )
        .execute(pool)
        .await?;
    }

    // Insert citations
    for citation in &protein.citations {
        insert_citation(pool, version_id, citation).await?;
    }

    Ok(version_id)
}
```

### Step 7: Create Aggregate Source

```rust
async fn create_aggregate_source(
    pool: &PgPool,
    org_id: Uuid,
    internal_version: &str,
    external_version: &str,
    protein_ids: &[Uuid]
) -> Result<Uuid> {
    // Create registry entry for aggregate
    let entry_id = sqlx::query_scalar!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
        VALUES ($1, 'all', 'All UniProt Proteins', 'Complete UniProt database', 'data_source')
        ON CONFLICT (slug) DO UPDATE SET updated_at = NOW()
        RETURNING id
        "#,
        org_id,
    )
    .fetch_one(pool)
    .await?;

    // Create data source
    sqlx::query!(
        r#"
        INSERT INTO data_sources (id, source_type)
        VALUES ($1, 'protein')
        ON CONFLICT (id) DO NOTHING
        "#,
        entry_id,
    )
    .execute(pool)
    .await?;

    // Create version
    let version_id = sqlx::query_scalar!(
        r#"
        INSERT INTO versions
            (entry_id, version, external_version, release_date)
        VALUES ($1, $2, $3, NOW())
        RETURNING id
        "#,
        entry_id,
        internal_version,
        external_version,
    )
    .fetch_one(pool)
    .await?;

    // Insert dependencies (batch for performance)
    let mut tx = pool.begin().await?;

    for chunk in protein_ids.chunks(1000) {
        let mut query = String::from(
            "INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version) VALUES "
        );

        for (i, protein_id) in chunk.iter().enumerate() {
            if i > 0 {
                query.push_str(", ");
            }
            query.push_str(&format!("('{}', '{}', '{}')", version_id, protein_id, internal_version));
        }

        sqlx::query(&query).execute(&mut *tx).await?;
    }

    tx.commit().await?;

    info!("Created aggregate source with {} dependencies", protein_ids.len());

    Ok(version_id)
}
```

### Step 8: Update Search Indexes

```rust
async fn update_search_indexes(pool: &PgPool) -> Result<()> {
    // Rebuild full-text search indexes
    sqlx::query("REINDEX INDEX registry_entries_search_idx")
        .execute(pool)
        .await?;

    sqlx::query("REINDEX INDEX protein_metadata_search_idx")
        .execute(pool)
        .await?;

    // Update statistics
    sqlx::query("ANALYZE registry_entries")
        .execute(pool)
        .await?;

    sqlx::query("ANALYZE protein_metadata")
        .execute(pool)
        .await?;

    info!("Search indexes updated");
    Ok(())
}
```

### Step 9: Send Notifications

```rust
async fn send_notifications(
    release: &ReleaseInfo,
    ingestion_stats: &IngestionStats
) -> Result<()> {
    let message = format!(
        "UniProt {} ingestion complete!\n\
         - Proteins ingested: {}\n\
         - Total size: {} GB\n\
         - Duration: {} minutes",
        release.external_version,
        ingestion_stats.proteins_count,
        ingestion_stats.total_size_gb,
        ingestion_stats.duration_minutes
    );

    // Log
    info!("{}", message);

    // TODO: Send email, Slack notification, webhook, etc.

    Ok(())
}
```

## Configuration

```toml
# config/ingest.toml

[uniprot]
enabled = true
ftp_url = "ftp://ftp.uniprot.org/pub/databases/uniprot/"
oldest_version = "2020_01"  # Don't sync older
databases = ["swissprot"]   # ["swissprot", "trembl"]

[uniprot.formats]
fasta = true
xml = true
dat = false  # Don't upload raw DAT files
json = true  # Generate from parsed data

[uniprot.filters]
# Optional: only ingest specific organisms
# organisms = [9606, 10090]  # Human, Mouse

[storage]
bucket = "bdp-data"
region = "us-east-1"

[database]
url = "postgresql://user:pass@localhost/bdp"
max_connections = 20

[performance]
parallel_uploads = 10
batch_size = 1000
```

## Performance Optimization

### Parallel Processing

```rust
async fn ingest_proteins_parallel(
    proteins: Vec<ProteinEntry>,
    internal_version: &str,
    pool: &PgPool,
    s3_client: &S3Client
) -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(10));  // Max 10 concurrent
    let mut tasks = Vec::new();

    for protein in proteins {
        let permit = semaphore.clone().acquire_owned().await?;
        let pool = pool.clone();
        let s3_client = s3_client.clone();
        let version = internal_version.to_string();

        let task = tokio::spawn(async move {
            let result = process_protein(&protein, &version, &pool, &s3_client).await;
            drop(permit);
            result
        });

        tasks.push(task);
    }

    // Wait for all
    for task in tasks {
        task.await??;
    }

    Ok(())
}
```

### Database Batching

Use batch inserts for better performance:
- Dependencies: Insert 1000 at a time
- Version files: Batch per protein
- Use transactions for atomicity

### Progress Tracking

```rust
use indicatif::{ProgressBar, ProgressStyle};

async fn ingest_with_progress(proteins: Vec<ProteinEntry>) -> Result<()> {
    let pb = ProgressBar::new(proteins.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
    );

    for protein in proteins {
        pb.set_message(format!("Processing {}", protein.accession));
        process_protein(&protein).await?;
        pb.inc(1);
    }

    pb.finish_with_message("Ingestion complete!");
    Ok(())
}
```

## Error Handling

### Retry Logic

```rust
async fn ingest_with_retry(protein: &ProteinEntry) -> Result<()> {
    let mut attempts = 0;
    let max_attempts = 3;

    loop {
        match process_protein(protein).await {
            Ok(_) => return Ok(()),
            Err(e) if attempts < max_attempts => {
                attempts += 1;
                warn!("Attempt {}/{} failed for {}: {}",
                    attempts, max_attempts, protein.accession, e);
                tokio::time::sleep(Duration::from_secs(5 * attempts)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### Partial Failure Recovery

```rust
async fn resume_ingestion(
    release: &ReleaseInfo,
    pool: &PgPool
) -> Result<Vec<String>> {
    // Find proteins already ingested
    let ingested = sqlx::query_scalar!(
        r#"
        SELECT pm.accession
        FROM protein_metadata pm
        JOIN data_sources ds ON ds.id = pm.data_source_id
        JOIN versions v ON v.entry_id = ds.id
        WHERE v.external_version = $1
        "#,
        release.external_version
    )
    .fetch_all(pool)
    .await?;

    Ok(ingested)
}
```

## Monitoring

### Metrics

```rust
use prometheus::{IntCounter, Histogram, register_int_counter, register_histogram};

lazy_static! {
    static ref PROTEINS_INGESTED: IntCounter = register_int_counter!(
        "bdp_proteins_ingested_total",
        "Total proteins ingested"
    ).unwrap();

    static ref INGESTION_DURATION: Histogram = register_histogram!(
        "bdp_ingestion_duration_seconds",
        "Time to ingest one protein"
    ).unwrap();
}

async fn process_protein_with_metrics(protein: &ProteinEntry) -> Result<()> {
    let timer = INGESTION_DURATION.start_timer();

    process_protein(protein).await?;

    timer.observe_duration();
    PROTEINS_INGESTED.inc();

    Ok(())
}
```

### Logging

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(pool, s3_client))]
async fn ingest_uniprot(pool: &PgPool, s3_client: &S3Client) -> Result<()> {
    info!("Starting UniProt ingestion");

    let release = check_new_release().await?;
    if release.is_none() {
        info!("No new release found");
        return Ok(());
    }

    let release = release.unwrap();
    info!("Found new release: {}", release.external_version);

    // ... ingestion logic

    info!("Ingestion complete");
    Ok(())
}
```

## Testing

### Unit Tests

```rust
#[test]
fn test_parse_release_notes() {
    let notes = r#"
    UniProt Release 2025_01
    Release Date: 15-Jan-2025
    UniProtKB/Swiss-Prot contains 570000 sequence entries
    "#;

    let info = parse_release_notes(notes).unwrap();
    assert_eq!(info.external_version, "2025_01");
    assert_eq!(info.swissprot_count, 570000);
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_full_ingestion_pipeline() {
    let pool = setup_test_db().await;
    let s3_client = setup_test_s3().await;

    let test_protein = ProteinEntry {
        accession: "P01308".into(),
        // ... test data
    };

    ingest_protein(&pool, &test_protein, "1.0", "2025_01", &files).await.unwrap();

    // Verify
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM protein_metadata WHERE accession = 'P01308'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, Some(1));
}
```

## Related Documents

- [Database Schema](./database-schema.md) - Data model
- [Version Mapping](./version-mapping.md) - Version translation
- [API Design](./api-design.md) - Exposed endpoints
- [Dependency Resolution](./dependency-resolution.md) - Aggregate sources
