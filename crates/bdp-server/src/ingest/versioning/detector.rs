//! Version bump detection trait and implementations
//!
//! This module provides the trait and implementations for detecting version bumps
//! for different data sources (UniProt, NCBI Taxonomy, Gene Ontology, GenBank/RefSeq).
//!
//! Detectors can optionally use an organization's custom versioning strategy if one
//! is defined in the database. If no strategy is defined, the detector falls back to
//! its default behavior.

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use super::types::{
    BumpType, ChangelogEntry, ChangelogSummary, TriggerReason, VersionChangelog, VersioningStrategy,
};

/// Fetch the organization's versioning strategy for a data source
///
/// Looks up the registry entry and its organization, then returns the
/// versioning strategy if one is defined.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `data_source_id` - The data source (registry entry) ID
///
/// # Returns
/// The versioning strategy if defined, None otherwise
pub async fn get_organization_versioning_strategy(
    pool: &PgPool,
    data_source_id: Uuid,
) -> Result<Option<VersioningStrategy>> {
    // Query the organization's versioning_strategy via the registry entry
    let result: Option<(Option<serde_json::Value>,)> = sqlx::query_as(
        r#"
        SELECT o.versioning_strategy
        FROM registry_entries re
        JOIN organizations o ON o.id = re.organization_id
        WHERE re.id = $1
        "#,
    )
    .bind(data_source_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch organization versioning strategy")?;

    match result {
        Some((Some(json_value),)) => {
            // Parse the JSON into a VersioningStrategy
            let strategy: VersioningStrategy = serde_json::from_value(json_value)
                .context("Failed to parse versioning strategy from database")?;
            debug!(
                data_source_id = %data_source_id,
                "Using organization's custom versioning strategy"
            );
            Ok(Some(strategy))
        },
        _ => {
            debug!(
                data_source_id = %data_source_id,
                "No custom versioning strategy defined, using detector defaults"
            );
            Ok(None)
        },
    }
}

/// Trait for detecting version bumps for different data sources
///
/// Implementations should compare old and new data to determine what changed
/// and whether those changes warrant a major or minor version bump.
#[async_trait]
pub trait VersionBumpDetector: Send + Sync {
    /// Compare old and new data, return changelog
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `data_source_id` - The data source (registry entry) ID
    /// * `previous_version_id` - The previous version ID (None for first version)
    ///
    /// # Returns
    /// A `VersionChangelog` describing what changed and the bump type
    async fn detect_changes(
        &self,
        pool: &PgPool,
        data_source_id: Uuid,
        previous_version_id: Option<Uuid>,
    ) -> Result<VersionChangelog>;

    /// Compare old and new data, using an organization's custom versioning strategy
    ///
    /// This method first attempts to fetch the organization's versioning strategy
    /// from the database. If one exists, it uses that to determine bump types.
    /// Otherwise, it falls back to the default detector behavior.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `data_source_id` - The data source (registry entry) ID
    /// * `previous_version_id` - The previous version ID (None for first version)
    ///
    /// # Returns
    /// A `VersionChangelog` describing what changed and the bump type
    async fn detect_changes_with_strategy(
        &self,
        pool: &PgPool,
        data_source_id: Uuid,
        previous_version_id: Option<Uuid>,
    ) -> Result<VersionChangelog> {
        // First get the changelog using the detector's default logic
        let mut changelog = self
            .detect_changes(pool, data_source_id, previous_version_id)
            .await?;

        // Try to get the organization's custom strategy
        if let Ok(Some(strategy)) = get_organization_versioning_strategy(pool, data_source_id).await
        {
            // Re-determine the bump type using the custom strategy
            let new_bump_type = strategy.determine_bump_from_entries(&changelog.entries);
            if new_bump_type != changelog.bump_type {
                info!(
                    data_source_id = %data_source_id,
                    original_bump = ?changelog.bump_type,
                    new_bump = ?new_bump_type,
                    "Organization's versioning strategy changed bump type"
                );
                changelog.bump_type = new_bump_type;
                // Regenerate summary text with new bump type
                changelog.summary_text = changelog.generate_summary_text();
            }
        }

        Ok(changelog)
    }

    /// Get the name of this detector for logging
    fn name(&self) -> &'static str;

    /// Get the default versioning strategy for this detector type
    ///
    /// This returns the built-in strategy that the detector would use
    /// if no organization-specific strategy is defined.
    fn default_strategy(&self) -> VersioningStrategy {
        VersioningStrategy::default()
    }
}

/// UniProt-specific version bump detector
///
/// Detects changes in proteins between versions:
/// - MAJOR if: proteins removed, accessions changed, sequences modified
/// - MINOR if: proteins added, annotations updated, metadata changes
#[derive(Debug, Clone, Default)]
pub struct UniProtBumpDetector;

impl UniProtBumpDetector {
    /// Create a new UniProt bump detector
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl VersionBumpDetector for UniProtBumpDetector {
    fn name(&self) -> &'static str {
        "UniProt"
    }

    fn default_strategy(&self) -> VersioningStrategy {
        VersioningStrategy::uniprot()
    }

    #[instrument(skip(self, pool), fields(detector = self.name()))]
    async fn detect_changes(
        &self,
        pool: &PgPool,
        data_source_id: Uuid,
        previous_version_id: Option<Uuid>,
    ) -> Result<VersionChangelog> {
        info!(
            data_source_id = %data_source_id,
            previous_version_id = ?previous_version_id,
            "Detecting UniProt version changes"
        );

        // If no previous version, this is the initial release
        let Some(prev_version_id) = previous_version_id else {
            return self.create_initial_changelog(pool, data_source_id).await;
        };

        let mut entries = Vec::new();
        let mut entries_added = 0i64;
        let mut entries_removed = 0i64;
        let mut entries_modified = 0i64;

        // Get counts from previous version
        let prev_count = self.get_protein_count(pool, prev_version_id).await?;

        // Get current counts (proteins linked to this data source)
        let current_count = self.get_current_protein_count(pool, data_source_id).await?;

        // Detect removed proteins (breaking change)
        let removed_proteins = self
            .detect_removed_proteins(pool, data_source_id, prev_version_id)
            .await?;
        if removed_proteins > 0 {
            entries.push(ChangelogEntry::removed(
                "proteins",
                removed_proteins,
                "Proteins removed or deprecated from SwissProt",
            ));
            entries_removed = removed_proteins;
        }

        // Detect added proteins (non-breaking)
        let added_proteins = self
            .detect_added_proteins(pool, data_source_id, prev_version_id)
            .await?;
        if added_proteins > 0 {
            entries.push(ChangelogEntry::added(
                "proteins",
                added_proteins,
                "New proteins added from SwissProt release",
            ));
            entries_added = added_proteins;
        }

        // Detect modified sequences (breaking change)
        let modified_sequences = self
            .detect_modified_sequences(pool, data_source_id, prev_version_id)
            .await?;
        if modified_sequences > 0 {
            entries.push(ChangelogEntry::modified(
                "sequences",
                modified_sequences,
                "Protein sequences corrected or updated",
                true, // Sequence changes are breaking
            ));
            entries_modified += modified_sequences;
        }

        // Detect modified annotations (non-breaking)
        let modified_annotations = self
            .detect_modified_annotations(pool, data_source_id, prev_version_id)
            .await?;
        if modified_annotations > 0 {
            entries.push(ChangelogEntry::modified(
                "annotations",
                modified_annotations,
                "Protein annotations updated (GO terms, features, etc.)",
                false, // Annotation changes are non-breaking
            ));
            entries_modified += modified_annotations;
        }

        // Create summary
        let summary = ChangelogSummary::new(
            prev_count,
            current_count,
            entries_added,
            entries_removed,
            entries_modified,
            TriggerReason::NewRelease,
        );

        // Determine bump type based on breaking changes
        let bump_type = VersionChangelog::determine_bump_type(&entries);

        let changelog = VersionChangelog::new(bump_type, entries, summary, "");
        let summary_text = changelog.generate_summary_text();

        Ok(VersionChangelog {
            summary_text,
            ..changelog
        })
    }
}

impl UniProtBumpDetector {
    /// Create initial changelog for first version
    async fn create_initial_changelog(
        &self,
        pool: &PgPool,
        data_source_id: Uuid,
    ) -> Result<VersionChangelog> {
        let count = self.get_current_protein_count(pool, data_source_id).await?;

        let entries = vec![ChangelogEntry::added(
            "proteins",
            count,
            "Initial protein import from SwissProt",
        )];

        let summary = ChangelogSummary::initial(count);
        let summary_text = format!("Initial release with {} proteins", count);

        Ok(VersionChangelog::new(BumpType::Minor, entries, summary, summary_text))
    }

    /// Get protein count for a specific version
    async fn get_protein_count(&self, pool: &PgPool, version_id: Uuid) -> Result<i64> {
        let count: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COUNT(DISTINCT pm.data_source_id)
            FROM protein_metadata pm
            JOIN versions v ON v.entry_id = pm.data_source_id
            WHERE v.id = $1
            "#,
        )
        .bind(version_id)
        .fetch_one(pool)
        .await
        .context("Failed to get protein count for version")?;

        Ok(count.unwrap_or(0))
    }

    /// Get current protein count for a data source
    async fn get_current_protein_count(&self, pool: &PgPool, data_source_id: Uuid) -> Result<i64> {
        // For aggregate sources, count dependencies
        // For individual proteins, return 1
        let count: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM protein_metadata pm
            JOIN registry_entries re ON re.id = pm.data_source_id
            WHERE re.organization_id = (
                SELECT organization_id FROM registry_entries WHERE id = $1
            )
            "#,
        )
        .bind(data_source_id)
        .fetch_one(pool)
        .await
        .context("Failed to get current protein count")?;

        Ok(count.unwrap_or(0))
    }

    /// Detect removed proteins between versions
    async fn detect_removed_proteins(
        &self,
        pool: &PgPool,
        _data_source_id: Uuid,
        prev_version_id: Uuid,
    ) -> Result<i64> {
        // Count proteins that were in previous version but not in current
        // This is a simplified implementation - in production you'd compare actual accessions
        let count: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM dependencies d
            WHERE d.version_id = $1
            AND NOT EXISTS (
                SELECT 1 FROM protein_metadata pm
                WHERE pm.data_source_id = d.depends_on_entry_id
            )
            "#,
        )
        .bind(prev_version_id)
        .fetch_one(pool)
        .await
        .context("Failed to detect removed proteins")?;

        Ok(count.unwrap_or(0))
    }

    /// Detect added proteins between versions
    async fn detect_added_proteins(
        &self,
        pool: &PgPool,
        data_source_id: Uuid,
        prev_version_id: Uuid,
    ) -> Result<i64> {
        // Count proteins in current that weren't in previous version
        let count: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM protein_metadata pm
            JOIN registry_entries re ON re.id = pm.data_source_id
            WHERE re.organization_id = (
                SELECT organization_id FROM registry_entries WHERE id = $1
            )
            AND NOT EXISTS (
                SELECT 1 FROM dependencies d
                WHERE d.version_id = $2
                AND d.depends_on_entry_id = pm.data_source_id
            )
            "#,
        )
        .bind(data_source_id)
        .bind(prev_version_id)
        .fetch_one(pool)
        .await
        .context("Failed to detect added proteins")?;

        Ok(count.unwrap_or(0))
    }

    /// Detect modified sequences between versions
    async fn detect_modified_sequences(
        &self,
        _pool: &PgPool,
        _data_source_id: Uuid,
        _prev_version_id: Uuid,
    ) -> Result<i64> {
        // In a full implementation, this would compare sequence checksums
        // between versions. For now, return 0 as we'd need sequence version tracking.
        debug!("Sequence modification detection not yet implemented");
        Ok(0)
    }

    /// Detect modified annotations between versions
    async fn detect_modified_annotations(
        &self,
        _pool: &PgPool,
        _data_source_id: Uuid,
        _prev_version_id: Uuid,
    ) -> Result<i64> {
        // In a full implementation, this would compare annotation hashes
        // between versions. For now, return 0.
        debug!("Annotation modification detection not yet implemented");
        Ok(0)
    }
}

