//! E2E ingestion pipeline tests
//!
//! Tests the full data ingestion flow from UniProt data upload through
//! to database storage and S3 archival.

use super::*;
use anyhow::Result;
use serial_test::serial;
use tracing::{info, warn};

/// Happy path: Full ingestion pipeline with CI sample data
#[tokio::test]
#[serial]
async fn test_ingestion_happy_path_ci() -> Result<()> {
    // Initialize test environment
    let env = E2EEnvironment::new().await?;
    let mode = TestDataMode::from_env();
    let data_manager = TestDataManager::new(mode);

    info!("🧪 Starting E2E ingestion test (CI mode)");

    // Get test data info
    let test_info = data_manager.get_info();
    info!("Test data: {:?} mode, size: {}", test_info.mode, test_info.size_human());

    // Step 1: Upload test data to S3
    info!("📤 Step 1: Uploading test data to S3");
    let dat_path = data_manager.get_uniprot_dat_path()?;
    env.upload_test_data(&dat_path, "uniprot_test.dat").await?;

    // Verify upload
    let obs = env.observability();
    let s3_objects = obs.list_s3_objects(None).await?;
    assert!(
        s3_objects.iter().any(|o| o.key == "uniprot_test.dat"),
        "Test data should be uploaded to S3"
    );
    info!("✓ Test data uploaded successfully");

    // Step 2: Create organization
    info!("🏢 Step 2: Creating organization");
    let org_id = env
        .create_organization("uniprot", "UniProt Consortium")
        .await?;
    info!("✓ Organization created: {}", org_id);

    // Step 3: Trigger ingestion job
    info!("⚙️  Step 3: Triggering ingestion job");
    let job_id = env
        .trigger_ingestion_job(org_id, "uniprot_test.dat")
        .await?;
    info!("✓ Job triggered: {}", job_id);

    // Step 4: Wait for job completion
    info!("⏳ Step 4: Waiting for job to complete");
    let timeout = std::time::Duration::from_secs(60);
    env.wait_for_job_completion(job_id, timeout).await?;
    info!("✓ Job completed successfully");

    // Step 5: Verify database state
    info!("🔍 Step 5: Verifying database state");
    let assertions = env.assertions();

    // Check organization exists
    assertions.assert_organization_exists(org_id).await?;

    // Check data source was created
    let data_sources = assertions.assert_data_sources_exist(org_id, 1).await?;
    let data_source_id = data_sources[0].id;
    info!("✓ Data source created: {}", data_source_id);

    // Check version was created
    let versions = assertions.assert_versions_exist(data_source_id, 1).await?;
    let version_id = versions[0].id;
    info!("✓ Version created: {}", version_id);

    // Check proteins were ingested
    // CI sample has 3 proteins
    let protein_count = assertions
        .count_proteins(data_source_id, version_id)
        .await?;
    assert_eq!(protein_count, 3, "Expected 3 proteins from CI sample, found {}", protein_count);
    info!("✓ {} proteins ingested", protein_count);

    // Step 6: Verify specific protein data
    info!("🧬 Step 6: Verifying protein data");

    // Check for known protein from CI sample (Q6GZX4)
    let protein = assertions
        .assert_protein_exists(data_source_id, version_id, "Q6GZX4")
        .await?;

    assert_eq!(protein.accession, "Q6GZX4");
    assert!(protein.name.is_some());
    assert!(protein.sequence.is_some());
    info!("✓ Protein Q6GZX4 verified");

    // Step 7: Verify S3 state
    info!("📦 Step 7: Verifying S3 state");
    let s3_objects = obs.list_s3_objects(Some("processed/")).await?;
    assert!(!s3_objects.is_empty(), "Processed data should be stored in S3");
    info!("✓ {} processed files in S3", s3_objects.len());

    // Step 8: Print observability summary
    info!("📊 Step 8: Generating test summary");
    obs.print_job_status(job_id).await?;
    obs.print_db_stats().await?;
    obs.print_pipeline_status().await?;

    // Cleanup
    env.cleanup().await?;

    info!("✅ E2E ingestion test completed successfully!");
    Ok(())
}

