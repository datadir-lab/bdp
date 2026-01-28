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

mod types;
mod detector;
mod cascade;
mod calculator;
mod storage;

// Re-export types
pub use types::{
    BumpType,
    ChangeType,
    ChangelogEntry,
    ChangelogSummary,
    CascadeResult,
    DataSourceDependent,
    TriggerReason,
    TriggerType,
    VersionChangelog,
    VersionChangeType,
    VersionInfo,
    VersioningStrategy,
    VersionTrigger,
};

// Re-export detector trait and implementations
pub use detector::{
    GenbankBumpDetector,
    GeneOntologyBumpDetector,
    NcbiTaxonomyBumpDetector,
    UniProtBumpDetector,
    VersionBumpDetector,
    get_detector,
    get_organization_versioning_strategy,
};

// Re-export cascade functions
pub use cascade::{
    cascade_version_bump,
    cascade_recursive,
    find_dependents,
    find_dependents_by_entry,
};

// Re-export calculator functions and types
pub use calculator::{
    SemanticVersion,
    calculate_next_version,
    calculate_next_version_full,
    create_version,
    get_latest_version,
    get_latest_version_id,
    get_previous_version_id,
    get_version_details,
    update_version_changelog,
    VersionDetails,
};

// Re-export storage functions
pub use storage::{
    save_changelog,
    get_changelog,
    get_changelog_by_id,
    list_changelogs_for_data_source,
    find_cascaded_changelogs,
    delete_changelog,
    count_changelogs_by_trigger,
};