/// NCBI Taxonomy version bump detector
///
/// Detects changes in taxonomy entries between versions:
/// - MAJOR if: taxa removed, scientific names changed, rank changed
/// - MINOR if: taxa added, common names updated, lineage refined
#[derive(Debug, Clone, Default)]
pub struct NcbiTaxonomyBumpDetector;

impl NcbiTaxonomyBumpDetector {
    /// Create a new NCBI Taxonomy bump detector
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl VersionBumpDetector for NcbiTaxonomyBumpDetector {
    fn name(&self) -> &'static str {
        "NCBI Taxonomy"
    }

    fn default_strategy(&self) -> VersioningStrategy {
        VersioningStrategy::ncbi_taxonomy()
    }

    #[instrument(skip(self, pool), fields(detector = self.name()))]
    async fn detect_changes(
        &self,
        pool: &PgPool,
        data_source_id: Uuid,
        previous_version_id: Option<Uuid>,
    ) -> Result<VersionChangelog> {
        info!(
            data_source_id = %data_source_id,
            previous_version_id = ?previous_version_id,
            "Detecting NCBI Taxonomy version changes"
        );

        // If no previous version, this is the initial release
        let Some(prev_version_id) = previous_version_id else {
            return self.create_initial_changelog(pool, data_source_id).await;
        };

        let mut entries = Vec::new();
        let mut entries_added = 0i64;
        let mut entries_removed = 0i64;
        let mut entries_modified = 0i64;

        // Get counts
        let prev_count = self.get_taxon_count(pool, prev_version_id).await?;
        let current_count = self.get_current_taxon_count(pool, data_source_id).await?;

        // Detect removed taxa (breaking - may break protein references)
        let removed_taxa = self
            .detect_removed_taxa(pool, data_source_id, prev_version_id)
            .await?;
        if removed_taxa > 0 {
            entries.push(ChangelogEntry::removed(
                "taxa",
                removed_taxa,
                "Taxonomy nodes removed or merged",
            ));
            entries_removed = removed_taxa;
        }

        // Detect added taxa (non-breaking)
        let added_taxa = self
            .detect_added_taxa(pool, data_source_id, prev_version_id)
            .await?;
        if added_taxa > 0 {
            entries.push(ChangelogEntry::added("taxa", added_taxa, "New taxonomy nodes added"));
            entries_added = added_taxa;
        }

        // Detect modified names (breaking for scientific names)
        let modified_names = self
            .detect_modified_names(pool, data_source_id, prev_version_id)
            .await?;
        if modified_names > 0 {
            entries.push(ChangelogEntry::modified(
                "names",
                modified_names,
                "Taxonomy names updated",
                true, // Name changes are breaking
            ));
            entries_modified += modified_names;
        }

        // Create summary
        let summary = ChangelogSummary::new(
            prev_count,
            current_count,
            entries_added,
            entries_removed,
            entries_modified,
            TriggerReason::NewRelease,
        );

        let bump_type = VersionChangelog::determine_bump_type(&entries);

        let changelog = VersionChangelog::new(bump_type, entries, summary, "");
        let summary_text = changelog.generate_summary_text();

        Ok(VersionChangelog {
            summary_text,
            ..changelog
        })
    }
}