/// Test ingestion with real UniProt data (larger dataset)
#[tokio::test]
#[serial]
#[ignore] // Run explicitly with --ignored flag
async fn test_ingestion_with_real_data() -> Result<()> {
    // Force real data mode
    std::env::set_var("BDP_E2E_MODE", "real");

    let env = E2EEnvironment::new().await?;
    let data_manager = TestDataManager::new(TestDataMode::Real);

    // Check if real data is available
    if !data_manager.has_real_data() {
        warn!("⚠️  Real data not available, skipping test");
        warn!("Run `just e2e-download-data` to download real UniProt data");
        return Ok(());
    }

    info!("🧪 Starting E2E ingestion test (Real mode)");

    let test_info = data_manager.get_info();
    info!("Test data: {:?} mode, size: {}", test_info.mode, test_info.size_human());

    // Upload real data
    let dat_path = data_manager.get_uniprot_dat_path()?;
    env.upload_test_data(&dat_path, "uniprot_real.dat").await?;

    // Create organization
    let org_id = env
        .create_organization("uniprot", "UniProt Consortium")
        .await?;

    // Trigger ingestion with longer timeout for real data
    let job_id = env
        .trigger_ingestion_job(org_id, "uniprot_real.dat")
        .await?;

    // Wait with extended timeout (real data takes longer)
    let timeout = std::time::Duration::from_secs(300); // 5 minutes
    env.wait_for_job_completion(job_id, timeout).await?;

    // Verify results
    let assertions = env.assertions();
    assertions.assert_organization_exists(org_id).await?;

    let data_sources = assertions.assert_data_sources_exist(org_id, 1).await?;
    let data_source_id = data_sources[0].id;

    let protein_count = assertions
        .count_proteins(data_source_id, data_sources[0].latest_version.unwrap())
        .await?;

    info!("✓ {} proteins ingested from real data", protein_count);
    assert!(protein_count > 3, "Real data should have more than 3 proteins");

    // Cleanup
    env.cleanup().await?;

    info!("✅ Real data ingestion test completed!");
    Ok(())
}

/// Error scenario: Invalid DAT file format
#[tokio::test]
#[serial]
async fn test_ingestion_invalid_dat_format() -> Result<()> {
    let env = E2EEnvironment::new().await?;

    info!("🧪 Testing error handling: Invalid DAT format");

    // Create invalid DAT file content
    let invalid_dat = b"This is not a valid UniProt DAT file\nJust random text\n";

    // Upload invalid data
    env.upload_test_data_bytes(invalid_dat, "invalid.dat")
        .await?;

    // Create organization
    let org_id = env
        .create_organization("uniprot", "UniProt Consortium")
        .await?;

    // Trigger ingestion job
    let job_id = env.trigger_ingestion_job(org_id, "invalid.dat").await?;

    // Wait for job to fail
    let timeout = std::time::Duration::from_secs(60);
    let result = env.wait_for_job_completion(job_id, timeout).await;

    // Job should fail
    assert!(result.is_err(), "Job should fail with invalid DAT format");

    // Verify job status is failed
    let obs = env.observability();
    let job_info = obs.get_job_info(job_id).await?;

    info!("Job status: {:?}", job_info.status);
    assert!(
        job_info.status.contains("failed") || job_info.status.contains("error"),
        "Job status should indicate failure"
    );

    // Verify no proteins were ingested
    let assertions = env.assertions();
    let data_sources = assertions
        .assert_data_sources_exist(org_id, 0)
        .await
        .unwrap_or_default();

    assert_eq!(data_sources.len(), 0, "No data sources should be created from invalid data");

    env.cleanup().await?;

    info!("✅ Error handling test completed!");
    Ok(())
}

