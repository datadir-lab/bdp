//! `bdp source` command implementation
//!
//! Manages data sources in the manifest.

use crate::{
    commands::output::Render,
    error::{CliError, Result},
    manifest::{validate_source_spec, Manifest},
};

/// Output from `bdp source` subcommands.
pub enum SourceOutput {
    Added { source: String },
    AlreadyExists { source: String },
    Removed { source: String },
    NotFound { source: String },
    List { sources: Vec<String> },
    Empty,
}

impl Render for SourceOutput {
    fn render(&self) {
        use colored::Colorize;

        match self {
            SourceOutput::Added { source } => {
                println!("{} Added source: {}", "✓".green(), source);
            },
            SourceOutput::AlreadyExists { source } => {
                println!("{} Source already exists: {}", "✓".green(), source);
            },
            SourceOutput::Removed { source } => {
                println!("{} Removed source: {}", "✓".green(), source);
            },
            SourceOutput::NotFound { source } => {
                println!("{} Source not found: {}", "✗".red(), source);
            },
            SourceOutput::List { sources } => {
                println!("Sources in {}:", "bdp.yml".cyan());
                for source in sources {
                    println!("  • {}", source);
                }
                println!("\nTotal: {} source(s)", sources.len());
            },
            SourceOutput::Empty => {
                println!("No sources defined in bdp.yml");
            },
        }
    }
}

/// Add a source to the manifest
pub async fn add(source: String) -> Result<SourceOutput> {
    // Validate source specification
    validate_source_spec(&source)?;

    // Load manifest
    let mut manifest = Manifest::load("bdp.yml").map_err(|_| {
        CliError::NotInitialized(
            "No bdp.yml found. Run 'bdp init' to create a project first.".to_string(),
        )
    })?;

    // Check if already exists
    if manifest.has_source(&source) {
        return Ok(SourceOutput::AlreadyExists { source });
    }

    // Add source
    manifest.add_source(source.clone());

    // Save manifest
    manifest.save("bdp.yml")?;

    Ok(SourceOutput::Added { source })
}

/// Remove a source from the manifest
pub async fn remove(source: String) -> Result<SourceOutput> {
    // Load manifest
    let mut manifest = Manifest::load("bdp.yml").map_err(|_| {
        CliError::NotInitialized(
            "No bdp.yml found. Run 'bdp init' to create a project first.".to_string(),
        )
    })?;

    // Remove source
    if manifest.remove_source(&source) {
        manifest.save("bdp.yml")?;
        Ok(SourceOutput::Removed { source })
    } else {
        Ok(SourceOutput::NotFound { source })
    }
}

/// List all sources in the manifest
pub async fn list() -> Result<SourceOutput> {
    // Load manifest
    let manifest = Manifest::load("bdp.yml").map_err(|_| {
        CliError::NotInitialized(
            "No bdp.yml found. Run 'bdp init' to create a project first.".to_string(),
        )
    })?;

    if manifest.sources.is_empty() {
        return Ok(SourceOutput::Empty);
    }

    Ok(SourceOutput::List {
        sources: manifest.sources.clone(),
    })
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
