//! Storage integration tests
//!
//! These tests verify the S3/MinIO storage functionality including:
//! - Upload and download operations
//! - Checksum verification
//! - Presigned URL generation
//! - File existence checks
//! - Metadata retrieval
//! - List operations
//! - Delete operations
//! - Copy operations
//!
//! **Requirements**:
//! - MinIO or S3 must be running and accessible
//! - S3_ENDPOINT environment variable must be set (e.g., "http://localhost:9000")
//! - Tests will be skipped if S3_ENDPOINT is not configured
//!
//! **Running tests**:
//! ```bash
//! # With MinIO running via docker-compose
//! cargo test --test storage_tests
//! ```

use bdp_server::storage::{config::StorageConfig, Storage};
use std::time::Duration;

/// Setup helper that creates a Storage instance if MinIO is available
async fn setup_storage() -> Option<Storage> {
    // Check if S3_ENDPOINT is configured
    if std::env::var("S3_ENDPOINT").is_err() {
        return None;
    }

    // Load config from environment
    let config = match StorageConfig::from_env() {
        Ok(cfg) => cfg,
        Err(_) => return None,
    };

    // Create storage instance
    match Storage::new(config).await {
        Ok(storage) => Some(storage),
        Err(e) => {
            eprintln!("Failed to create storage client: {}", e);
            None
        },
    }
}

/// Helper to generate a unique test key
fn test_key(test_name: &str, suffix: &str) -> String {
    format!("test/{}/{}", test_name, suffix)
}

/// Helper to create test data
fn test_data(content: &str) -> Vec<u8> {
    content.as_bytes().to_vec()
}

// ============================================================================
// Upload and Download Tests
// ============================================================================

#[tokio::test]
async fn test_storage_upload_download() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("upload_download", "test.txt");
    let original_data = test_data("Hello, Storage!");

    // Upload the file
    let upload_result = storage
        .upload(&key, original_data.clone(), Some("text/plain".to_string()))
        .await
        .expect("Upload should succeed");

    assert_eq!(upload_result.key, key);
    assert_eq!(upload_result.size, original_data.len() as i64);
    assert!(!upload_result.checksum.is_empty());

    // Download the file
    let downloaded_data = storage
        .download(&key)
        .await
        .expect("Download should succeed");

    // Verify contents match
    assert_eq!(downloaded_data, original_data);

    // Cleanup
    storage.delete(&key).await.ok();
}

#[tokio::test]
async fn test_storage_upload_download_binary() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("upload_download_binary", "data.bin");
    // Create binary data with all byte values
    let original_data: Vec<u8> = (0..=255).collect();

    // Upload binary data
    let upload_result = storage
        .upload(&key, original_data.clone(), Some("application/octet-stream".to_string()))
        .await
        .expect("Upload should succeed");

    assert_eq!(upload_result.size, 256);

    // Download and verify
    let downloaded_data = storage
        .download(&key)
        .await
        .expect("Download should succeed");

    assert_eq!(downloaded_data, original_data);

    // Cleanup
    storage.delete(&key).await.ok();
}

#[tokio::test]
async fn test_storage_upload_large_file() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("upload_large", "large.dat");
    // Create 1MB of data
    let original_data = vec![0x42u8; 1024 * 1024];

    // Upload
    let upload_result = storage
        .upload(&key, original_data.clone(), None)
        .await
        .expect("Upload should succeed");

    assert_eq!(upload_result.size, 1024 * 1024);

    // Download and verify size (not comparing all bytes to save time)
    let downloaded_data = storage
        .download(&key)
        .await
        .expect("Download should succeed");

    assert_eq!(downloaded_data.len(), original_data.len());

    // Cleanup
    storage.delete(&key).await.ok();
}

// ============================================================================
// Checksum Verification Tests
// ============================================================================

#[tokio::test]
async fn test_storage_checksum_verification() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("checksum", "data.txt");
    let data = test_data("Checksum test data");

    // Calculate expected checksum (SHA256)
    let expected_checksum = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&data);
        format!("{:x}", hasher.finalize())
    };

    // Upload and get checksum
    let upload_result = storage
        .upload(&key, data, Some("text/plain".to_string()))
        .await
        .expect("Upload should succeed");

    // Verify checksum matches
    assert_eq!(upload_result.checksum, expected_checksum);
    assert_eq!(upload_result.checksum.len(), 64); // SHA256 is 64 hex chars

    // Cleanup
    storage.delete(&key).await.ok();
}

