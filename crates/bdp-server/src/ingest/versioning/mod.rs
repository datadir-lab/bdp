//! Version management and change detection for ingestion pipelines
//!
//! This module provides the infrastructure for automatic semantic versioning
//! based on detected changes during data ingestion. It supports:
//!
//! - Automatic version bumps (MAJOR for breaking changes, MINOR for additions)
//! - Structured changelog generation
//! - Dependency cascade tracking
//!
//! # Example
//!
//! ```rust,ignore
//! use bdp_server::ingest::versioning::{VersionBumpDetector, VersionChangelog, BumpType};
//!
//! // Implement change detection for your data source
//! impl VersionBumpDetector for MyDataSourceDetector {
//!     async fn detect_changes(&self, pool: &PgPool, ...) -> Result<VersionChangelog> {
//!         // Your detection logic here
//!     }
//! }
//! ```

mod calculator;
mod cascade;
mod detector;
mod storage;
mod types;

// Re-export types
pub use types::{
    BumpType, CascadeResult, ChangeType, ChangelogEntry, ChangelogSummary, DataSourceDependent,
    TriggerReason, TriggerType, VersionChangeType, VersionChangelog, VersionInfo, VersionTrigger,
    VersioningStrategy,
};

// Re-export detector trait and implementations
pub use detector::{
    get_detector, get_organization_versioning_strategy, GenbankBumpDetector,
    GeneOntologyBumpDetector, NcbiTaxonomyBumpDetector, UniProtBumpDetector, VersionBumpDetector,
};

// Re-export cascade functions
pub use cascade::{
    cascade_recursive, cascade_version_bump, find_dependents, find_dependents_by_entry,
};

// Re-export calculator functions and types
pub use calculator::{
    calculate_next_version, calculate_next_version_full, create_version, get_latest_version,
    get_latest_version_id, get_previous_version_id, get_version_details, update_version_changelog,
    SemanticVersion, VersionDetails,
};

// Re-export storage functions
pub use storage::{
    count_changelogs_by_trigger, delete_changelog, find_cascaded_changelogs, get_changelog,
    get_changelog_by_id, list_changelogs_for_data_source, save_changelog,
};