impl NcbiTaxonomyBumpDetector {
    /// Create initial changelog for first version
    async fn create_initial_changelog(
        &self,
        pool: &PgPool,
        data_source_id: Uuid,
    ) -> Result<VersionChangelog> {
        let count = self.get_current_taxon_count(pool, data_source_id).await?;

        let entries =
            vec![ChangelogEntry::added("taxa", count, "Initial taxonomy import from NCBI")];

        let summary = ChangelogSummary::initial(count);
        let summary_text = format!("Initial release with {} taxonomy nodes", count);

        Ok(VersionChangelog::new(BumpType::Minor, entries, summary, summary_text))
    }

    /// Get taxon count for a specific version
    async fn get_taxon_count(&self, pool: &PgPool, version_id: Uuid) -> Result<i64> {
        let count: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM dependencies d
            WHERE d.version_id = $1
            "#,
        )
        .bind(version_id)
        .fetch_one(pool)
        .await
        .context("Failed to get taxon count for version")?;

        Ok(count.unwrap_or(0))
    }

    /// Get current taxon count for a data source
    async fn get_current_taxon_count(&self, pool: &PgPool, data_source_id: Uuid) -> Result<i64> {
        let count: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM taxonomy_metadata tm
            JOIN registry_entries re ON re.id = tm.data_source_id
            WHERE re.organization_id = (
                SELECT organization_id FROM registry_entries WHERE id = $1
            )
            "#,
        )
        .bind(data_source_id)
        .fetch_one(pool)
        .await
        .context("Failed to get current taxon count")?;

        Ok(count.unwrap_or(0))
    }

    /// Detect removed taxa between versions
    async fn detect_removed_taxa(
        &self,
        _pool: &PgPool,
        _data_source_id: Uuid,
        _prev_version_id: Uuid,
    ) -> Result<i64> {
        // Simplified implementation - would compare taxonomy_ids between versions
        debug!("Removed taxa detection not yet fully implemented");
        Ok(0)
    }

    /// Detect added taxa between versions
    async fn detect_added_taxa(
        &self,
        _pool: &PgPool,
        _data_source_id: Uuid,
        _prev_version_id: Uuid,
    ) -> Result<i64> {
        // Simplified implementation
        debug!("Added taxa detection not yet fully implemented");
        Ok(0)
    }

    /// Detect modified names between versions
    async fn detect_modified_names(
        &self,
        _pool: &PgPool,
        _data_source_id: Uuid,
        _prev_version_id: Uuid,
    ) -> Result<i64> {
        // Simplified implementation
        debug!("Modified names detection not yet fully implemented");
        Ok(0)
    }
}

