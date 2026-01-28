//! Core types for version bump detection and changelog generation
//!
//! This module defines the core types used for tracking changes between
//! data source versions and generating structured changelogs.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Organization Versioning Strategy Types
// ============================================================================

/// Per-organization versioning strategy
///
/// Defines what constitutes MAJOR vs MINOR version bumps for an organization's
/// data sources. This allows different organizations to have different rules
/// based on their data characteristics.
///
/// # Example
///
/// ```rust,ignore
/// use bdp_server::ingest::versioning::VersioningStrategy;
///
/// let strategy = VersioningStrategy {
///     major_triggers: vec![
///         VersionTrigger {
///             change_type: VersionChangeType::Removed,
///             category: "proteins".to_string(),
///             description: "Proteins removed or deprecated".to_string(),
///         },
///     ],
///     minor_triggers: vec![
///         VersionTrigger {
///             change_type: VersionChangeType::Added,
///             category: "proteins".to_string(),
///             description: "New proteins added".to_string(),
///         },
///     ],
///     default_bump: BumpType::Minor,
///     cascade_on_major: true,
///     cascade_on_minor: true,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersioningStrategy {
    /// Changes that trigger a MAJOR version bump
    pub major_triggers: Vec<VersionTrigger>,

    /// Changes that trigger a MINOR version bump
    pub minor_triggers: Vec<VersionTrigger>,

    /// Default bump type when no triggers match
    #[serde(default = "default_bump")]
    pub default_bump: BumpType,

    /// Whether to cascade version bumps to dependents on MAJOR changes
    #[serde(default = "default_true")]
    pub cascade_on_major: bool,

    /// Whether to cascade version bumps to dependents on MINOR changes
    #[serde(default = "default_true")]
    pub cascade_on_minor: bool,
}

fn default_bump() -> BumpType {
    BumpType::Minor
}

fn default_true() -> bool {
    true
}

impl Default for VersioningStrategy {
    fn default() -> Self {
        Self {
            major_triggers: vec![
                VersionTrigger {
                    change_type: VersionChangeType::Removed,
                    category: "entries".to_string(),
                    description: "Entries removed or deprecated".to_string(),
                },
                VersionTrigger {
                    change_type: VersionChangeType::Modified,
                    category: "sequences".to_string(),
                    description: "Sequence data changed".to_string(),
                },
            ],
            minor_triggers: vec![
                VersionTrigger {
                    change_type: VersionChangeType::Added,
                    category: "entries".to_string(),
                    description: "New entries added".to_string(),
                },
                VersionTrigger {
                    change_type: VersionChangeType::Modified,
                    category: "annotations".to_string(),
                    description: "Annotations updated".to_string(),
                },
            ],
            default_bump: BumpType::Minor,
            cascade_on_major: true,
            cascade_on_minor: true,
        }
    }
}

impl VersioningStrategy {
    /// Create a new versioning strategy with custom triggers
    pub fn new(major_triggers: Vec<VersionTrigger>, minor_triggers: Vec<VersionTrigger>) -> Self {
        Self {
            major_triggers,
            minor_triggers,
            ..Default::default()
        }
    }

    /// Determine the bump type for a given change
    ///
    /// Checks if the change matches any major triggers first, then minor triggers,
    /// and falls back to the default bump type.
    pub fn determine_bump(&self, change_type: &ChangeType, category: &str) -> BumpType {
        // Convert ChangeType to VersionChangeType for comparison
        let version_change_type = match change_type {
            ChangeType::Added => VersionChangeType::Added,
            ChangeType::Removed => VersionChangeType::Removed,
            ChangeType::Modified => VersionChangeType::Modified,
            ChangeType::Schema => VersionChangeType::Schema,
            ChangeType::Dependency => VersionChangeType::Dependency,
        };

        // Check major triggers first
        for trigger in &self.major_triggers {
            if trigger.matches(&version_change_type, category) {
                return BumpType::Major;
            }
        }

        // Check minor triggers
        for trigger in &self.minor_triggers {
            if trigger.matches(&version_change_type, category) {
                return BumpType::Minor;
            }
        }

        // Fall back to default
        self.default_bump
    }

