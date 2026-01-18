# UniProt Ingestion Testing Guide

## Overview

This document describes the testing strategy for the idempotent UniProt ingestion pipeline.

## Test Coverage

### 1. Version Discovery & Idempotency ✅

**Test File**: `tests/uniprot_integration_test.rs`

#### Test Scenarios

1. **Version Discovery Filters Already Ingested**
   - Given: Versions 2024_11, 2024_12, 2025_01 available on FTP
   - And: 2024_11 already ingested
   - When: Running version discovery
   - Then: Only 2024_12 and 2025_01 are identified as new

2. **Idempotent Pipeline Skips Ingested Versions**
   - Given: Version 2024_12 marked as completed in database
   - When: Checking ingestion status
   - Then: `is_version_ingested("2024_12")` returns true
   - And: `is_version_ingested("2025_01")` returns false

3. **Versions Processed Oldest First**
   - Given: Versions 2025_01, 2024_11, 2024_12 discovered (unsorted)
   - When: Sorting versions
   - Then: Order is 2024_11 → 2024_12 → 2025_01

### 2. Current → Versioned Migration ✅

**Test File**: `tests/uniprot_integration_test.rs::test_current_to_versioned_migration`

#### Scenario

**Initial State**:
```
FTP Structure:
├── current_release/ → version 2025_01
└── previous_releases/
    ├── release-2024_11/
    └── release-2024_12/

Database:
- Job: version=2025_01, is_current=true, status=completed
```

**After UniProt Updates** (new release 2025_02):
```
FTP Structure:
├── current_release/ → version 2025_02 (NEW)
└── previous_releases/
    ├── release-2024_11/
    ├── release-2024_12/
    └── release-2025_01/ (MOVED from current)
```

**Expected Behavior**:
- ✅ Discover 2025_02 as new current
- ✅ See 2025_01 in previous_releases
- ✅ Recognize 2025_01 is same version (just migrated)
- ✅ **DO NOT re-ingest 2025_01**
- ✅ Only ingest 2025_02

**Test Assertion**:
```rust
let should_reingest = discovery.should_reingest(
    &discovered_old_as_previous, // 2025_01 in previous_releases
    "2025_01",                    // What we ingested
    true,                         // Was ingested as current
);

assert!(!should_reingest); // Should NOT re-ingest
```

### 3. DAT Parsing Validation ✅

**Test File**: `tests/uniprot_parsing_test.rs`

#### Test Cases

1. **Parse Sample DAT Entry**
   - Real UniProt entry (P00505 - Aspartate aminotransferase)
   - Validates all fields extracted correctly:
     - ✅ Accession: P00505
     - ✅ Entry name: AATM_HUMAN
     - ✅ Protein name: "Aspartate aminotransferase, mitochondrial"
     - ✅ Gene name: GOT2
     - ✅ Organism: "Homo sapiens (Human)"
     - ✅ Taxonomy ID: 9606
     - ✅ Sequence length: 401 AA
     - ✅ Molecular mass: 47476 Da
     - ✅ Sequence: 401 characters, uppercase letters only

2. **Parse Multiple Entries**
   - Creates 2 entries in one file
   - Validates both parsed correctly
   - Checks no cross-contamination

3. **Parse With Limit**
   - Creates 5 entries
   - Parser with limit=3
   - Validates only 3 parsed

4. **FASTA Generation**
   - Validates format: `>sp|{acc}|{name} {protein} OS={org} OX={tax} GN={gene}`
   - Checks sequence wrapped at 60 characters
   - Example output:
   ```
   >sp|P00505|AATM_HUMAN Aspartate aminotransferase, mitochondrial OS=Homo sapiens (Human) OX=9606 GN=GOT2
   MALLHSGRVLSGASAAATAVKFERTILKTPEKTVRAIVPGVFGRTLQEAGKQFRNALQLE
   ANPDVAISAGVRTDDVLGKTGIDITHGQQKQFHPRYIRVPKVLDGDVVIEVHGRYAAGGI
   ...
   ```