/// Gene Ontology version bump detector
///
/// Detects changes in GO terms between versions:
/// - MAJOR if: terms obsoleted, term IDs reassigned
/// - MINOR if: terms added, definitions updated, relationships added
#[derive(Debug, Clone, Default)]
pub struct GeneOntologyBumpDetector;

impl GeneOntologyBumpDetector {
    /// Create a new Gene Ontology bump detector
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl VersionBumpDetector for GeneOntologyBumpDetector {
    fn name(&self) -> &'static str {
        "Gene Ontology"
    }

    fn default_strategy(&self) -> VersioningStrategy {
        VersioningStrategy::gene_ontology()
    }

    #[instrument(skip(self, pool), fields(detector = self.name()))]
    async fn detect_changes(
        &self,
        pool: &PgPool,
        data_source_id: Uuid,
        previous_version_id: Option<Uuid>,
    ) -> Result<VersionChangelog> {
        info!(
            data_source_id = %data_source_id,
            previous_version_id = ?previous_version_id,
            "Detecting Gene Ontology version changes"
        );

        // If no previous version, this is the initial release
        let Some(prev_version_id) = previous_version_id else {
            return self.create_initial_changelog(pool, data_source_id).await;
        };

        let mut entries = Vec::new();
        let mut entries_added = 0i64;
        let mut entries_removed = 0i64;
        let mut entries_modified = 0i64;

        // Get counts
        let prev_count = self.get_term_count(pool, prev_version_id).await?;
        let current_count = self.get_current_term_count(pool, data_source_id).await?;

        // Detect obsoleted terms (breaking)
        let obsoleted = self
            .detect_obsoleted_terms(pool, data_source_id, prev_version_id)
            .await?;
        if obsoleted > 0 {
            entries.push(ChangelogEntry::removed(
                "terms",
                obsoleted,
                "GO terms marked as obsolete",
            ));
            entries_removed = obsoleted;
        }

        // Detect added terms (non-breaking)
        let added = self
            .detect_added_terms(pool, data_source_id, prev_version_id)
            .await?;
        if added > 0 {
            entries.push(ChangelogEntry::added("terms", added, "New GO terms added"));
            entries_added = added;
        }

        // Detect modified definitions (non-breaking)
        let modified = self
            .detect_modified_definitions(pool, data_source_id, prev_version_id)
            .await?;
        if modified > 0 {
            entries.push(ChangelogEntry::modified(
                "definitions",
                modified,
                "GO term definitions updated",
                false,
            ));
            entries_modified = modified;
        }

        // Create summary
        let summary = ChangelogSummary::new(
            prev_count,
            current_count,
            entries_added,
            entries_removed,
            entries_modified,
            TriggerReason::NewRelease,
        );

        let bump_type = VersionChangelog::determine_bump_type(&entries);

        let changelog = VersionChangelog::new(bump_type, entries, summary, "");
        let summary_text = changelog.generate_summary_text();

        Ok(VersionChangelog {
            summary_text,
            ..changelog
        })
    }
}