#[tokio::test]
async fn test_storage_checksum_empty_file() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("checksum_empty", "empty.txt");
    let data = test_data("");

    // SHA256 of empty string
    let expected_checksum = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    // Upload empty file
    let upload_result = storage
        .upload(&key, data, None)
        .await
        .expect("Upload should succeed");

    assert_eq!(upload_result.checksum, expected_checksum);
    assert_eq!(upload_result.size, 0);

    // Cleanup
    storage.delete(&key).await.ok();
}

// ============================================================================
// Presigned URL Tests
// ============================================================================

#[tokio::test]
async fn test_storage_presigned_url() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("presigned", "test.txt");
    let data = test_data("Presigned URL test");

    // Upload a file first
    storage
        .upload(&key, data, Some("text/plain".to_string()))
        .await
        .expect("Upload should succeed");

    // Generate presigned URL
    let presigned_url = storage
        .generate_presigned_url(&key, Duration::from_secs(300))
        .await
        .expect("Presigned URL generation should succeed");

    // Verify URL is valid
    assert!(presigned_url.starts_with("http"));
    assert!(presigned_url.contains(&key));

    // URL should contain query parameters for presigning
    assert!(presigned_url.contains("X-Amz-"));

    // Cleanup
    storage.delete(&key).await.ok();
}

#[tokio::test]
async fn test_storage_presigned_url_different_durations() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("presigned_duration", "test.txt");
    let data = test_data("Duration test");

    // Upload a file
    storage
        .upload(&key, data, None)
        .await
        .expect("Upload should succeed");

    // Generate URLs with different expiration times
    let url_5min = storage
        .generate_presigned_url(&key, Duration::from_secs(300))
        .await
        .expect("Should succeed");

    let url_1hour = storage
        .generate_presigned_url(&key, Duration::from_secs(3600))
        .await
        .expect("Should succeed");

    // Both should be valid URLs
    assert!(url_5min.starts_with("http"));
    assert!(url_1hour.starts_with("http"));

    // URLs should be different (different expiration times)
    assert_ne!(url_5min, url_1hour);

    // Cleanup
    storage.delete(&key).await.ok();
}

// ============================================================================
// Exists Tests
// ============================================================================

#[tokio::test]
async fn test_storage_exists() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("exists", "test.txt");
    let data = test_data("Existence test");

    // File should not exist initially
    let exists_before = storage
        .exists(&key)
        .await
        .expect("Exists check should succeed");
    assert!(!exists_before, "File should not exist before upload");

    // Upload the file
    storage
        .upload(&key, data, None)
        .await
        .expect("Upload should succeed");

    // File should exist now
    let exists_after = storage
        .exists(&key)
        .await
        .expect("Exists check should succeed");
    assert!(exists_after, "File should exist after upload");

    // Cleanup
    storage.delete(&key).await.ok();

    // File should not exist after deletion
    let exists_after_delete = storage
        .exists(&key)
        .await
        .expect("Exists check should succeed");
    assert!(!exists_after_delete, "File should not exist after delete");
}

#[tokio::test]
async fn test_storage_exists_nonexistent() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("exists_nonexistent", "does-not-exist.txt");

    // Check for non-existent file
    let exists = storage
        .exists(&key)
        .await
        .expect("Exists check should succeed even for non-existent files");

    assert!(!exists, "Non-existent file should return false");
}

// ============================================================================
// Metadata Tests
// ============================================================================

#[tokio::test]
async fn test_storage_metadata() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("metadata", "test.txt");
    let data = test_data("Metadata test data");
    let content_type = "text/plain";

    // Upload with content type
    storage
        .upload(&key, data.clone(), Some(content_type.to_string()))
        .await
        .expect("Upload should succeed");

    // Get metadata
    let metadata = storage
        .get_metadata(&key)
        .await
        .expect("Get metadata should succeed");

    // Verify metadata
    assert_eq!(metadata.key, key);
    assert_eq!(metadata.size, data.len() as i64);
    assert_eq!(metadata.content_type.as_deref(), Some(content_type));
    assert!(metadata.last_modified.is_some(), "Last modified should be set");

    // Cleanup
    storage.delete(&key).await.ok();
}

#[tokio::test]
async fn test_storage_metadata_no_content_type() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("metadata_no_ct", "test.bin");
    let data = test_data("No content type");

    // Upload without content type
    storage
        .upload(&key, data.clone(), None)
        .await
        .expect("Upload should succeed");

    // Get metadata
    let metadata = storage
        .get_metadata(&key)
        .await
        .expect("Get metadata should succeed");

    // Size should still be correct
    assert_eq!(metadata.size, data.len() as i64);

    // Cleanup
    storage.delete(&key).await.ok();
}

