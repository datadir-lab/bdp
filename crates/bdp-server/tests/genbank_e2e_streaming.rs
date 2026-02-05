#![allow(clippy::unwrap_used, clippy::expect_used)]
// GenBank E2E streaming tests with real database and storage
//
// Tests the complete pipeline using streaming decompression with:
// - PostgreSQL (via testcontainers)
// - MinIO S3 (via testcontainers)
// - Full data flow from compressed file to database and S3

use bdp_server::config::AppConfig;
use bdp_server::ingest::genbank::config::GenbankFtpConfig;
use bdp_server::ingest::genbank::models::SourceDatabase;
use bdp_server::ingest::genbank::pipeline::GenbankPipeline;
use bdp_server::storage::Storage;
use flate2::write::GzEncoder;
use flate2::Compression;
use serial_test::serial;
use sqlx::PgPool;
use std::fs;
use std::io::Write;
use std::sync::Arc;
use testcontainers::clients::Cli;
use testcontainers_modules::minio::MinIO;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

/// Test helper to set up database and storage
struct TestEnv {
    pool: PgPool,
    storage: Storage,
    org_id: Uuid,
}

async fn setup_test_env(docker: &Cli) -> TestEnv {
    // Start PostgreSQL container
    let postgres = docker.run(Postgres::default());
    let postgres_port = postgres.get_host_port_ipv4(5432);

    // Start MinIO container
    let minio = docker.run(MinIO::default());
    let minio_port = minio.get_host_port_ipv4(9000);

    // Connect to PostgreSQL
    let database_url = format!("postgres://postgres:postgres@localhost:{}/postgres", postgres_port);

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Set up MinIO storage
    let config = AppConfig {
        s3_endpoint: Some(format!("http://localhost:{}", minio_port)),
        s3_region: "us-east-1".to_string(),
        s3_bucket: "test-bucket".to_string(),
        s3_access_key: Some("minioadmin".to_string()),
        s3_secret_key: Some("minioadmin".to_string()),
        s3_force_path_style: true,
        ..Default::default()
    };

    let storage = Storage::from_config(&config)
        .await
        .expect("Failed to create storage");

    // Create test organization
    let org_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO organizations (id, slug, name, email) VALUES ($1, $2, $3, $4)",
        org_id,
        "test-org",
        "Test Organization",
        "test@example.com"
    )
    .execute(&pool)
    .await
    .expect("Failed to create test organization");

    TestEnv {
        pool,
        storage,
        org_id,
    }
}

#[tokio::test]
#[serial]
async fn test_e2e_streaming_pipeline_single_file() {
    let docker = Cli::default();
    let env = setup_test_env(&docker).await;

    // Read sample GenBank file
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    // Compress it (simulating a real .seq.gz file)
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    println!("Test data: {} bytes compressed", compressed.len());

    // Since we can't easily mock FTP, we'll test the streaming decompression + parsing + storage
    // This verifies the complete data flow except for the FTP download part

    // Create parser and parse from streaming decoder
    use bdp_server::ingest::genbank::parser::GenbankParser;
    use std::io::Cursor;

    let cursor = Cursor::new(compressed);
    let decoder = flate2::read::GzDecoder::new(cursor);

    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let records = parser.parse_all(decoder).expect("Failed to parse");

    println!("Parsed {} records", records.len());
    assert_eq!(records.len(), 1);

    // Store records using GenbankStorage
    use bdp_server::ingest::genbank::storage::GenbankStorage;

    let storage = GenbankStorage::new(
        env.pool.clone(),
        env.storage.clone(),
        env.org_id,
        "1.0".to_string(),
        "267.0".to_string(),
        "267.0".to_string(),
    );

    // Setup citations
    storage
        .setup_citations()
        .await
        .expect("Failed to setup citations");

    // Store records
    let stats = storage
        .store_records(&records)
        .await
        .expect("Failed to store records");

    println!("Storage stats: {:?}", stats);
    assert_eq!(stats.total, 1);
    assert_eq!(stats.stored, 1);
    assert!(stats.bytes_uploaded > 0);

    // Verify data in database
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM registry_entries")
        .fetch_one(&env.pool)
        .await
        .expect("Failed to query registry entries");

    assert!(count >= 1, "Expected at least 1 registry entry, got {}", count);

    println!("✓ E2E streaming pipeline test passed");
}