impl GeneOntologyBumpDetector {
    /// Create initial changelog for first version
    async fn create_initial_changelog(
        &self,
        pool: &PgPool,
        data_source_id: Uuid,
    ) -> Result<VersionChangelog> {
        let count = self.get_current_term_count(pool, data_source_id).await?;

        let entries = vec![ChangelogEntry::added("terms", count, "Initial GO term import")];

        let summary = ChangelogSummary::initial(count);
        let summary_text = format!("Initial release with {} GO terms", count);

        Ok(VersionChangelog::new(BumpType::Minor, entries, summary, summary_text))
    }

    /// Get term count for a specific version
    async fn get_term_count(&self, pool: &PgPool, version_id: Uuid) -> Result<i64> {
        let count: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM go_metadata gm
            JOIN versions v ON v.entry_id = gm.data_source_id
            WHERE v.id = $1
            "#,
        )
        .bind(version_id)
        .fetch_one(pool)
        .await
        .context("Failed to get GO term count for version")?;

        Ok(count.unwrap_or(0))
    }

    /// Get current GO term count for a data source
    async fn get_current_term_count(&self, pool: &PgPool, data_source_id: Uuid) -> Result<i64> {
        let count: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM go_metadata gm
            JOIN registry_entries re ON re.id = gm.data_source_id
            WHERE re.organization_id = (
                SELECT organization_id FROM registry_entries WHERE id = $1
            )
            "#,
        )
        .bind(data_source_id)
        .fetch_one(pool)
        .await
        .context("Failed to get current GO term count")?;

        Ok(count.unwrap_or(0))
    }

    /// Detect obsoleted terms
    async fn detect_obsoleted_terms(
        &self,
        _pool: &PgPool,
        _data_source_id: Uuid,
        _prev_version_id: Uuid,
    ) -> Result<i64> {
        debug!("Obsoleted terms detection not yet fully implemented");
        Ok(0)
    }

    /// Detect added terms
    async fn detect_added_terms(
        &self,
        _pool: &PgPool,
        _data_source_id: Uuid,
        _prev_version_id: Uuid,
    ) -> Result<i64> {
        debug!("Added terms detection not yet fully implemented");
        Ok(0)
    }

    /// Detect modified definitions
    async fn detect_modified_definitions(
        &self,
        _pool: &PgPool,
        _data_source_id: Uuid,
        _prev_version_id: Uuid,
    ) -> Result<i64> {
        debug!("Modified definitions detection not yet fully implemented");
        Ok(0)
    }
}

