//! Dependency cascade logic
//!
//! This module handles cascading version bumps to dependent data sources
//! when an upstream dependency is updated.

use anyhow::{Context, Result};
use sqlx::{PgPool, Row};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use super::calculator::{calculate_next_version, create_version};
use super::storage::save_changelog;
use super::types::{
    BumpType, CascadeResult, ChangelogEntry, ChangelogSummary, DataSourceDependent, TriggerReason,
    VersionChangelog,
};

/// Find all data sources that depend on the given version
///
/// This traverses the dependency graph to find all direct dependents
/// of the specified version.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `version_id` - The version ID to find dependents for
///
/// # Returns
/// A list of data sources that depend on this version
#[instrument(skip(pool))]
pub async fn find_dependents(pool: &PgPool, version_id: Uuid) -> Result<Vec<DataSourceDependent>> {
    // First, get the entry_id for this version
    let entry_id: Uuid = sqlx::query_scalar(
        r#"
        SELECT entry_id
        FROM versions
        WHERE id = $1
        "#,
    )
    .bind(version_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch version entry_id")?
    .ok_or_else(|| anyhow::anyhow!("Version {} not found", version_id))?;

    // Find all versions that have a dependency on this entry
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT
            re.id as entry_id,
            v.id as version_id,
            re.slug,
            re.name,
            v.version as current_version,
            o.slug as organization_slug
        FROM dependencies d
        JOIN versions v ON v.id = d.version_id
        JOIN registry_entries re ON re.id = v.entry_id
        JOIN organizations o ON o.id = re.organization_id
        WHERE d.depends_on_entry_id = $1
        AND v.id = (
            -- Get the latest version for each entry
            SELECT v2.id
            FROM versions v2
            WHERE v2.entry_id = re.id
            ORDER BY v2.version_major DESC, v2.version_minor DESC, v2.version_patch DESC
            LIMIT 1
        )
        ORDER BY re.slug
        "#,
    )
    .bind(entry_id)
    .fetch_all(pool)
    .await
    .context("Failed to find dependents")?;

    let result: Vec<DataSourceDependent> = rows
        .into_iter()
        .map(|r| DataSourceDependent {
            entry_id: r.get("entry_id"),
            version_id: r.get("version_id"),
            slug: r.get("slug"),
            name: r.get("name"),
            current_version: r.get("current_version"),
            organization_slug: r.get("organization_slug"),
        })
        .collect();

    debug!(
        version_id = %version_id,
        dependent_count = result.len(),
        "Found dependents"
    );

    Ok(result)
}

/// Find all data sources that depend on the given entry (any version)
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `entry_id` - The registry entry ID to find dependents for
///
/// # Returns
/// A list of data sources that depend on this entry
#[instrument(skip(pool))]
pub async fn find_dependents_by_entry(
    pool: &PgPool,
    entry_id: Uuid,
) -> Result<Vec<DataSourceDependent>> {
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT
            re.id as entry_id,
            v.id as version_id,
            re.slug,
            re.name,
            v.version as current_version,
            o.slug as organization_slug
        FROM dependencies d
        JOIN versions v ON v.id = d.version_id
        JOIN registry_entries re ON re.id = v.entry_id
        JOIN organizations o ON o.id = re.organization_id
        WHERE d.depends_on_entry_id = $1
        AND v.id = (
            -- Get the latest version for each entry
            SELECT v2.id
            FROM versions v2
            WHERE v2.entry_id = re.id
            ORDER BY v2.version_major DESC, v2.version_minor DESC, v2.version_patch DESC
            LIMIT 1
        )
        ORDER BY re.slug
        "#,
    )
    .bind(entry_id)
    .fetch_all(pool)
    .await
    .context("Failed to find dependents by entry")?;

    let result: Vec<DataSourceDependent> = rows
        .into_iter()
        .map(|r| DataSourceDependent {
            entry_id: r.get("entry_id"),
            version_id: r.get("version_id"),
            slug: r.get("slug"),
            name: r.get("name"),
            current_version: r.get("current_version"),
            organization_slug: r.get("organization_slug"),
        })
        .collect();

    debug!(
        entry_id = %entry_id,
        dependent_count = result.len(),
        "Found dependents by entry"
    );

    Ok(result)
}

