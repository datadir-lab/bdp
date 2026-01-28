//! Lockfile handling (bdl.lock)
//!
//! The lockfile stores resolved dependency information with exact versions and checksums.

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// BDP lockfile (bdl.lock)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Lockfile {
    /// Lockfile format version
    pub lockfile_version: u32,

    /// Timestamp when lockfile was generated
    pub generated: DateTime<Utc>,

    /// Locked source entries
    #[serde(default)]
    pub sources: HashMap<String, SourceEntry>,

    /// Locked tool entries
    #[serde(default)]
    pub tools: HashMap<String, ToolEntry>,
}

/// Entry for a locked source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceEntry {
    /// Resolved source specification (e.g., "uniprot:P01308@1.0")
    pub resolved: String,

    /// Format of the source (e.g., "fasta", "gtf")
    pub format: String,

    /// SHA-256 checksum
    pub checksum: String,

    /// File size in bytes
    pub size: i64,

    /// External version string
    pub external_version: String,

    /// Number of dependencies (for sources with dependencies)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_count: Option<i32>,
}

/// Entry for a locked tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolEntry {
    /// Resolved tool specification (e.g., "ncbi:blast@2.14.0")
    pub resolved: String,

    /// Tool version
    pub version: String,

    /// Download URL or registry location
    pub url: String,

    /// SHA-256 checksum
    pub checksum: String,

    /// Size in bytes
    pub size: i64,
}

impl Lockfile {
    /// Create a new empty lockfile
    pub fn new() -> Self {
        Self {
            lockfile_version: 1,
            generated: Utc::now(),
            sources: HashMap::new(),
            tools: HashMap::new(),
        }
    }

    /// Load lockfile from file
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let lockfile: Lockfile = serde_json::from_str(&content)?;
        Ok(lockfile)
    }

    /// Save lockfile to file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Add or update a source entry
    pub fn add_source(&mut self, spec: String, entry: SourceEntry) {
        self.sources.insert(spec, entry);
        self.generated = Utc::now();
    }

    /// Add or update a tool entry
    pub fn add_tool(&mut self, spec: String, entry: ToolEntry) {
        self.tools.insert(spec, entry);
        self.generated = Utc::now();
    }

    /// Get a source entry
    pub fn get_source(&self, spec: &str) -> Option<&SourceEntry> {
        self.sources.get(spec)
    }

    /// Get a tool entry
    pub fn get_tool(&self, spec: &str) -> Option<&ToolEntry> {
        self.tools.get(spec)
    }

    /// Remove a source entry
    pub fn remove_source(&mut self, spec: &str) -> bool {
        self.sources.remove(spec).is_some()
    }

    /// Remove a tool entry
    pub fn remove_tool(&mut self, spec: &str) -> bool {
        self.tools.remove(spec).is_some()
    }

    /// Check if lockfile is empty
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty() && self.tools.is_empty()
    }

    /// Get total number of locked entries
    pub fn entry_count(&self) -> usize {
        self.sources.len() + self.tools.len()
    }
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceEntry {
    /// Create a new source entry
    pub fn new(
        resolved: String,
        format: String,
        checksum: String,
        size: i64,
        external_version: String,
    ) -> Self {
        Self {
            resolved,
            format,
            checksum,
            size,
            external_version,
            dependency_count: None,
        }
    }

    /// Create a source entry with dependency count
    pub fn with_dependencies(
        resolved: String,
        format: String,
        checksum: String,
        size: i64,
        external_version: String,
        dependency_count: i32,
    ) -> Self {
        Self {
            resolved,
            format,
            checksum,
            size,
            external_version,
            dependency_count: Some(dependency_count),
        }
    }
}

impl ToolEntry {
    /// Create a new tool entry
    pub fn new(
        resolved: String,
        version: String,
        url: String,
        checksum: String,
        size: i64,
    ) -> Self {
        Self {
            resolved,
            version,
            url,
            checksum,
            size,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_lockfile_creation() {
        let lockfile = Lockfile::new();
        assert_eq!(lockfile.lockfile_version, 1);
        assert!(lockfile.sources.is_empty());
        assert!(lockfile.tools.is_empty());
        assert!(lockfile.is_empty());
    }

    #[test]
    fn test_add_source_entry() {
        let mut lockfile = Lockfile::new();

        let entry = SourceEntry::new(
            "uniprot:P01308@1.0".to_string(),
            "fasta".to_string(),
            "abc123".to_string(),
            1024,
            "1.0.0".to_string(),
        );

        lockfile.add_source("uniprot:P01308-fasta@1.0".to_string(), entry.clone());

        assert_eq!(lockfile.sources.len(), 1);
        assert!(!lockfile.is_empty());

        let retrieved = lockfile.get_source("uniprot:P01308-fasta@1.0").unwrap();
        assert_eq!(retrieved, &entry);
    }

    #[test]
    fn test_add_tool_entry() {
        let mut lockfile = Lockfile::new();

        let entry = ToolEntry::new(
            "ncbi:blast@2.14.0".to_string(),
            "2.14.0".to_string(),
            "https://example.com/blast".to_string(),
            "def456".to_string(),
            2048,
        );

        lockfile.add_tool("ncbi:blast@2.14.0".to_string(), entry.clone());

        assert_eq!(lockfile.tools.len(), 1);
        let retrieved = lockfile.get_tool("ncbi:blast@2.14.0").unwrap();
        assert_eq!(retrieved, &entry);
    }

    #[test]
    fn test_remove_entries() {
        let mut lockfile = Lockfile::new();

        let source = SourceEntry::new(
            "uniprot:P01308@1.0".to_string(),
            "fasta".to_string(),
            "abc123".to_string(),
            1024,
            "1.0.0".to_string(),
        );

        lockfile.add_source("uniprot:P01308-fasta@1.0".to_string(), source);

        assert!(lockfile.remove_source("uniprot:P01308-fasta@1.0"));
        assert!(!lockfile.remove_source("nonexistent"));
        assert!(lockfile.is_empty());
    }

    #[test]
    fn test_lockfile_save_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let mut lockfile = Lockfile::new();

        let source = SourceEntry::new(
            "uniprot:P01308@1.0".to_string(),
            "fasta".to_string(),
            "abc123".to_string(),
            1024,
            "1.0.0".to_string(),
        );

        lockfile.add_source("uniprot:P01308-fasta@1.0".to_string(), source);

        lockfile.save(path).unwrap();

        let loaded = Lockfile::load(path).unwrap();
        assert_eq!(loaded.sources.len(), 1);
        assert_eq!(loaded.lockfile_version, 1);
    }

    #[test]
    fn test_source_entry_with_dependencies() {
        let entry = SourceEntry::with_dependencies(
            "ensembl:homo_sapiens@110".to_string(),
            "gtf".to_string(),
            "xyz789".to_string(),
            4096,
            "110.0".to_string(),
            5,
        );

        assert_eq!(entry.dependency_count, Some(5));
    }

    #[test]
    fn test_entry_count() {
        let mut lockfile = Lockfile::new();

        let source = SourceEntry::new(
            "uniprot:P01308@1.0".to_string(),
            "fasta".to_string(),
            "abc123".to_string(),
            1024,
            "1.0.0".to_string(),
        );

        let tool = ToolEntry::new(
            "ncbi:blast@2.14.0".to_string(),
            "2.14.0".to_string(),
            "https://example.com/blast".to_string(),
            "def456".to_string(),
            2048,
        );

        lockfile.add_source("source1".to_string(), source);
        lockfile.add_tool("tool1".to_string(), tool);

        assert_eq!(lockfile.entry_count(), 2);
    }
}
