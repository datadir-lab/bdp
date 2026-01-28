//! Manifest file handling (bdp.yml)
//!
//! The manifest defines project metadata and data source dependencies.

use crate::error::{CliError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// BDP manifest file (bdp.yml)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    /// Project metadata
    pub project: ProjectMetadata,

    /// Data source dependencies (e.g., "uniprot:P01308-fasta@1.0")
    #[serde(default)]
    pub sources: Vec<String>,

    /// Tool dependencies (e.g., "ncbi:blast@2.14.0")
    #[serde(default)]
    pub tools: Vec<String>,
}

/// Project metadata section
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectMetadata {
    /// Project name
    pub name: String,

    /// Project version
    pub version: String,

    /// Optional project description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Manifest {
    /// Create a new manifest with the given project metadata
    pub fn new(name: String, version: String) -> Self {
        Self {
            project: ProjectMetadata {
                name,
                version,
                description: None,
            },
            sources: Vec::new(),
            tools: Vec::new(),
        }
    }

    /// Create a new manifest with description
    pub fn with_description(name: String, version: String, description: String) -> Self {
        Self {
            project: ProjectMetadata {
                name,
                version,
                description: Some(description),
            },
            sources: Vec::new(),
            tools: Vec::new(),
        }
    }

    /// Load manifest from a file
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(CliError::FileNotFound(path.display().to_string()));
        }

        let content = std::fs::read_to_string(path)?;
        let manifest: Manifest = serde_yaml::from_str(&content)
            .map_err(|e| CliError::invalid_manifest(format!("Failed to parse YAML: {}", e)))?;

        Ok(manifest)
    }

    /// Save manifest to a file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Add a source to the manifest
    pub fn add_source(&mut self, source: String) {
        if !self.sources.contains(&source) {
            self.sources.push(source);
        }
    }

    /// Remove a source from the manifest
    ///
    /// Returns true if the source was found and removed, false otherwise
    pub fn remove_source(&mut self, source: &str) -> bool {
        if let Some(pos) = self.sources.iter().position(|s| s == source) {
            self.sources.remove(pos);
            true
        } else {
            false
        }
    }

    /// Check if a source exists in the manifest
    pub fn has_source(&self, source: &str) -> bool {
        self.sources.iter().any(|s| s == source)
    }

    /// Add a tool to the manifest
    pub fn add_tool(&mut self, tool: String) {
        if !self.tools.contains(&tool) {
            self.tools.push(tool);
        }
    }

    /// Remove a tool from the manifest
    ///
    /// Returns true if the tool was found and removed, false otherwise
    pub fn remove_tool(&mut self, tool: &str) -> bool {
        if let Some(pos) = self.tools.iter().position(|t| t == tool) {
            self.tools.remove(pos);
            true
        } else {
            false
        }
    }

    /// Check if a tool exists in the manifest
    pub fn has_tool(&self, tool: &str) -> bool {
        self.tools.iter().any(|t| t == tool)
    }

    /// Validate the manifest structure
    pub fn validate(&self) -> Result<()> {
        if self.project.name.is_empty() {
            return Err(CliError::invalid_manifest("Project name cannot be empty"));
        }

        if self.project.version.is_empty() {
            return Err(CliError::invalid_manifest(
                "Project version cannot be empty",
            ));
        }

        // Validate source specifications
        for source in &self.sources {
            validate_source_spec(source)?;
        }

        // Validate tool specifications
        for tool in &self.tools {
            validate_source_spec(tool)?; // Tools use same format
        }

        Ok(())
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new("my-project".to_string(), "0.1.0".to_string())
    }
}

/// Validate a source specification format
///
/// Valid format: "registry:identifier-format@version"
/// Examples:
/// - "uniprot:P01308-fasta@1.0"
/// - "ncbi:blast@2.14.0"
/// - "ensembl:homo_sapiens-gtf@110"
pub fn validate_source_spec(spec: &str) -> Result<()> {
    // Check for basic structure: registry:identifier@version
    let parts: Vec<&str> = spec.split(':').collect();
    if parts.len() != 2 {
        return Err(CliError::invalid_source_spec(format!(
            "Expected format 'registry:identifier@version' or 'registry:identifier-format@version', got '{}'",
            spec
        )));
    }

    let registry = parts[0];
    if registry.is_empty() {
        return Err(CliError::invalid_source_spec("Registry cannot be empty"));
    }

    let identifier_version = parts[1];
    if !identifier_version.contains('@') {
        return Err(CliError::invalid_source_spec(format!(
            "Missing '@' separator in '{}', expected 'identifier@version'",
            identifier_version
        )));
    }

    let version_parts: Vec<&str> = identifier_version.split('@').collect();
    if version_parts.len() != 2 {
        return Err(CliError::invalid_source_spec(format!(
            "Invalid version format in '{}'",
            identifier_version
        )));
    }

    let identifier = version_parts[0];
    let version = version_parts[1];

    if identifier.is_empty() {
        return Err(CliError::invalid_source_spec("Identifier cannot be empty"));
    }

    if version.is_empty() {
        return Err(CliError::invalid_source_spec("Version cannot be empty"));
    }

    Ok(())
}