    /// Determine bump type from a list of changelog entries
    pub fn determine_bump_from_entries(&self, entries: &[ChangelogEntry]) -> BumpType {
        for entry in entries {
            if entry.is_breaking {
                return BumpType::Major;
            }
            let bump = self.determine_bump(&entry.change_type, &entry.category);
            if bump.is_major() {
                return BumpType::Major;
            }
        }
        self.default_bump
    }

    /// Check if cascading should occur for the given bump type
    pub fn should_cascade(&self, bump_type: BumpType) -> bool {
        match bump_type {
            BumpType::Major => self.cascade_on_major,
            BumpType::Minor => self.cascade_on_minor,
        }
    }

    /// Create a UniProt-specific versioning strategy
    pub fn uniprot() -> Self {
        Self {
            major_triggers: vec![
                VersionTrigger {
                    change_type: VersionChangeType::Removed,
                    category: "proteins".to_string(),
                    description: "Proteins removed or deprecated".to_string(),
                },
                VersionTrigger {
                    change_type: VersionChangeType::Modified,
                    category: "sequences".to_string(),
                    description: "Protein sequences corrected or updated".to_string(),
                },
            ],
            minor_triggers: vec![
                VersionTrigger {
                    change_type: VersionChangeType::Added,
                    category: "proteins".to_string(),
                    description: "New proteins added from SwissProt release".to_string(),
                },
                VersionTrigger {
                    change_type: VersionChangeType::Modified,
                    category: "annotations".to_string(),
                    description: "Protein annotations updated (GO terms, features, etc.)"
                        .to_string(),
                },
            ],
            default_bump: BumpType::Minor,
            cascade_on_major: true,
            cascade_on_minor: true,
        }
    }

    /// Create an NCBI Taxonomy-specific versioning strategy
    pub fn ncbi_taxonomy() -> Self {
        Self {
            major_triggers: vec![
                VersionTrigger {
                    change_type: VersionChangeType::Removed,
                    category: "taxa".to_string(),
                    description: "Taxonomy nodes removed or merged".to_string(),
                },
                VersionTrigger {
                    change_type: VersionChangeType::Modified,
                    category: "names".to_string(),
                    description: "Scientific names changed".to_string(),
                },
            ],
            minor_triggers: vec![
                VersionTrigger {
                    change_type: VersionChangeType::Added,
                    category: "taxa".to_string(),
                    description: "New taxonomy nodes added".to_string(),
                },
                VersionTrigger {
                    change_type: VersionChangeType::Modified,
                    category: "lineage".to_string(),
                    description: "Lineage relationships refined".to_string(),
                },
            ],
            default_bump: BumpType::Minor,
            cascade_on_major: true,
            cascade_on_minor: true,
        }
    }

    /// Create a Gene Ontology-specific versioning strategy
    pub fn gene_ontology() -> Self {
        Self {
            major_triggers: vec![VersionTrigger {
                change_type: VersionChangeType::Removed,
                category: "terms".to_string(),
                description: "GO terms marked as obsolete".to_string(),
            }],
            minor_triggers: vec![
                VersionTrigger {
                    change_type: VersionChangeType::Added,
                    category: "terms".to_string(),
                    description: "New GO terms added".to_string(),
                },
                VersionTrigger {
                    change_type: VersionChangeType::Modified,
                    category: "definitions".to_string(),
                    description: "GO term definitions updated".to_string(),
                },
                VersionTrigger {
                    change_type: VersionChangeType::Modified,
                    category: "relationships".to_string(),
                    description: "Term relationships added or updated".to_string(),
                },
            ],
            default_bump: BumpType::Minor,
            cascade_on_major: true,
            cascade_on_minor: false, // GO minor changes typically don't cascade
        }
    }

    /// Create a GenBank/RefSeq-specific versioning strategy
    pub fn genbank() -> Self {
        Self {
            major_triggers: vec![
                VersionTrigger {
                    change_type: VersionChangeType::Removed,
                    category: "sequences".to_string(),
                    description: "Sequences withdrawn or superseded".to_string(),
                },
                VersionTrigger {
                    change_type: VersionChangeType::Modified,
                    category: "sequences".to_string(),
                    description: "Sequence data corrected".to_string(),
                },
            ],
            minor_triggers: vec![
                VersionTrigger {
                    change_type: VersionChangeType::Added,
                    category: "sequences".to_string(),
                    description: "New sequences added".to_string(),
                },
                VersionTrigger {
                    change_type: VersionChangeType::Modified,
                    category: "annotations".to_string(),
                    description: "Sequence annotations updated".to_string(),
                },
            ],
            default_bump: BumpType::Minor,
            cascade_on_major: true,
            cascade_on_minor: true,
        }
    }
}

