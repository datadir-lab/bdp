//! `bdp init` command implementation
//!
//! Initializes a new BDP project with audit logging.

use crate::audit::{execute_with_audit, get_machine_id, AuditLogger, LocalAuditLogger};
use crate::audit::types::EventType;
use crate::error::{CliError, Result};
use crate::gitignore;
use crate::manifest::Manifest;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Initialize a new BDP project
pub async fn run(
    path: String,
    name: Option<String>,
    version: String,
    description: Option<String>,
    force: bool,
) -> Result<()> {
    let project_dir = PathBuf::from(&path);

    // Create directory if it doesn't exist
    if !project_dir.exists() {
        fs::create_dir_all(&project_dir)?;
    }

    // Initialize audit logger (create .bdp/bdp.db)
    let bdp_dir = project_dir.join(".bdp");
    fs::create_dir_all(&bdp_dir)?;

    let audit_db_path = bdp_dir.join("bdp.db");
    let machine_id = get_machine_id()?;
    let audit = Arc::new(LocalAuditLogger::new(audit_db_path, machine_id)?) as Arc<dyn AuditLogger>;

    // Execute with audit middleware
    execute_with_audit(
        audit,
        EventType::InitStart,
        EventType::InitSuccess,
        EventType::InitFailure,
        None,
        json!({
            "path": &path,
            "name": &name,
            "version": &version,
            "description": &description,
            "force": force
        }),
        || async {
            run_init_command(
                &project_dir,
                name,
                version,
                description,
                force,
            ).await
        },
    )
    .await
}

/// Internal implementation of init command
async fn run_init_command(
    project_dir: &Path,
    name: Option<String>,
    version: String,
    description: Option<String>,
    force: bool,
) -> Result<()> {
    let manifest_path = project_dir.join("bdp.yml");

    // Check if already initialized
    if manifest_path.exists() && !force {
        return Err(CliError::AlreadyInitialized(
            "bdp.yml already exists. Use --force to overwrite.".to_string(),
        ));
    }

    // Determine project name
    let project_name = name.unwrap_or_else(|| {
        project_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-project")
            .to_string()
    });

    // Create manifest
    let manifest = if let Some(desc) = description {
        Manifest::with_description(project_name.clone(), version, desc)
    } else {
        Manifest::new(project_name.clone(), version)
    };

    // Save manifest
    manifest.save(&manifest_path)?;

    // Create .bdp directory structure
    create_bdp_directories(project_dir)?;

    // Manage .gitignore
    gitignore::update_gitignore(project_dir)?;

    println!("✓ Initialized BDP project: {}", project_name);
    println!("  Created: bdp.yml");
    println!("  Created: .bdp/");
    println!("  Created: .bdp/bdp.db (audit trail)");
    println!("  Updated: .gitignore");
    println!();
    println!("Note: The audit trail (.bdp/bdp.db) is editable and intended");
    println!("      for research documentation, not legal evidence.");

    Ok(())
}

/// Create .bdp directory structure
fn create_bdp_directories(project_dir: &Path) -> Result<()> {
    let bdp_dir = project_dir.join(".bdp");
    fs::create_dir_all(bdp_dir.join("cache"))?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_init_command() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_string_lossy().to_string();

        let result = run(
            path.clone(),
            Some("test-project".to_string()),
            "0.1.0".to_string(),
            Some("Test description".to_string()),
            false,
        )
        .await;

        assert!(result.is_ok());

        // Check files created
        let manifest_path = temp_dir.path().join("bdp.yml");
        assert!(manifest_path.exists());

        let gitignore_path = temp_dir.path().join(".gitignore");
        assert!(gitignore_path.exists());

        let bdp_dir = temp_dir.path().join(".bdp");
        assert!(bdp_dir.exists());

        // Check audit database created
        let audit_db = temp_dir.path().join(".bdp/bdp.db");
        assert!(audit_db.exists());

        // Load and verify manifest
        let manifest = Manifest::load(&manifest_path).unwrap();
        assert_eq!(manifest.project.name, "test-project");
        assert_eq!(manifest.project.version, "0.1.0");
        assert_eq!(
            manifest.project.description,
            Some("Test description".to_string())
        );
    }

    #[tokio::test]
    async fn test_init_already_initialized() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_string_lossy().to_string();

        // First init
        run(
            path.clone(),
            Some("test".to_string()),
            "0.1.0".to_string(),
            None,
            false,
        )
        .await
        .unwrap();

        // Second init without force
        let result = run(path.clone(), Some("test".to_string()), "0.1.0".to_string(), None, false).await;
        assert!(result.is_err());

        // Second init with force
        let result = run(path, Some("test2".to_string()), "0.2.0".to_string(), None, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_init_creates_audit_trail() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_string_lossy().to_string();

        run(
            path.clone(),
            Some("test".to_string()),
            "0.1.0".to_string(),
            None,
            false,
        )
        .await
        .unwrap();

        // Verify audit database exists and has events
        let audit_db_path = temp_dir.path().join(".bdp/bdp.db");
        assert!(audit_db_path.exists());

        let machine_id = get_machine_id().unwrap();
        let audit = LocalAuditLogger::new(audit_db_path, machine_id).unwrap();

        // Verify audit chain integrity
        let is_valid = audit.verify_integrity().await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_init_multiple_workspaces_independent_audit() {
        // Test that different workspaces have independent audit trails
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let path1 = temp_dir1.path().to_string_lossy().to_string();
        let path2 = temp_dir2.path().to_string_lossy().to_string();

        // Initialize two separate projects
        run(
            path1.clone(),
            Some("project1".to_string()),
            "1.0.0".to_string(),
            Some("First project".to_string()),
            false,
        )
        .await
        .unwrap();

        run(
            path2.clone(),
            Some("project2".to_string()),
            "2.0.0".to_string(),
            Some("Second project".to_string()),
            false,
        )
        .await
        .unwrap();

        // Verify both have audit databases
        let audit_db1 = temp_dir1.path().join(".bdp/bdp.db");
        let audit_db2 = temp_dir2.path().join(".bdp/bdp.db");

        assert!(audit_db1.exists());
        assert!(audit_db2.exists());

        // Verify both audit trails are valid
        let machine_id = get_machine_id().unwrap();

        let audit1 = LocalAuditLogger::new(audit_db1, machine_id.clone()).unwrap();
        let audit2 = LocalAuditLogger::new(audit_db2, machine_id).unwrap();

        assert!(audit1.verify_integrity().await.unwrap());
        assert!(audit2.verify_integrity().await.unwrap());

        // Verify manifests are different (not shared)
        let manifest1 = Manifest::load(&temp_dir1.path().join("bdp.yml")).unwrap();
        let manifest2 = Manifest::load(&temp_dir2.path().join("bdp.yml")).unwrap();

        assert_eq!(manifest1.project.name, "project1");
        assert_eq!(manifest2.project.name, "project2");
        assert_ne!(manifest1.project.version, manifest2.project.version);
    }
}