/// Create new versions for all dependents with updated dependency
///
/// This function cascades a version bump through the dependency graph,
/// creating new versions for all data sources that depend on the updated version.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `source_version_id` - The version that was updated (triggering cascade)
/// * `source_changelog` - The changelog from the source update
///
/// # Returns
/// A list of new version IDs created for dependents
#[instrument(skip(pool, source_changelog))]
pub async fn cascade_version_bump(
    pool: &PgPool,
    source_version_id: Uuid,
    source_changelog: &VersionChangelog,
) -> Result<Vec<CascadeResult>> {
    info!(
        source_version_id = %source_version_id,
        source_bump_type = %source_changelog.bump_type,
        "Starting version cascade"
    );

    // Find all dependents of this version
    let dependents = find_dependents(pool, source_version_id).await?;

    if dependents.is_empty() {
        debug!("No dependents found, cascade complete");
        return Ok(Vec::new());
    }

    info!(
        dependent_count = dependents.len(),
        "Found dependents to cascade"
    );

    // Get source version info for changelog
    let source_row = sqlx::query(
        r#"
        SELECT re.slug, v.version
        FROM versions v
        JOIN registry_entries re ON re.id = v.entry_id
        WHERE v.id = $1
        "#,
    )
    .bind(source_version_id)
    .fetch_one(pool)
    .await
    .context("Failed to fetch source version info")?;

    struct SourceInfo {
        slug: String,
        version: String,
    }
    let source_info = SourceInfo {
        slug: source_row.get("slug"),
        version: source_row.get("version"),
    };

    let mut cascade_results = Vec::new();

    for dependent in dependents {
        match cascade_single_dependent(
            pool,
            &dependent,
            source_version_id,
            source_changelog,
            &source_info.slug,
            &source_info.version,
        )
        .await
        {
            Ok(result) => {
                info!(
                    dependent_slug = %dependent.slug,
                    new_version = %result.new_version,
                    "Cascaded version bump"
                );
                cascade_results.push(result);
            }
            Err(e) => {
                warn!(
                    dependent_slug = %dependent.slug,
                    error = %e,
                    "Failed to cascade version bump to dependent"
                );
                // Continue with other dependents even if one fails
            }
        }
    }

    info!(
        cascaded_count = cascade_results.len(),
        "Version cascade complete"
    );

    Ok(cascade_results)
}

/// Cascade version bump to a single dependent
async fn cascade_single_dependent(
    pool: &PgPool,
    dependent: &DataSourceDependent,
    source_version_id: Uuid,
    source_changelog: &VersionChangelog,
    source_slug: &str,
    source_version: &str,
) -> Result<CascadeResult> {
    // Calculate new version for dependent
    // If source had breaking changes, dependent also gets major bump
    let bump_type = if source_changelog.has_breaking_changes() {
        BumpType::Major
    } else {
        BumpType::Minor
    };

    let new_version = calculate_next_version(&dependent.current_version, bump_type);

    debug!(
        dependent_slug = %dependent.slug,
        current_version = %dependent.current_version,
        new_version = %new_version,
        bump_type = %bump_type,
        "Calculating cascade version"
    );

    // Create new version for dependent
    let new_version_id = create_version(pool, dependent.entry_id, &new_version, None).await?;

    // Create changelog entry for dependent
    let changelog = create_cascade_changelog(
        bump_type,
        source_slug,
        source_version,
        source_version_id,
        source_changelog,
    );

    // Save changelog
    let changelog_id = save_changelog(pool, new_version_id, &changelog).await?;

    // Copy dependencies from previous version to new version, updating the cascaded one
    copy_dependencies_with_update(
        pool,
        dependent.version_id,
        new_version_id,
        source_version_id,
        &new_version,
    )
    .await?;

    Ok(CascadeResult {
        entry_id: dependent.entry_id,
        entry_slug: dependent.slug.clone(),
        new_version_id,
        new_version,
        changelog_id,
    })
}

/// Create a changelog for a cascaded version bump
fn create_cascade_changelog(
    bump_type: BumpType,
    source_slug: &str,
    source_version: &str,
    source_version_id: Uuid,
    source_changelog: &VersionChangelog,
) -> VersionChangelog {
    let description = format!(
        "Updated dependency {} to version {}",
        source_slug, source_version
    );

    let entries = vec![ChangelogEntry::dependency(
        "dependencies",
        description.clone(),
        source_changelog.has_breaking_changes(),
    )];

    let summary = ChangelogSummary::new(
        0,
        0,
        0,
        0,
        0,
        TriggerReason::UpstreamDependency,
    );

    let summary_text = format!(
        "Dependency update ({} version bump): {}",
        bump_type, description
    );

    VersionChangelog::from_dependency(bump_type, entries, summary, summary_text, source_version_id)
}