/// A trigger condition for version bumps
///
/// Defines what type of change to a specific category should trigger
/// a version bump.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionTrigger {
    /// The type of change (added, removed, modified, etc.)
    pub change_type: VersionChangeType,

    /// The category of data affected (proteins, taxa, terms, sequences, etc.)
    pub category: String,

    /// Human-readable description of what this trigger means
    pub description: String,
}

impl VersionTrigger {
    /// Create a new version trigger
    pub fn new(
        change_type: VersionChangeType,
        category: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            change_type,
            category: category.into(),
            description: description.into(),
        }
    }

    /// Check if this trigger matches the given change type and category
    pub fn matches(&self, change_type: &VersionChangeType, category: &str) -> bool {
        self.change_type == *change_type && (self.category == category || self.category == "*")
    }
}

/// Change type for version triggers
///
/// This is separate from ChangeType to allow for serialization in the
/// versioning strategy JSON without coupling to the changelog enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionChangeType {
    /// New entries added
    Added,
    /// Entries removed or deprecated
    Removed,
    /// Entries modified
    Modified,
    /// Schema or format changes
    Schema,
    /// Dependency changes
    Dependency,
}

impl std::fmt::Display for VersionChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionChangeType::Added => write!(f, "added"),
            VersionChangeType::Removed => write!(f, "removed"),
            VersionChangeType::Modified => write!(f, "modified"),
            VersionChangeType::Schema => write!(f, "schema"),
            VersionChangeType::Dependency => write!(f, "dependency"),
        }
    }
}

// ============================================================================
// Existing Types
// ============================================================================

/// Version bump type - only MAJOR or MINOR, no patch
///
/// BDP uses a simplified semantic versioning scheme:
/// - MAJOR: Breaking changes (protein removed, accession changed, sequence modified)
/// - MINOR: Non-breaking additions or updates (proteins added, annotations updated)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BumpType {
    /// Breaking changes that may require downstream consumers to update
    Major,
    /// Non-breaking additions or updates
    Minor,
}

impl BumpType {
    /// Returns true if this is a major (breaking) bump
    pub fn is_major(&self) -> bool {
        matches!(self, BumpType::Major)
    }

    /// Returns true if this is a minor (non-breaking) bump
    pub fn is_minor(&self) -> bool {
        matches!(self, BumpType::Minor)
    }

    /// Convert to database enum string
    pub fn as_db_str(&self) -> &'static str {
        match self {
            BumpType::Major => "major",
            BumpType::Minor => "minor",
        }
    }

    /// Create from database enum string
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "major" => Some(BumpType::Major),
            "minor" => Some(BumpType::Minor),
            _ => None,
        }
    }
}

impl std::fmt::Display for BumpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BumpType::Major => write!(f, "major"),
            BumpType::Minor => write!(f, "minor"),
        }
    }
}

/// Changelog entry type - categorizes what kind of change occurred
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// New entries added to the data source
    Added,
    /// Entries removed or deprecated from the data source
    Removed,
    /// Existing entries modified (annotations, metadata)
    Modified,
    /// Schema or format changes
    Schema,
    /// Dependency version updated
    Dependency,
}

impl ChangeType {
    /// Convert to database enum string
    pub fn as_db_str(&self) -> &'static str {
        match self {
            ChangeType::Added => "added",
            ChangeType::Removed => "removed",
            ChangeType::Modified => "modified",
            ChangeType::Schema => "schema",
            ChangeType::Dependency => "dependency",
        }
    }

    /// Create from database enum string
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "added" => Some(ChangeType::Added),
            "removed" => Some(ChangeType::Removed),
            "modified" => Some(ChangeType::Modified),
            "schema" => Some(ChangeType::Schema),
            "dependency" => Some(ChangeType::Dependency),
            _ => None,
        }
    }
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_db_str())
    }
}