5. **JSON Generation**
   - Validates well-formed JSON
   - Checks all fields present
   - Example:
   ```json
   {
     "accession": "P00505",
     "entry_name": "AATM_HUMAN",
     "protein_name": "Aspartate aminotransferase, mitochondrial",
     "gene_name": "GOT2",
     "organism_name": "Homo sapiens (Human)",
     "taxonomy_id": 9606,
     "sequence": "MALLHSGRVL...",
     "sequence_length": 401,
     "mass_da": 47476
   }
   ```

6. **Sequence Checksum**
   - SHA-256 of sequence
   - 64 hex characters
   - Deterministic (same sequence = same checksum)

7. **Entry Validation**
   - Checks all required fields present
   - Validates sequence length matches actual length
   - Ensures taxonomy ID positive
   - Ensures mass positive

8. **Edge Cases**
   - Empty file → 0 entries
   - Malformed entry → handled gracefully
   - Missing optional fields → works without them

## Running Tests

### Unit Tests

```bash
# Run all parsing tests
cargo test --package bdp-server --test uniprot_parsing_test

# Run specific test
cargo test --package bdp-server --test uniprot_parsing_test test_parse_sample_dat_entry

# Run with output
cargo test --package bdp-server --test uniprot_parsing_test -- --nocapture
```

### Integration Tests

```bash
# Run all integration tests (requires database)
cargo test --package bdp-server --test uniprot_integration_test

# Run specific integration test
cargo test --package bdp-server --test uniprot_integration_test test_current_to_versioned_migration
```

### Prerequisites for Integration Tests

1. **Database Running**:
   ```bash
   docker-compose up -d postgres
   ```

2. **Environment Variable**:
   ```bash
   export DATABASE_URL="postgresql://bdp:bdp_dev_password@localhost:5432/bdp"
   ```

3. **Migrations Applied**:
   ```bash
   sqlx migrate run
   ```

## Manual Testing with Real FTP Data

### Test Plan

1. **Small Sample Test**
   ```bash
   # Download first 100 proteins from UniProt
   curl "ftp://ftp.uniprot.org/pub/databases/uniprot/current_release/knowledgebase/complete/uniprot_sprot.dat.gz" | \
     gunzip | head -5000 > test_sample.dat

   # Parse with our parser
   cargo test test_parse_real_ftp_data -- --ignored
   ```

2. **Full Version Test**
   ```bash
   # Test complete pipeline with real version
   cargo run --bin test-uniprot-ingest -- \
     --version "2024_12" \
     --limit 1000
   ```

3. **Idempotency Test**
   ```bash
   # Run twice, should skip second time
   cargo run --bin test-uniprot-ingest -- --version "2024_12"
   cargo run --bin test-uniprot-ingest -- --version "2024_12"
   # Second run should show: "Version already ingested, skipping"
   ```

4. **Migration Test**
   ```bash
   # Simulate current → previous migration
   # 1. Ingest as current
   cargo run --bin test-uniprot-ingest -- --version "2025_01" --as-current

   # 2. Manually move to previous_releases in database
   psql -c "UPDATE ingestion_jobs SET source_metadata = jsonb_set(source_metadata, '{is_current}', 'false') WHERE external_version = '2025_01';"

   # 3. Re-run discovery
   cargo run --bin test-uniprot-ingest -- --discover-only
   # Should NOT re-ingest 2025_01
   ```

## Expected Parsing Accuracy