#[tokio::test]
async fn test_storage_metadata_various_content_types() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let test_cases = vec![
        ("json", "application/json", r#"{"key":"value"}"#),
        ("xml", "application/xml", "<root><item/></root>"),
        ("csv", "text/csv", "a,b,c\n1,2,3"),
        ("binary", "application/octet-stream", "binary data"),
    ];

    for (suffix, content_type, content) in test_cases {
        let key = test_key("metadata_ct", &format!("test.{}", suffix));
        let data = test_data(content);

        storage
            .upload(&key, data, Some(content_type.to_string()))
            .await
            .expect("Upload should succeed");

        let metadata = storage
            .get_metadata(&key)
            .await
            .expect("Get metadata should succeed");

        assert_eq!(metadata.content_type.as_deref(), Some(content_type));

        // Cleanup
        storage.delete(&key).await.ok();
    }
}

// ============================================================================
// List Tests
// ============================================================================

#[tokio::test]
async fn test_storage_list() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let prefix = "test/list_test/";
    let files = vec!["file1.txt", "file2.txt", "file3.txt"];

    // Upload multiple files with same prefix
    for file in &files {
        let key = format!("{}{}", prefix, file);
        storage
            .upload(&key, test_data("test"), None)
            .await
            .expect("Upload should succeed");
    }

    // List files with prefix
    let listed_keys = storage
        .list(prefix, None)
        .await
        .expect("List should succeed");

    // Verify all files are listed
    assert_eq!(listed_keys.len(), files.len());
    for file in &files {
        let expected_key = format!("{}{}", prefix, file);
        assert!(
            listed_keys.contains(&expected_key),
            "Listed keys should contain {}",
            expected_key
        );
    }

    // Cleanup
    for key in &listed_keys {
        storage.delete(key).await.ok();
    }
}

#[tokio::test]
async fn test_storage_list_with_max_keys() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let prefix = "test/list_max/";

    // Upload 5 files
    for i in 0..5 {
        let key = format!("{}file{}.txt", prefix, i);
        storage
            .upload(&key, test_data("test"), None)
            .await
            .expect("Upload should succeed");
    }

    // List with max_keys = 3
    let listed_keys = storage
        .list(prefix, Some(3))
        .await
        .expect("List should succeed");

    // Should only return 3 keys
    assert_eq!(listed_keys.len(), 3);

    // Cleanup - list all and delete
    let all_keys = storage
        .list(prefix, None)
        .await
        .expect("List should succeed");
    for key in &all_keys {
        storage.delete(key).await.ok();
    }
}

#[tokio::test]
async fn test_storage_list_empty_prefix() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let prefix = "test/list_empty/";

    // List non-existent prefix
    let listed_keys = storage
        .list(prefix, None)
        .await
        .expect("List should succeed even for empty prefix");

    // Should return empty list
    assert_eq!(listed_keys.len(), 0);
}

#[tokio::test]
async fn test_storage_list_nested_paths() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let base_prefix = "test/list_nested/";
    let keys = vec![
        format!("{}a/file1.txt", base_prefix),
        format!("{}a/file2.txt", base_prefix),
        format!("{}b/file1.txt", base_prefix),
        format!("{}file.txt", base_prefix),
    ];

    // Upload files
    for key in &keys {
        storage
            .upload(key, test_data("test"), None)
            .await
            .expect("Upload should succeed");
    }

    // List all with base prefix
    let all_listed = storage
        .list(&base_prefix, None)
        .await
        .expect("List should succeed");
    assert_eq!(all_listed.len(), 4);

    // List only 'a' subdirectory
    let a_prefix = format!("{}a/", base_prefix);
    let a_listed = storage
        .list(&a_prefix, None)
        .await
        .expect("List should succeed");
    assert_eq!(a_listed.len(), 2);

    // Cleanup
    for key in &all_listed {
        storage.delete(key).await.ok();
    }
}

// ============================================================================
// Delete Tests
// ============================================================================

#[tokio::test]
async fn test_storage_delete() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("delete", "test.txt");
    let data = test_data("Delete test");

    // Upload a file
    storage
        .upload(&key, data, None)
        .await
        .expect("Upload should succeed");

    // Verify it exists
    let exists_before = storage
        .exists(&key)
        .await
        .expect("Exists check should succeed");
    assert!(exists_before);

    // Delete the file
    storage.delete(&key).await.expect("Delete should succeed");

    // Verify it's gone
    let exists_after = storage
        .exists(&key)
        .await
        .expect("Exists check should succeed");
    assert!(!exists_after);
}