/// Single changelog entry describing one type of change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    /// Type of change (added, removed, modified, schema, dependency)
    pub change_type: ChangeType,

    /// Category of items affected (proteins, sequences, annotations, taxa, terms, etc.)
    pub category: String,

    /// Number of items affected (optional for schema changes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,

    /// Human-readable description of the change
    pub description: String,

    /// Whether this change is breaking (triggers MAJOR bump)
    pub is_breaking: bool,
}

impl ChangelogEntry {
    /// Create a new changelog entry
    pub fn new(
        change_type: ChangeType,
        category: impl Into<String>,
        description: impl Into<String>,
        is_breaking: bool,
    ) -> Self {
        Self {
            change_type,
            category: category.into(),
            count: None,
            description: description.into(),
            is_breaking,
        }
    }

    /// Create a changelog entry with a count
    pub fn with_count(
        change_type: ChangeType,
        category: impl Into<String>,
        count: i64,
        description: impl Into<String>,
        is_breaking: bool,
    ) -> Self {
        Self {
            change_type,
            category: category.into(),
            count: Some(count),
            description: description.into(),
            is_breaking,
        }
    }

    /// Create an "added" entry
    pub fn added(category: impl Into<String>, count: i64, description: impl Into<String>) -> Self {
        Self::with_count(ChangeType::Added, category, count, description, false)
    }

    /// Create a "removed" entry (breaking by default)
    pub fn removed(
        category: impl Into<String>,
        count: i64,
        description: impl Into<String>,
    ) -> Self {
        Self::with_count(ChangeType::Removed, category, count, description, true)
    }

    /// Create a "modified" entry
    pub fn modified(
        category: impl Into<String>,
        count: i64,
        description: impl Into<String>,
        is_breaking: bool,
    ) -> Self {
        Self::with_count(ChangeType::Modified, category, count, description, is_breaking)
    }

    /// Create a "schema" entry
    pub fn schema(description: impl Into<String>, is_breaking: bool) -> Self {
        Self::new(ChangeType::Schema, "schema", description, is_breaking)
    }

    /// Create a "dependency" entry
    pub fn dependency(
        category: impl Into<String>,
        description: impl Into<String>,
        is_breaking: bool,
    ) -> Self {
        Self::new(ChangeType::Dependency, category, description, is_breaking)
    }
}

/// Summary statistics for a version changelog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogSummary {
    /// Total number of entries before the update
    pub total_entries_before: i64,

    /// Total number of entries after the update
    pub total_entries_after: i64,

    /// Number of new entries added
    pub entries_added: i64,

    /// Number of entries removed
    pub entries_removed: i64,

    /// Number of entries modified
    pub entries_modified: i64,

    /// What triggered this version update
    pub triggered_by: TriggerReason,
}

impl ChangelogSummary {
    /// Create a new changelog summary
    pub fn new(
        total_entries_before: i64,
        total_entries_after: i64,
        entries_added: i64,
        entries_removed: i64,
        entries_modified: i64,
        triggered_by: TriggerReason,
    ) -> Self {
        Self {
            total_entries_before,
            total_entries_after,
            entries_added,
            entries_removed,
            entries_modified,
            triggered_by,
        }
    }

    /// Create an empty summary for a new data source
    pub fn initial(count: i64) -> Self {
        Self {
            total_entries_before: 0,
            total_entries_after: count,
            entries_added: count,
            entries_removed: 0,
            entries_modified: 0,
            triggered_by: TriggerReason::NewRelease,
        }
    }

    /// Calculate net change in entries
    pub fn net_change(&self) -> i64 {
        self.total_entries_after - self.total_entries_before
    }

    /// Calculate total changes (added + removed + modified)
    pub fn total_changes(&self) -> i64 {
        self.entries_added + self.entries_removed + self.entries_modified
    }
}

impl Default for ChangelogSummary {
    fn default() -> Self {
        Self {
            total_entries_before: 0,
            total_entries_after: 0,
            entries_added: 0,
            entries_removed: 0,
            entries_modified: 0,
            triggered_by: TriggerReason::NewRelease,
        }
    }
}