Based on UniProt DAT format specification (https://web.expasy.org/docs/userman.html):

| Field | Required | Example | Our Parser |
|-------|----------|---------|------------|
| ID (Entry name) | ✅ | `AATM_HUMAN` | ✅ Extracted |
| AC (Accession) | ✅ | `P00505` | ✅ Primary AC |
| DT (Date) | ✅ | `21-JUL-1986` | ✅ Parsed |
| DE (Description) | ✅ | `RecName: Full=...` | ✅ Protein name |
| GN (Gene name) | ❌ | `Name=GOT2` | ✅ Extracted |
| OS (Organism) | ✅ | `Homo sapiens` | ✅ Extracted |
| OX (Taxonomy) | ✅ | `NCBI_TaxID=9606` | ✅ Extracted |
| SQ (Sequence) | ✅ | `SEQUENCE 401 AA` | ✅ Full sequence |

### Known Limitations

1. **Secondary Accessions**: Only primary AC is extracted
2. **Full DE Parsing**: Currently extracts RecName: Full, not all alternatives
3. **Cross-References**: Parsed but not indexed
4. **Features**: Parsed but not structured
5. **Comments**: Parsed as text, not structured

## Test Results

### Parsing Tests

```
test test_empty_file ... ok
test test_entry_validation ... ok
test test_expected_uniprot_format ... ok
test test_fasta_generation ... ok
test test_json_generation ... ok
test test_malformed_entry ... ok
test test_parse_multiple_entries ... ok
test test_parse_sample_dat_entry ... ok
test test_parse_with_limit ... ok
test test_sequence_checksum ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Integration Tests

```
test test_current_to_versioned_migration ... ok
test test_idempotent_pipeline_skips_ingested ... ok
test test_idempotent_stats_calculation ... ok
test test_version_discovery_filters_ingested ... ok
test test_versions_processed_oldest_first ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Validation Against UniProt Reference

### Sample Entry Validation

**Entry**: P00505 (Aspartate aminotransferase, mitochondrial)

**UniProt Web**: https://www.uniprot.org/uniprotkb/P00505

| Field | UniProt Web | Our Parser | Match |
|-------|-------------|------------|-------|
| Entry | AATM_HUMAN | AATM_HUMAN | ✅ |
| Primary AC | P00505 | P00505 | ✅ |
| Protein | Aspartate aminotransferase, mitochondrial | Aspartate aminotransferase, mitochondrial | ✅ |
| Gene | GOT2 | GOT2 | ✅ |
| Organism | Homo sapiens | Homo sapiens (Human) | ✅ |
| TaxID | 9606 | 9606 | ✅ |
| Length | 401 AA | 401 | ✅ |
| Mass | 47,476 Da | 47476 | ✅ |

✅ **100% accuracy on all tested fields**

## Continuous Testing

### Pre-commit Checks

```bash
# Run before every commit
cargo test --package bdp-server --test uniprot_parsing_test
cargo test --package bdp-server --test uniprot_integration_test
```

### CI/CD Pipeline

```yaml
test:
  - name: UniProt Parsing Tests
    run: cargo test --package bdp-server --test uniprot_parsing_test

  - name: UniProt Integration Tests
    run: cargo test --package bdp-server --test uniprot_integration_test
    env:
      DATABASE_URL: postgresql://postgres:postgres@localhost:5432/test_db
```

## Troubleshooting

### Test Failures

1. **Database Connection Errors**:
   - Check `DATABASE_URL` environment variable
   - Ensure PostgreSQL is running: `docker-compose up -d postgres`
   - Verify migrations applied: `sqlx migrate run`

2. **Parsing Failures**:
   - Check UniProt DAT format hasn't changed
   - Validate sample data matches expected format
   - Check for encoding issues (should be UTF-8)

3. **Idempotency Failures**:
   - Clear test data: `psql -c "DELETE FROM ingestion_jobs WHERE job_type LIKE 'uniprot_%';"`
   - Restart tests with clean database

## Next Steps

1. ✅ Implement full FTP download in pipeline
2. ✅ Add metalink MD5 verification
3. ✅ Integrate with worker pool for parallel parsing
4. ✅ Add S3 upload for FASTA/JSON files
5. ⏳ Performance benchmarking (parse 10K proteins)
6. ⏳ Load testing (100K+ proteins)
7. ⏳ Monitoring and alerting setup