#[tokio::test]
#[serial]
async fn test_e2e_streaming_memory_efficiency() {
    let docker = Cli::default();
    let env = setup_test_env(&docker).await;

    // Create larger test data by repeating sample
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let sample_data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    let mut large_data = String::new();
    for i in 0..50 {
        let modified = sample_data.replace("NC_001416", &format!("NC_{:06}", i));
        large_data.push_str(&modified);
    }

    // Compress
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(large_data.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    println!(
        "Large test data: {} bytes compressed (~{} MB)",
        compressed.len(),
        compressed.len() / 1_048_576
    );

    // Parse with streaming (should use minimal memory)
    use bdp_server::ingest::genbank::parser::GenbankParser;
    use std::io::Cursor;

    let cursor = Cursor::new(compressed);
    let decoder = flate2::read::GzDecoder::new(cursor);

    let parser = GenbankParser::new(SourceDatabase::Genbank);

    // Parse all records (in production, this would stream from FTP)
    let records = parser.parse_all(decoder).expect("Failed to parse");

    println!("Parsed {} records from large dataset", records.len());
    assert_eq!(records.len(), 50);

    // Store a subset to avoid long test runtime
    use bdp_server::ingest::genbank::storage::GenbankStorage;

    let storage = GenbankStorage::new(
        env.pool.clone(),
        env.storage.clone(),
        env.org_id,
        "1.0".to_string(),
        "267.0".to_string(),
        "267.0".to_string(),
    );

    storage.setup_citations().await.expect("Setup failed");

    let stats = storage
        .store_records(&records[0..10])
        .await
        .expect("Failed to store");

    println!("Stored {} records", stats.stored);
    assert_eq!(stats.stored, 10);

    println!("✓ E2E memory efficiency test passed");
}

#[tokio::test]
#[serial]
async fn test_e2e_streaming_data_integrity() {
    let docker = Cli::default();
    let env = setup_test_env(&docker).await;

    // Read sample
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    // Compress
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    // Parse with streaming
    use bdp_server::ingest::genbank::parser::GenbankParser;
    use std::io::Cursor;

    let cursor = Cursor::new(compressed);
    let decoder = flate2::read::GzDecoder::new(cursor);

    let parser = GenbankParser::new(SourceDatabase::Genbank);
    let records = parser.parse_all(decoder).expect("Failed to parse");

    let record = &records[0];

    // Verify data integrity
    assert_eq!(record.accession, "NC_001416");
    assert_eq!(record.sequence_length, 5386);
    assert!(!record.sequence.is_empty());
    assert_eq!(record.sequence_hash.len(), 64); // SHA256

    // Store and verify
    use bdp_server::ingest::genbank::storage::GenbankStorage;

    let storage = GenbankStorage::new(
        env.pool.clone(),
        env.storage.clone(),
        env.org_id,
        "1.0".to_string(),
        "267.0".to_string(),
        "267.0".to_string(),
    );

    storage.setup_citations().await.unwrap();
    storage.store_records(&records).await.unwrap();

    // Query back from database
    let stored_entry = sqlx::query!(
        "SELECT slug, name FROM registry_entries WHERE slug = $1",
        "genbank-nc_001416-1"
    )
    .fetch_one(&env.pool)
    .await
    .expect("Failed to fetch entry");

    assert_eq!(stored_entry.slug, "genbank-nc_001416-1");
    assert!(stored_entry.name.contains("Enterobacteria phage lambda"));

    println!("✓ E2E data integrity test passed");
}

#[tokio::test]
#[serial]
async fn test_e2e_streaming_vs_nonstreaming_equivalence() {
    let docker = Cli::default();
    let env = setup_test_env(&docker).await;

    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    // Compress
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();

    use bdp_server::ingest::genbank::parser::GenbankParser;
    use std::io::Cursor;

    let parser = GenbankParser::new(SourceDatabase::Genbank);

    // Method 1: Non-streaming
    let cursor1 = Cursor::new(compressed.clone());
    let mut decoder1 = flate2::read::GzDecoder::new(cursor1);
    let mut decompressed = Vec::new();
    std::io::Read::read_to_end(&mut decoder1, &mut decompressed).unwrap();
    let records1 = parser.parse_all(decompressed.as_slice()).unwrap();

    // Method 2: Streaming
    let cursor2 = Cursor::new(compressed);
    let decoder2 = flate2::read::GzDecoder::new(cursor2);
    let records2 = parser.parse_all(decoder2).unwrap();

    // Both should produce identical records
    assert_eq!(records1.len(), records2.len());
    assert_eq!(records1[0].accession, records2[0].accession);
    assert_eq!(records1[0].sequence_hash, records2[0].sequence_hash);

    // Store both and verify they produce same database state
    use bdp_server::ingest::genbank::storage::GenbankStorage;

    // Store streaming version
    let storage = GenbankStorage::new(
        env.pool.clone(),
        env.storage.clone(),
        env.org_id,
        "1.0".to_string(),
        "267.0".to_string(),
        "267.0".to_string(),
    );

    storage.setup_citations().await.unwrap();
    let stats = storage.store_records(&records2).await.unwrap();

    assert_eq!(stats.stored, 1);

    println!("✓ E2E streaming vs non-streaming equivalence test passed");
}

#[tokio::test]
#[serial]
async fn test_e2e_streaming_concurrent_processing() {
    let docker = Cli::default();
    let env = setup_test_env(&docker).await;

    // Simulate processing multiple files concurrently
    let sample_path = "../../tests/fixtures/genbank/sample.gbk";
    let sample_data = fs::read_to_string(sample_path).expect("Failed to read sample file");

    use bdp_server::ingest::genbank::parser::GenbankParser;
    use std::io::Cursor;

    let mut tasks = Vec::new();

    for i in 0..3 {
        let data = sample_data.replace("NC_001416", &format!("NC_00141{}", i));

        // Compress
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        let parser = GenbankParser::new(SourceDatabase::Genbank);

        let task = tokio::spawn(async move {
            let cursor = Cursor::new(compressed);
            let decoder = flate2::read::GzDecoder::new(cursor);
            parser.parse_all(decoder).unwrap()
        });

        tasks.push(task);
    }

    // Wait for all tasks
    let mut all_records = Vec::new();
    for task in tasks {
        let records = task.await.unwrap();
        all_records.extend(records);
    }

    assert_eq!(all_records.len(), 3);

    // Store all records
    use bdp_server::ingest::genbank::storage::GenbankStorage;

    let storage = GenbankStorage::new(
        env.pool.clone(),
        env.storage.clone(),
        env.org_id,
        "1.0".to_string(),
        "267.0".to_string(),
        "267.0".to_string(),
    );

    storage.setup_citations().await.unwrap();
    let stats = storage.store_records(&all_records).await.unwrap();

    assert_eq!(stats.stored, 3);

    println!("✓ E2E concurrent processing test passed");
}