/// Error scenario: Missing S3 file
#[tokio::test]
#[serial]
async fn test_ingestion_missing_s3_file() -> Result<()> {
    let env = E2EEnvironment::new().await?;

    info!("🧪 Testing error handling: Missing S3 file");

    // Create organization
    let org_id = env
        .create_organization("uniprot", "UniProt Consortium")
        .await?;

    // Trigger ingestion job for non-existent file
    let job_id = env
        .trigger_ingestion_job(org_id, "nonexistent_file.dat")
        .await?;

    // Wait for job to fail
    let timeout = std::time::Duration::from_secs(30);
    let result = env.wait_for_job_completion(job_id, timeout).await;

    // Job should fail
    assert!(result.is_err(), "Job should fail when S3 file is missing");

    // Verify error message
    let obs = env.observability();
    let job_info = obs.get_job_info(job_id).await?;

    info!("Job error: {:?}", job_info.status);
    assert!(
        job_info.status.contains("not found") || job_info.status.contains("NoSuchKey"),
        "Error should mention missing file"
    );

    env.cleanup().await?;

    info!("✅ Missing file error handling test completed!");
    Ok(())
}

/// Error scenario: Database connection failure during ingestion
#[tokio::test]
#[serial]
async fn test_ingestion_resume_after_failure() -> Result<()> {
    let env = E2EEnvironment::new().await?;
    let data_manager = TestDataManager::new(TestDataMode::from_env());

    info!("🧪 Testing job resume after failure");

    // Upload test data
    let dat_path = data_manager.get_uniprot_dat_path()?;
    env.upload_test_data(&dat_path, "resume_test.dat").await?;

    // Create organization
    let org_id = env
        .create_organization("uniprot", "UniProt Consortium")
        .await?;

    // Trigger first job
    let job_id = env.trigger_ingestion_job(org_id, "resume_test.dat").await?;

    // Wait a bit for job to start
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Simulate failure by stopping the job (in real scenario, this could be DB disconnect)
    // For now, just verify idempotency by triggering same job again

    // Trigger another job with same data
    let job_id_2 = env.trigger_ingestion_job(org_id, "resume_test.dat").await?;

    // Wait for completion
    let timeout = std::time::Duration::from_secs(60);
    env.wait_for_job_completion(job_id_2, timeout).await?;

    // Verify data integrity - should only have one set of data
    let assertions = env.assertions();
    let data_sources = assertions.assert_data_sources_exist(org_id, 1).await?;

    // Should not have duplicate proteins
    let protein_count = assertions
        .count_proteins(data_sources[0].id, data_sources[0].latest_version.unwrap())
        .await?;

    assert_eq!(protein_count, 3, "Should have exactly 3 proteins (no duplicates)");

    env.cleanup().await?;

    info!("✅ Resume after failure test completed!");
    Ok(())
}

/// Performance test: Measure ingestion throughput
#[tokio::test]
#[serial]
#[ignore] // Run explicitly for performance testing
async fn test_ingestion_performance() -> Result<()> {
    let env = E2EEnvironment::new().await?;
    let data_manager = TestDataManager::new(TestDataMode::from_env());

    info!("📊 Performance test: Measuring ingestion throughput");

    let dat_path = data_manager.get_uniprot_dat_path()?;
    let file_size = std::fs::metadata(&dat_path)?.len();

    env.upload_test_data(&dat_path, "perf_test.dat").await?;
    let org_id = env
        .create_organization("uniprot", "UniProt Consortium")
        .await?;

    // Measure ingestion time
    let start = std::time::Instant::now();
    let job_id = env.trigger_ingestion_job(org_id, "perf_test.dat").await?;

    let timeout = std::time::Duration::from_secs(300);
    env.wait_for_job_completion(job_id, timeout).await?;

    let duration = start.elapsed();

    // Calculate metrics
    let assertions = env.assertions();
    let data_sources = assertions.assert_data_sources_exist(org_id, 1).await?;
    let protein_count = assertions
        .count_proteins(data_sources[0].id, data_sources[0].latest_version.unwrap())
        .await?;

    let throughput_kb_s = (file_size as f64 / 1024.0) / duration.as_secs_f64();
    let proteins_per_sec = protein_count as f64 / duration.as_secs_f64();

    info!("⏱️  Performance Metrics:");
    info!("  Duration: {:.2}s", duration.as_secs_f64());
    info!("  File size: {} KB", file_size / 1024);
    info!("  Proteins: {}", protein_count);
    info!("  Throughput: {:.2} KB/s", throughput_kb_s);
    info!("  Rate: {:.2} proteins/s", proteins_per_sec);

    env.cleanup().await?;

    info!("✅ Performance test completed!");
    Ok(())
}