/// Reason for triggering a version update
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TriggerReason {
    /// New upstream release available (e.g., new UniProt release)
    #[default]
    NewRelease,
    /// Triggered by an upstream dependency update
    UpstreamDependency,
    /// Manually triggered update (re-ingestion, correction, etc.)
    Manual,
}

impl TriggerReason {
    /// Convert to database enum string
    pub fn as_db_str(&self) -> &'static str {
        match self {
            TriggerReason::NewRelease => "new_release",
            TriggerReason::UpstreamDependency => "upstream_dependency",
            TriggerReason::Manual => "manual",
        }
    }

    /// Create from database enum string
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "new_release" => Some(TriggerReason::NewRelease),
            "upstream_dependency" => Some(TriggerReason::UpstreamDependency),
            "manual" => Some(TriggerReason::Manual),
            _ => None,
        }
    }
}

impl std::fmt::Display for TriggerReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriggerReason::NewRelease => write!(f, "new release"),
            TriggerReason::UpstreamDependency => write!(f, "upstream dependency update"),
            TriggerReason::Manual => write!(f, "manual trigger"),
        }
    }
}

/// Complete changelog for a version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionChangelog {
    /// Type of version bump (major or minor)
    pub bump_type: BumpType,

    /// List of individual change entries
    pub entries: Vec<ChangelogEntry>,

    /// Summary statistics
    pub summary: ChangelogSummary,

    /// Human-readable summary text
    pub summary_text: String,

    /// Version ID that triggered this update (for dependency cascades)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggered_by_version_id: Option<Uuid>,
}

impl VersionChangelog {
    /// Create a new version changelog
    pub fn new(
        bump_type: BumpType,
        entries: Vec<ChangelogEntry>,
        summary: ChangelogSummary,
        summary_text: impl Into<String>,
    ) -> Self {
        Self {
            bump_type,
            entries,
            summary,
            summary_text: summary_text.into(),
            triggered_by_version_id: None,
        }
    }

    /// Create a changelog triggered by a dependency update
    pub fn from_dependency(
        bump_type: BumpType,
        entries: Vec<ChangelogEntry>,
        summary: ChangelogSummary,
        summary_text: impl Into<String>,
        triggered_by: Uuid,
    ) -> Self {
        Self {
            bump_type,
            entries,
            summary,
            summary_text: summary_text.into(),
            triggered_by_version_id: Some(triggered_by),
        }
    }

    /// Determine bump type from entries (any breaking change = major)
    pub fn determine_bump_type(entries: &[ChangelogEntry]) -> BumpType {
        if entries.iter().any(|e| e.is_breaking) {
            BumpType::Major
        } else {
            BumpType::Minor
        }
    }

    /// Check if any entries are breaking changes
    pub fn has_breaking_changes(&self) -> bool {
        self.entries.iter().any(|e| e.is_breaking)
    }

    /// Generate a summary text from the entries and summary
    pub fn generate_summary_text(&self) -> String {
        let mut parts = Vec::new();

        if self.summary.entries_added > 0 {
            parts.push(format!("{} added", self.summary.entries_added));
        }
        if self.summary.entries_removed > 0 {
            parts.push(format!("{} removed", self.summary.entries_removed));
        }
        if self.summary.entries_modified > 0 {
            parts.push(format!("{} modified", self.summary.entries_modified));
        }

        let changes_text = if parts.is_empty() {
            "No changes".to_string()
        } else {
            parts.join(", ")
        };

        let trigger_text = match self.summary.triggered_by {
            TriggerReason::NewRelease => "New upstream release",
            TriggerReason::UpstreamDependency => "Upstream dependency update",
            TriggerReason::Manual => "Manual update",
        };

        format!("{} ({} version bump): {}", trigger_text, self.bump_type, changes_text)
    }
}

/// Information about a data source that depends on another
#[derive(Debug, Clone)]
pub struct DataSourceDependent {
    /// Registry entry ID of the dependent data source
    pub entry_id: Uuid,

    /// Version ID of the dependent (current version that will be updated)
    pub version_id: Uuid,

    /// Slug of the dependent data source
    pub slug: String,

    /// Name of the dependent data source
    pub name: String,

    /// Current version string
    pub current_version: String,

    /// Organization slug
    pub organization_slug: String,
}