#[tokio::test]
async fn test_storage_delete_nonexistent() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("delete_nonexistent", "does-not-exist.txt");

    // Delete non-existent file should succeed (S3 behavior)
    let result = storage.delete(&key).await;
    assert!(result.is_ok(), "Deleting non-existent file should not error");
}

#[tokio::test]
async fn test_storage_delete_multiple() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let prefix = "test/delete_multiple/";
    let keys: Vec<String> = (0..3).map(|i| format!("{}file{}.txt", prefix, i)).collect();

    // Upload multiple files
    for key in &keys {
        storage
            .upload(key, test_data("test"), None)
            .await
            .expect("Upload should succeed");
    }

    // Verify all exist
    for key in &keys {
        let exists = storage
            .exists(key)
            .await
            .expect("Exists check should succeed");
        assert!(exists);
    }

    // Delete all files
    for key in &keys {
        storage.delete(key).await.expect("Delete should succeed");
    }

    // Verify all are gone
    for key in &keys {
        let exists = storage
            .exists(key)
            .await
            .expect("Exists check should succeed");
        assert!(!exists);
    }
}

// ============================================================================
// Copy Tests
// ============================================================================

#[tokio::test]
async fn test_storage_copy() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let source_key = test_key("copy", "source.txt");
    let dest_key = test_key("copy", "dest.txt");
    let data = test_data("Copy test data");

    // Upload source file
    storage
        .upload(&source_key, data.clone(), Some("text/plain".to_string()))
        .await
        .expect("Upload should succeed");

    // Copy to destination
    storage
        .copy(&source_key, &dest_key)
        .await
        .expect("Copy should succeed");

    // Verify both files exist
    let source_exists = storage
        .exists(&source_key)
        .await
        .expect("Exists check should succeed");
    let dest_exists = storage
        .exists(&dest_key)
        .await
        .expect("Exists check should succeed");

    assert!(source_exists, "Source file should still exist");
    assert!(dest_exists, "Destination file should exist");

    // Verify destination content matches source
    let dest_data = storage
        .download(&dest_key)
        .await
        .expect("Download should succeed");
    assert_eq!(dest_data, data);

    // Cleanup
    storage.delete(&source_key).await.ok();
    storage.delete(&dest_key).await.ok();
}

#[tokio::test]
async fn test_storage_copy_preserves_metadata() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let source_key = test_key("copy_metadata", "source.json");
    let dest_key = test_key("copy_metadata", "dest.json");
    let data = test_data(r#"{"test":"data"}"#);
    let content_type = "application/json";

    // Upload source with content type
    storage
        .upload(&source_key, data.clone(), Some(content_type.to_string()))
        .await
        .expect("Upload should succeed");

    // Copy file
    storage
        .copy(&source_key, &dest_key)
        .await
        .expect("Copy should succeed");

    // Get metadata of both files
    let source_metadata = storage
        .get_metadata(&source_key)
        .await
        .expect("Get metadata should succeed");
    let dest_metadata = storage
        .get_metadata(&dest_key)
        .await
        .expect("Get metadata should succeed");

    // Verify metadata is preserved
    assert_eq!(source_metadata.size, dest_metadata.size);
    assert_eq!(source_metadata.content_type, dest_metadata.content_type);

    // Cleanup
    storage.delete(&source_key).await.ok();
    storage.delete(&dest_key).await.ok();
}

#[tokio::test]
async fn test_storage_copy_overwrite() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let source_key = test_key("copy_overwrite", "source.txt");
    let dest_key = test_key("copy_overwrite", "dest.txt");

    // Upload source and destination
    storage
        .upload(&source_key, test_data("source data"), None)
        .await
        .expect("Upload should succeed");
    storage
        .upload(&dest_key, test_data("old destination data"), None)
        .await
        .expect("Upload should succeed");

    // Copy should overwrite destination
    storage
        .copy(&source_key, &dest_key)
        .await
        .expect("Copy should succeed");

    // Verify destination has source content
    let dest_data = storage
        .download(&dest_key)
        .await
        .expect("Download should succeed");
    assert_eq!(dest_data, test_data("source data"));

    // Cleanup
    storage.delete(&source_key).await.ok();
    storage.delete(&dest_key).await.ok();
}

// ============================================================================
// Build Key Tests
// ============================================================================

