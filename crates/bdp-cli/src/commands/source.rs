//! `bdp source` command implementation
//!
//! Manages data sources in the manifest.

use crate::error::{CliError, Result};
use crate::manifest::{validate_source_spec, Manifest};
use colored::Colorize;

/// Add a source to the manifest
pub async fn add(source: String) -> Result<()> {
    // Validate source specification
    validate_source_spec(&source)?;

    // Load manifest
    let mut manifest = Manifest::load("bdp.yml")
        .map_err(|_| CliError::NotInitialized("Run 'bdp init' first".to_string()))?;

    // Check if already exists
    if manifest.has_source(&source) {
        println!("{} Source already exists: {}", "✓".green(), source);
        return Ok(());
    }

    // Add source
    manifest.add_source(source.clone());

    // Save manifest
    manifest.save("bdp.yml")?;

    println!("{} Added source: {}", "✓".green(), source);

    Ok(())
}

/// Remove a source from the manifest
pub async fn remove(source: String) -> Result<()> {
    // Load manifest
    let mut manifest = Manifest::load("bdp.yml")
        .map_err(|_| CliError::NotInitialized("Run 'bdp init' first".to_string()))?;

    // Remove source
    if manifest.remove_source(&source) {
        manifest.save("bdp.yml")?;
        println!("{} Removed source: {}", "✓".green(), source);
    } else {
        println!("{} Source not found: {}", "✗".red(), source);
    }

    Ok(())
}

/// List all sources in the manifest
pub async fn list() -> Result<()> {
    // Load manifest
    let manifest = Manifest::load("bdp.yml")
        .map_err(|_| CliError::NotInitialized("Run 'bdp init' first".to_string()))?;

    if manifest.sources.is_empty() {
        println!("No sources defined in bdp.yml");
        return Ok(());
    }

    println!("Sources in {}:", "bdp.yml".cyan());
    for source in &manifest.sources {
        println!("  • {}", source);
    }

    println!("\nTotal: {} source(s)", manifest.sources.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest;

    /// Test source validation logic
    #[tokio::test]
    async fn test_validate_source_spec() {
        // Valid specs
        assert!(validate_source_spec("uniprot:P01308-fasta@1.0").is_ok());
        assert!(validate_source_spec("ncbi:blast@2.14.0").is_ok());

        // Invalid specs
        assert!(validate_source_spec("invalid").is_err());
        assert!(validate_source_spec("missing:version").is_err());
    }

    /// Test manifest source operations
    #[tokio::test]
    async fn test_manifest_source_operations() {
        let mut manifest = Manifest::new("test".to_string(), "0.1.0".to_string());

        // Add source
        manifest.add_source("uniprot:P01308-fasta@1.0".to_string());
        assert!(manifest.has_source("uniprot:P01308-fasta@1.0"));

        // Remove source
        assert!(manifest.remove_source("uniprot:P01308-fasta@1.0"));
        assert!(!manifest.has_source("uniprot:P01308-fasta@1.0"));

        // Remove non-existent source
        assert!(!manifest.remove_source("nonexistent:source@1.0"));
    }

    // Note: Full command integration tests that change directories should be
    // run as integration tests in tests/ directory to avoid interfering with
    // parallel test execution.
}