/// Alias for TriggerReason for API compatibility
pub type TriggerType = TriggerReason;

/// Information about a version (for queries)
#[derive(Debug, Clone)]
pub struct VersionInfo {
    /// Version UUID
    pub id: Uuid,
    /// Registry entry ID (data source ID)
    pub entry_id: Uuid,
    /// Version string (e.g., "1.0.0")
    pub version: String,
    /// External version from source (e.g., "2024_01")
    pub external_version: Option<String>,
    /// Major version number
    pub version_major: i32,
    /// Minor version number
    pub version_minor: i32,
    /// Patch version number
    pub version_patch: i32,
}

impl VersionInfo {
    /// Get the full semantic version string
    pub fn semver_string(&self) -> String {
        format!("{}.{}.{}", self.version_major, self.version_minor, self.version_patch)
    }
}

/// Result of a version cascade operation
#[derive(Debug, Clone)]
pub struct CascadeResult {
    /// ID of the dependent data source entry
    pub entry_id: Uuid,
    /// Slug of the dependent data source
    pub entry_slug: String,
    /// ID of the newly created version
    pub new_version_id: Uuid,
    /// New version string
    pub new_version: String,
    /// ID of the changelog entry created
    pub changelog_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bump_type_display() {
        assert_eq!(BumpType::Major.to_string(), "major");
        assert_eq!(BumpType::Minor.to_string(), "minor");
    }

    #[test]
    fn test_bump_type_db_conversion() {
        assert_eq!(BumpType::Major.as_db_str(), "major");
        assert_eq!(BumpType::Minor.as_db_str(), "minor");
        assert_eq!(BumpType::from_db_str("major"), Some(BumpType::Major));
        assert_eq!(BumpType::from_db_str("minor"), Some(BumpType::Minor));
        assert_eq!(BumpType::from_db_str("invalid"), None);
    }

    #[test]
    fn test_change_type_db_conversion() {
        assert_eq!(ChangeType::Added.as_db_str(), "added");
        assert_eq!(ChangeType::Removed.as_db_str(), "removed");
        assert_eq!(ChangeType::from_db_str("added"), Some(ChangeType::Added));
        assert_eq!(ChangeType::from_db_str("invalid"), None);
    }

    #[test]
    fn test_changelog_entry_creation() {
        let entry = ChangelogEntry::added("proteins", 100, "New proteins from release");
        assert_eq!(entry.change_type, ChangeType::Added);
        assert_eq!(entry.category, "proteins");
        assert_eq!(entry.count, Some(100));
        assert!(!entry.is_breaking);

        let entry = ChangelogEntry::removed("proteins", 5, "Deprecated proteins");
        assert!(entry.is_breaking);
    }

    #[test]
    fn test_changelog_summary() {
        let summary = ChangelogSummary::new(1000, 1100, 150, 50, 200, TriggerReason::NewRelease);
        assert_eq!(summary.net_change(), 100);
        assert_eq!(summary.total_changes(), 400);
    }

    #[test]
    fn test_determine_bump_type() {
        let entries_minor = vec![
            ChangelogEntry::added("proteins", 100, "Added"),
            ChangelogEntry::modified("annotations", 50, "Updated", false),
        ];
        assert_eq!(VersionChangelog::determine_bump_type(&entries_minor), BumpType::Minor);

        let entries_major = vec![
            ChangelogEntry::added("proteins", 100, "Added"),
            ChangelogEntry::removed("proteins", 10, "Removed"),
        ];
        assert_eq!(VersionChangelog::determine_bump_type(&entries_major), BumpType::Major);
    }

    #[test]
    fn test_trigger_reason_display() {
        assert_eq!(TriggerReason::NewRelease.to_string(), "new release");
        assert_eq!(TriggerReason::UpstreamDependency.to_string(), "upstream dependency update");
        assert_eq!(TriggerReason::Manual.to_string(), "manual trigger");
    }

    // ========================================================================
    // VersioningStrategy Tests
    // ========================================================================

    #[test]
    fn test_versioning_strategy_default() {
        let strategy = VersioningStrategy::default();
        assert!(!strategy.major_triggers.is_empty());
        assert!(!strategy.minor_triggers.is_empty());
        assert_eq!(strategy.default_bump, BumpType::Minor);
        assert!(strategy.cascade_on_major);
        assert!(strategy.cascade_on_minor);
    }