#[tokio::test]
async fn test_build_key_integration() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = storage.build_key("uniprot", "human-proteome", "2024.01", "data.fasta");
    let data = test_data("FASTA data");

    // Upload using built key
    storage
        .upload(&key, data.clone(), Some("application/x-fasta".to_string()))
        .await
        .expect("Upload should succeed");

    // Verify the key format
    assert_eq!(key, "data-sources/uniprot/human-proteome/2024.01/data.fasta");

    // Verify file exists at that path
    let exists = storage
        .exists(&key)
        .await
        .expect("Exists check should succeed");
    assert!(exists);

    // List files under the org/name/version prefix
    let prefix = "data-sources/uniprot/human-proteome/2024.01/";
    let listed = storage
        .list(prefix, None)
        .await
        .expect("List should succeed");
    assert!(listed.contains(&key));

    // Cleanup
    storage.delete(&key).await.ok();
}

#[tokio::test]
async fn test_build_tool_key_integration() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = storage.build_tool_key("ncbi", "blast", "2.14.0", "blast-linux.tar.gz");
    let data = test_data("Binary tool data");

    // Upload using built key
    storage
        .upload(&key, data.clone(), Some("application/gzip".to_string()))
        .await
        .expect("Upload should succeed");

    // Verify the key format
    assert_eq!(key, "tools/ncbi/blast/2.14.0/blast-linux.tar.gz");

    // Verify file exists
    let exists = storage
        .exists(&key)
        .await
        .expect("Exists check should succeed");
    assert!(exists);

    // Cleanup
    storage.delete(&key).await.ok();
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_storage_download_nonexistent() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("download_nonexistent", "does-not-exist.txt");

    // Download non-existent file should fail
    let result = storage.download(&key).await;
    assert!(result.is_err(), "Downloading non-existent file should fail");
}

#[tokio::test]
async fn test_storage_metadata_nonexistent() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("metadata_nonexistent", "does-not-exist.txt");

    // Get metadata for non-existent file should fail
    let result = storage.get_metadata(&key).await;
    assert!(result.is_err(), "Getting metadata for non-existent file should fail");
}

#[tokio::test]
async fn test_storage_presigned_url_nonexistent() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("presigned_nonexistent", "does-not-exist.txt");

    // Generate presigned URL for non-existent file should still succeed
    // (URL generation doesn't check if file exists)
    let result = storage
        .generate_presigned_url(&key, Duration::from_secs(300))
        .await;
    assert!(
        result.is_ok(),
        "Presigned URL generation should succeed even for non-existent files"
    );
}

#[tokio::test]
async fn test_storage_copy_nonexistent_source() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let source_key = test_key("copy_nonexistent", "source.txt");
    let dest_key = test_key("copy_nonexistent", "dest.txt");

    // Copy from non-existent source should fail
    let result = storage.copy(&source_key, &dest_key).await;
    assert!(result.is_err(), "Copying from non-existent source should fail");
}

// ============================================================================
// Edge Cases and Special Characters
// ============================================================================

#[tokio::test]
async fn test_storage_key_with_special_characters() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    // Test various special characters in key names
    let keys = vec![
        "test/special/file-with-dashes.txt",
        "test/special/file_with_underscores.txt",
        "test/special/file.with.dots.txt",
        "test/special/file (with spaces).txt",
    ];

    for key in &keys {
        let data = test_data("special character test");

        // Upload
        storage
            .upload(key, data.clone(), None)
            .await
            .expect(&format!("Upload should succeed for key: {}", key));

        // Verify exists
        let exists = storage
            .exists(key)
            .await
            .expect("Exists check should succeed");
        assert!(exists, "File should exist for key: {}", key);

        // Download
        let downloaded = storage
            .download(key)
            .await
            .expect("Download should succeed");
        assert_eq!(downloaded, data);

        // Cleanup
        storage.delete(key).await.ok();
    }
}

#[tokio::test]
async fn test_storage_unicode_content() {
    let Some(storage) = setup_storage().await else {
        println!("Skipping test: S3_ENDPOINT not configured");
        return;
    };

    let key = test_key("unicode", "test.txt");
    let unicode_data = test_data("Hello 世界! 🚀 Здравствуй мир!");

    // Upload unicode content
    storage
        .upload(&key, unicode_data.clone(), Some("text/plain; charset=utf-8".to_string()))
        .await
        .expect("Upload should succeed");

    // Download and verify
    let downloaded = storage
        .download(&key)
        .await
        .expect("Download should succeed");
    assert_eq!(downloaded, unicode_data);

    // Cleanup
    storage.delete(&key).await.ok();
}