/// GenBank/RefSeq version bump detector
///
/// Detects changes in sequence records between versions:
/// - MAJOR if: sequences removed, accessions changed, sequences modified
/// - MINOR if: sequences added, annotations updated
#[derive(Debug, Clone, Default)]
pub struct GenbankBumpDetector;

impl GenbankBumpDetector {
    /// Create a new GenBank bump detector
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl VersionBumpDetector for GenbankBumpDetector {
    fn name(&self) -> &'static str {
        "GenBank/RefSeq"
    }

    fn default_strategy(&self) -> VersioningStrategy {
        VersioningStrategy::genbank()
    }

    #[instrument(skip(self, pool), fields(detector = self.name()))]
    async fn detect_changes(
        &self,
        pool: &PgPool,
        data_source_id: Uuid,
        previous_version_id: Option<Uuid>,
    ) -> Result<VersionChangelog> {
        info!(
            data_source_id = %data_source_id,
            previous_version_id = ?previous_version_id,
            "Detecting GenBank/RefSeq version changes"
        );

        // If no previous version, this is the initial release
        let Some(prev_version_id) = previous_version_id else {
            return self.create_initial_changelog(pool, data_source_id).await;
        };

        let mut entries = Vec::new();
        let mut entries_added = 0i64;
        let mut entries_removed = 0i64;
        let mut entries_modified = 0i64;

        // Get counts
        let prev_count = self.get_sequence_count(pool, prev_version_id).await?;
        let current_count = self
            .get_current_sequence_count(pool, data_source_id)
            .await?;

        // Detect removed sequences (breaking)
        let removed = self
            .detect_removed_sequences(pool, data_source_id, prev_version_id)
            .await?;
        if removed > 0 {
            entries.push(ChangelogEntry::removed(
                "sequences",
                removed,
                "Sequences withdrawn or superseded",
            ));
            entries_removed = removed;
        }

        // Detect added sequences (non-breaking)
        let added = self
            .detect_added_sequences(pool, data_source_id, prev_version_id)
            .await?;
        if added > 0 {
            entries.push(ChangelogEntry::added("sequences", added, "New sequences added"));
            entries_added = added;
        }

        // Detect modified sequences (breaking)
        let modified = self
            .detect_modified_sequences(pool, data_source_id, prev_version_id)
            .await?;
        if modified > 0 {
            entries.push(ChangelogEntry::modified(
                "sequences",
                modified,
                "Sequence records updated",
                true, // Sequence modifications are breaking
            ));
            entries_modified = modified;
        }

        // Create summary
        let summary = ChangelogSummary::new(
            prev_count,
            current_count,
            entries_added,
            entries_removed,
            entries_modified,
            TriggerReason::NewRelease,
        );

        let bump_type = VersionChangelog::determine_bump_type(&entries);

        let changelog = VersionChangelog::new(bump_type, entries, summary, "");
        let summary_text = changelog.generate_summary_text();

        Ok(VersionChangelog {
            summary_text,
            ..changelog
        })
    }
}