    #[test]
    fn test_versioning_strategy_determine_bump() {
        let strategy = VersioningStrategy::uniprot();

        // Removed proteins should trigger major
        let bump = strategy.determine_bump(&ChangeType::Removed, "proteins");
        assert_eq!(bump, BumpType::Major);

        // Added proteins should trigger minor
        let bump = strategy.determine_bump(&ChangeType::Added, "proteins");
        assert_eq!(bump, BumpType::Minor);

        // Modified sequences should trigger major
        let bump = strategy.determine_bump(&ChangeType::Modified, "sequences");
        assert_eq!(bump, BumpType::Major);

        // Modified annotations should trigger minor
        let bump = strategy.determine_bump(&ChangeType::Modified, "annotations");
        assert_eq!(bump, BumpType::Minor);
    }

    #[test]
    fn test_versioning_strategy_should_cascade() {
        let mut strategy = VersioningStrategy::default();

        assert!(strategy.should_cascade(BumpType::Major));
        assert!(strategy.should_cascade(BumpType::Minor));

        strategy.cascade_on_minor = false;
        assert!(strategy.should_cascade(BumpType::Major));
        assert!(!strategy.should_cascade(BumpType::Minor));
    }

    #[test]
    fn test_versioning_strategy_serialization() {
        let strategy = VersioningStrategy::uniprot();
        let json = serde_json::to_string(&strategy).expect("Serialization failed");
        let deserialized: VersioningStrategy =
            serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(strategy, deserialized);
    }

    #[test]
    fn test_version_trigger_matches() {
        let trigger =
            VersionTrigger::new(VersionChangeType::Removed, "proteins", "Proteins removed");

        assert!(trigger.matches(&VersionChangeType::Removed, "proteins"));
        assert!(!trigger.matches(&VersionChangeType::Added, "proteins"));
        assert!(!trigger.matches(&VersionChangeType::Removed, "taxa"));
    }

    #[test]
    fn test_version_trigger_wildcard_category() {
        let trigger = VersionTrigger::new(VersionChangeType::Removed, "*", "Any removal");

        assert!(trigger.matches(&VersionChangeType::Removed, "proteins"));
        assert!(trigger.matches(&VersionChangeType::Removed, "taxa"));
        assert!(!trigger.matches(&VersionChangeType::Added, "proteins"));
    }

    #[test]
    fn test_organization_specific_strategies() {
        // UniProt
        let uniprot = VersioningStrategy::uniprot();
        assert!(uniprot
            .major_triggers
            .iter()
            .any(|t| t.category == "proteins"));
        assert!(uniprot
            .minor_triggers
            .iter()
            .any(|t| t.category == "annotations"));

        // NCBI Taxonomy
        let ncbi = VersioningStrategy::ncbi_taxonomy();
        assert!(ncbi.major_triggers.iter().any(|t| t.category == "taxa"));
        assert!(ncbi.minor_triggers.iter().any(|t| t.category == "lineage"));

        // Gene Ontology
        let go = VersioningStrategy::gene_ontology();
        assert!(go.major_triggers.iter().any(|t| t.category == "terms"));
        assert!(!go.cascade_on_minor); // GO doesn't cascade minor changes

        // GenBank
        let genbank = VersioningStrategy::genbank();
        assert!(genbank
            .major_triggers
            .iter()
            .any(|t| t.category == "sequences"));
    }

    #[test]
    fn test_determine_bump_from_entries() {
        let strategy = VersioningStrategy::uniprot();

        // Minor changes only
        let entries = vec![
            ChangelogEntry::added("proteins", 100, "Added"),
            ChangelogEntry::modified("annotations", 50, "Updated", false),
        ];
        assert_eq!(strategy.determine_bump_from_entries(&entries), BumpType::Minor);

        // With breaking change
        let entries = vec![
            ChangelogEntry::added("proteins", 100, "Added"),
            ChangelogEntry::removed("proteins", 10, "Removed"),
        ];
        assert_eq!(strategy.determine_bump_from_entries(&entries), BumpType::Major);
    }
}