/// Copy dependencies from old version to new version, updating the cascaded dependency
async fn copy_dependencies_with_update(
    pool: &PgPool,
    old_version_id: Uuid,
    new_version_id: Uuid,
    updated_entry_version_id: Uuid,
    new_dep_version: &str,
) -> Result<()> {
    // Get the entry_id of the updated dependency
    let updated_entry_id: Uuid = sqlx::query_scalar(
        r#"
        SELECT entry_id
        FROM versions
        WHERE id = $1
        "#,
    )
    .bind(updated_entry_version_id)
    .fetch_one(pool)
    .await
    .context("Failed to get updated entry ID")?;

    // Copy all dependencies from old version to new version
    sqlx::query(
        r#"
        INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version, dependency_type)
        SELECT $2, depends_on_entry_id,
            CASE
                WHEN depends_on_entry_id = $4 THEN $5
                ELSE depends_on_version
            END,
            dependency_type
        FROM dependencies
        WHERE version_id = $1
        ON CONFLICT (version_id, depends_on_entry_id) DO UPDATE
        SET depends_on_version = EXCLUDED.depends_on_version
        "#,
    )
    .bind(old_version_id)
    .bind(new_version_id)
    .bind(updated_entry_version_id) // unused in query but kept for clarity
    .bind(updated_entry_id)
    .bind(new_dep_version)
    .execute(pool)
    .await
    .context("Failed to copy dependencies")?;

    // Update dependency count on new version
    let dep_count: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) as count
        FROM dependencies
        WHERE version_id = $1
        "#,
    )
    .bind(new_version_id)
    .fetch_one(pool)
    .await?;

    sqlx::query(
        r#"
        UPDATE versions
        SET dependency_count = $2
        WHERE id = $1
        "#,
    )
    .bind(new_version_id)
    .bind(dep_count.unwrap_or(0) as i32)
    .execute(pool)
    .await
    .context("Failed to update dependency count")?;

    debug!(
        old_version_id = %old_version_id,
        new_version_id = %new_version_id,
        dependency_count = dep_count.unwrap_or(0),
        "Copied dependencies with update"
    );

    Ok(())
}

/// Recursively cascade version bumps through the dependency graph
///
/// This performs a breadth-first cascade, creating new versions for all
/// transitive dependents.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `source_version_id` - The initial version that was updated
/// * `source_changelog` - The changelog from the initial update
/// * `max_depth` - Maximum depth to cascade (prevents infinite loops)
///
/// # Returns
/// All cascade results from all levels
#[instrument(skip(pool, source_changelog))]
pub async fn cascade_recursive(
    pool: &PgPool,
    source_version_id: Uuid,
    source_changelog: &VersionChangelog,
    max_depth: usize,
) -> Result<Vec<CascadeResult>> {
    if max_depth == 0 {
        warn!("Max cascade depth reached, stopping recursion");
        return Ok(Vec::new());
    }

    let mut all_results = Vec::new();

    // Cascade to direct dependents
    let direct_results = cascade_version_bump(pool, source_version_id, source_changelog).await?;

    // For each cascaded version, recursively cascade to their dependents
    for result in &direct_results {
        // Create a minimal changelog for the recursive cascade
        let cascade_changelog = VersionChangelog::new(
            source_changelog.bump_type,
            vec![ChangelogEntry::dependency(
                "dependencies",
                format!("Cascaded from {}", result.entry_slug),
                source_changelog.has_breaking_changes(),
            )],
            ChangelogSummary::default(),
            "Cascaded dependency update",
        );

        let nested_results = Box::pin(cascade_recursive(
            pool,
            result.new_version_id,
            &cascade_changelog,
            max_depth - 1,
        ))
        .await?;

        all_results.extend(nested_results);
    }

    all_results.extend(direct_results);
    Ok(all_results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_cascade_changelog() {
        let source_changelog = VersionChangelog::new(
            BumpType::Major,
            vec![ChangelogEntry::removed("proteins", 10, "Removed proteins")],
            ChangelogSummary::default(),
            "Test",
        );

        let changelog = create_cascade_changelog(
            BumpType::Major,
            "uniprot",
            "1.0",
            Uuid::new_v4(),
            &source_changelog,
        );

        assert_eq!(changelog.bump_type, BumpType::Major);
        assert!(changelog.triggered_by_version_id.is_some());
        assert_eq!(changelog.entries.len(), 1);
        assert!(changelog.entries[0].is_breaking);
    }

    #[test]
    fn test_cascade_changelog_non_breaking() {
        let source_changelog = VersionChangelog::new(
            BumpType::Minor,
            vec![ChangelogEntry::added("proteins", 100, "Added proteins")],
            ChangelogSummary::default(),
            "Test",
        );

        let changelog = create_cascade_changelog(
            BumpType::Minor,
            "uniprot",
            "1.1",
            Uuid::new_v4(),
            &source_changelog,
        );

        assert_eq!(changelog.bump_type, BumpType::Minor);
        assert!(!changelog.entries[0].is_breaking);
    }
}
