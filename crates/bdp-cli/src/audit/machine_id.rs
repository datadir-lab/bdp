//! Machine ID generation for audit trail
//!
//! Generates a stable machine identifier without collecting personal information.

use crate::error::{CliError, Result};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// Get or create machine ID
///
/// Machine ID is stored in `.bdp/machine-id` and persists across commands.
/// Format: `{hostname}-{random-suffix}`
pub fn get_machine_id() -> Result<String> {
    let id_file = get_machine_id_path()?;

    // Try to read existing ID
    if id_file.exists() {
        match fs::read_to_string(&id_file) {
            Ok(id) => {
                let trimmed = id.trim().to_string();
                if !trimmed.is_empty() {
                    return Ok(trimmed);
                }
            }
            Err(_) => {
                // Fall through to generate new ID
            }
        }
    }

    // Generate new ID
    let machine_id = generate_machine_id()?;

    // Create parent directory if needed
    if let Some(parent) = id_file.parent() {
        fs::create_dir_all(parent)?;
    }

    // Save for future use
    fs::write(&id_file, &machine_id)?;

    Ok(machine_id)
}

/// Get machine ID file path
fn get_machine_id_path() -> Result<PathBuf> {
    let bdp_dir = PathBuf::from(".bdp");
    Ok(bdp_dir.join("machine-id"))
}

/// Generate a new machine ID
fn generate_machine_id() -> Result<String> {
    // Get hostname
    let hostname = hostname::get()
        .map_err(|e| CliError::Audit(format!("Failed to get hostname: {}", e)))?
        .to_string_lossy()
        .to_string();

    // Add random suffix for uniqueness (privacy - don't use MAC address)
    let suffix = Uuid::new_v4().to_string()[..8].to_string();

    Ok(format!("{}-{}", sanitize_hostname(&hostname), suffix))
}

/// Sanitize hostname for use in machine ID
fn sanitize_hostname(hostname: &str) -> String {
    hostname
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .take(32) // Limit length
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_machine_id() {
        let id1 = generate_machine_id().unwrap();
        let id2 = generate_machine_id().unwrap();

        // Should be non-empty
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());

        // Should contain hyphen
        assert!(id1.contains('-'));
        assert!(id2.contains('-'));

        // Should be different (because of random suffix)
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_sanitize_hostname() {
        assert_eq!(sanitize_hostname("my-host"), "my-host");
        assert_eq!(sanitize_hostname("my_host"), "my_host");
        assert_eq!(sanitize_hostname("my host!"), "myhost");
        assert_eq!(
            sanitize_hostname("very-long-hostname-that-exceeds-the-limit-and-should-be-truncated"),
            "very-long-hostname-that-exceeds-"
        );
    }

    #[test]
    fn test_get_machine_id_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        // Change to temp directory
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // First call generates ID
        let id1 = get_machine_id().unwrap();
        assert!(!id1.is_empty());

        // Second call returns same ID
        let id2 = get_machine_id().unwrap();
        assert_eq!(id1, id2);

        // Verify file was created
        let id_file = temp_dir.path().join(".bdp/machine-id");
        assert!(id_file.exists());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }
}