impl GenbankBumpDetector {
    /// Create initial changelog for first version
    async fn create_initial_changelog(
        &self,
        pool: &PgPool,
        data_source_id: Uuid,
    ) -> Result<VersionChangelog> {
        let count = self
            .get_current_sequence_count(pool, data_source_id)
            .await?;

        let entries = vec![ChangelogEntry::added(
            "sequences",
            count,
            "Initial sequence import from GenBank/RefSeq",
        )];

        let summary = ChangelogSummary::initial(count);
        let summary_text = format!("Initial release with {} sequences", count);

        Ok(VersionChangelog::new(BumpType::Minor, entries, summary, summary_text))
    }

    /// Get sequence count for a specific version
    async fn get_sequence_count(&self, pool: &PgPool, version_id: Uuid) -> Result<i64> {
        let count: Option<i32> = sqlx::query_scalar(
            r#"
            SELECT dependency_count
            FROM versions
            WHERE id = $1
            "#,
        )
        .bind(version_id)
        .fetch_one(pool)
        .await
        .context("Failed to get sequence count for version")?;

        Ok(count.unwrap_or(0) as i64)
    }

    /// Get current sequence count for a data source
    async fn get_current_sequence_count(&self, pool: &PgPool, data_source_id: Uuid) -> Result<i64> {
        let count: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM dependencies d
            JOIN versions v ON v.id = d.version_id
            WHERE v.entry_id = $1
            "#,
        )
        .bind(data_source_id)
        .fetch_one(pool)
        .await
        .context("Failed to get current sequence count")?;

        Ok(count.unwrap_or(0))
    }

    /// Detect removed sequences
    async fn detect_removed_sequences(
        &self,
        _pool: &PgPool,
        _data_source_id: Uuid,
        _prev_version_id: Uuid,
    ) -> Result<i64> {
        debug!("Removed sequences detection not yet fully implemented");
        Ok(0)
    }

    /// Detect added sequences
    async fn detect_added_sequences(
        &self,
        _pool: &PgPool,
        _data_source_id: Uuid,
        _prev_version_id: Uuid,
    ) -> Result<i64> {
        debug!("Added sequences detection not yet fully implemented");
        Ok(0)
    }

    /// Detect modified sequences
    async fn detect_modified_sequences(
        &self,
        _pool: &PgPool,
        _data_source_id: Uuid,
        _prev_version_id: Uuid,
    ) -> Result<i64> {
        debug!("Modified sequences detection not yet fully implemented");
        Ok(0)
    }
}

/// Get the appropriate detector for a data source type
pub fn get_detector(source_type: &str) -> Option<Box<dyn VersionBumpDetector>> {
    match source_type {
        "protein" | "bundle" => Some(Box::new(UniProtBumpDetector::new())),
        "taxonomy" => Some(Box::new(NcbiTaxonomyBumpDetector::new())),
        "ontology" => Some(Box::new(GeneOntologyBumpDetector::new())),
        "genome" | "sequence" => Some(Box::new(GenbankBumpDetector::new())),
        _ => {
            warn!(source_type = %source_type, "No version bump detector for source type");
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_detector() {
        assert!(get_detector("protein").is_some());
        assert!(get_detector("taxonomy").is_some());
        assert!(get_detector("ontology").is_some());
        assert!(get_detector("genome").is_some());
        assert!(get_detector("unknown").is_none());
    }

    #[test]
    fn test_detector_names() {
        assert_eq!(UniProtBumpDetector::new().name(), "UniProt");
        assert_eq!(NcbiTaxonomyBumpDetector::new().name(), "NCBI Taxonomy");
        assert_eq!(GeneOntologyBumpDetector::new().name(), "Gene Ontology");
        assert_eq!(GenbankBumpDetector::new().name(), "GenBank/RefSeq");
    }
}