/// Parse a source specification into its components
///
/// Returns (registry, identifier, version, format)
///
/// Format: registry:identifier-format@version
/// - registry: The source registry (e.g., "uniprot", "ncbi")
/// - identifier: The resource identifier (e.g., "P01308", "blast")
/// - format: Optional format suffix (e.g., "fasta", "xml") - last segment after '-'
/// - version: The version string (e.g., "1.0", "2.14.0")
pub fn parse_source_spec(spec: &str) -> Result<(String, String, String, Option<String>)> {
    validate_source_spec(spec)?;

    let parts: Vec<&str> = spec.split(':').collect();
    let registry = parts[0].to_string();

    let identifier_version = parts[1];
    let version_parts: Vec<&str> = identifier_version.split('@').collect();
    let identifier_with_format = version_parts[0];
    let version = version_parts[1].to_string();

    // Check if identifier includes format (e.g., "P01308-fasta")
    // Format is always the last segment after '-'
    if let Some(dash_pos) = identifier_with_format.rfind('-') {
        let identifier = identifier_with_format[..dash_pos].to_string();
        let format = identifier_with_format[dash_pos + 1..].to_string();
        Ok((registry, identifier, version, Some(format)))
    } else {
        Ok((registry, identifier_with_format.to_string(), version, None))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_manifest_creation() {
        let manifest = Manifest::new("test-project".to_string(), "0.1.0".to_string());
        assert_eq!(manifest.project.name, "test-project");
        assert_eq!(manifest.project.version, "0.1.0");
        assert_eq!(manifest.sources.len(), 0);
        assert_eq!(manifest.tools.len(), 0);
    }

    #[test]
    fn test_manifest_with_description() {
        let manifest = Manifest::with_description(
            "test-project".to_string(),
            "0.1.0".to_string(),
            "A test project".to_string(),
        );
        assert_eq!(manifest.project.description, Some("A test project".to_string()));
    }

    #[test]
    fn test_add_remove_source() {
        let mut manifest = Manifest::default();

        manifest.add_source("uniprot:P01308-fasta@1.0".to_string());
        assert_eq!(manifest.sources.len(), 1);
        assert!(manifest.has_source("uniprot:P01308-fasta@1.0"));

        // Adding duplicate should not increase count
        manifest.add_source("uniprot:P01308-fasta@1.0".to_string());
        assert_eq!(manifest.sources.len(), 1);

        let removed = manifest.remove_source("uniprot:P01308-fasta@1.0");
        assert!(removed);
        assert_eq!(manifest.sources.len(), 0);

        let removed = manifest.remove_source("nonexistent");
        assert!(!removed);
    }

    #[test]
    fn test_manifest_save_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let mut manifest = Manifest::new("test-project".to_string(), "0.1.0".to_string());
        manifest.add_source("uniprot:P01308-fasta@1.0".to_string());
        manifest.add_tool("ncbi:blast@2.14.0".to_string());

        manifest.save(path).unwrap();

        let loaded = Manifest::load(path).unwrap();
        assert_eq!(loaded, manifest);
    }

    #[test]
    fn test_validate_source_spec() {
        // Valid specs
        assert!(validate_source_spec("uniprot:P01308-fasta@1.0").is_ok());
        assert!(validate_source_spec("ncbi:blast@2.14.0").is_ok());
        assert!(validate_source_spec("ensembl:homo_sapiens-gtf@110").is_ok());

        // Invalid specs
        assert!(validate_source_spec("invalid").is_err());
        assert!(validate_source_spec("registry:").is_err());
        assert!(validate_source_spec(":identifier@1.0").is_err());
        assert!(validate_source_spec("registry:identifier").is_err());
        assert!(validate_source_spec("registry:@1.0").is_err());
        assert!(validate_source_spec("registry:identifier@").is_err());
    }

    #[test]
    fn test_parse_source_spec() {
        let (registry, identifier, version, format) =
            parse_source_spec("uniprot:P01308-fasta@1.0").unwrap();
        assert_eq!(registry, "uniprot");
        assert_eq!(identifier, "P01308");
        assert_eq!(version, "1.0");
        assert_eq!(format, Some("fasta".to_string()));

        let (registry, identifier, version, format) = parse_source_spec("ncbi:blast@2.14.0").unwrap();
        assert_eq!(registry, "ncbi");
        assert_eq!(identifier, "blast");
        assert_eq!(version, "2.14.0");
        assert_eq!(format, None);
    }

    #[test]
    fn test_manifest_validate() {
        let mut manifest = Manifest::new("test".to_string(), "1.0".to_string());
        assert!(manifest.validate().is_ok());

        manifest.add_source("invalid-spec".to_string());
        assert!(manifest.validate().is_err());
    }
}
