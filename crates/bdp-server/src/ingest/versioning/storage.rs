//! Database operations for version changelogs
//!
//! This module provides functions for storing and retrieving version changelogs
//! from the database.

use anyhow::{Context, Result};
use sqlx::{PgPool, Row};
use tracing::{debug, instrument};
use uuid::Uuid;

use super::types::{
    BumpType, ChangelogEntry, ChangelogSummary, TriggerReason, VersionChangelog,
};

/// Store changelog in database
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `version_id` - The version ID this changelog belongs to
/// * `changelog` - The changelog to store
///
/// # Returns
/// The UUID of the created changelog record
#[instrument(skip(pool, changelog))]
pub async fn save_changelog(
    pool: &PgPool,
    version_id: Uuid,
    changelog: &VersionChangelog,
) -> Result<Uuid> {
    // Serialize entries and summary to JSON
    let entries_json = serde_json::to_value(&changelog.entries)
        .context("Failed to serialize changelog entries")?;

    let summary_json = serde_json::to_value(&changelog.summary)
        .context("Failed to serialize changelog summary")?;

    let bump_type_str = changelog.bump_type.as_db_str();
    let trigger_type_str = changelog.summary.triggered_by.as_db_str();

    let changelog_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO version_changelogs (
            version_id,
            bump_type,
            entries,
            summary,
            summary_text,
            triggered_by_version_id,
            triggered_by
        )
        VALUES (
            $1,
            $2::version_bump_type,
            $3,
            $4,
            $5,
            $6,
            $7::changelog_trigger_type
        )
        ON CONFLICT (version_id) DO UPDATE
        SET bump_type = EXCLUDED.bump_type,
            entries = EXCLUDED.entries,
            summary = EXCLUDED.summary,
            summary_text = EXCLUDED.summary_text,
            triggered_by_version_id = EXCLUDED.triggered_by_version_id,
            triggered_by = EXCLUDED.triggered_by
        RETURNING id
        "#,
    )
    .bind(version_id)
    .bind(bump_type_str)
    .bind(&entries_json)
    .bind(&summary_json)
    .bind(&changelog.summary_text)
    .bind(changelog.triggered_by_version_id)
    .bind(trigger_type_str)
    .fetch_one(pool)
    .await
    .context("Failed to save changelog")?;

    debug!(
        version_id = %version_id,
        changelog_id = %changelog_id,
        bump_type = %changelog.bump_type,
        "Saved changelog"
    );

    Ok(changelog_id)
}

/// Internal record structure for fetching changelogs
#[allow(dead_code)]
struct ChangelogRecord {
    id: Uuid,
    version_id: Uuid,
    bump_type: String,
    entries: serde_json::Value,
    summary: serde_json::Value,
    summary_text: Option<String>,
    triggered_by_version_id: Option<Uuid>,
    triggered_by: Option<String>,
}

/// Get changelog for a version
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `version_id` - The version ID to get changelog for
///
/// # Returns
/// The changelog if found, or None if no changelog exists for this version
#[instrument(skip(pool))]
pub async fn get_changelog(pool: &PgPool, version_id: Uuid) -> Result<Option<VersionChangelog>> {
    let row = sqlx::query(
        r#"
        SELECT
            id,
            version_id,
            bump_type::text as bump_type,
            entries,
            summary,
            summary_text,
            triggered_by_version_id,
            triggered_by::text as triggered_by
        FROM version_changelogs
        WHERE version_id = $1
        "#,
    )
    .bind(version_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch changelog")?;

    match row {
        Some(r) => {
            let record = ChangelogRecord {
                id: r.get("id"),
                version_id: r.get("version_id"),
                bump_type: r.get("bump_type"),
                entries: r.get("entries"),
                summary: r.get("summary"),
                summary_text: r.get("summary_text"),
                triggered_by_version_id: r.get("triggered_by_version_id"),
                triggered_by: r.get("triggered_by"),
            };
            let changelog = parse_changelog_record(record)?;
            debug!(
                version_id = %version_id,
                bump_type = %changelog.bump_type,
                "Retrieved changelog"
            );
            Ok(Some(changelog))
        }
        None => {
            debug!(version_id = %version_id, "No changelog found");
            Ok(None)
        }
    }
}

/// Get changelog by changelog ID
#[instrument(skip(pool))]
pub async fn get_changelog_by_id(pool: &PgPool, changelog_id: Uuid) -> Result<Option<VersionChangelog>> {
    let row = sqlx::query(
        r#"
        SELECT
            id,
            version_id,
            bump_type::text as bump_type,
            entries,
            summary,
            summary_text,
            triggered_by_version_id,
            triggered_by::text as triggered_by
        FROM version_changelogs
        WHERE id = $1
        "#,
    )
    .bind(changelog_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch changelog by ID")?;

    match row {
        Some(r) => {
            let record = ChangelogRecord {
                id: r.get("id"),
                version_id: r.get("version_id"),
                bump_type: r.get("bump_type"),
                entries: r.get("entries"),
                summary: r.get("summary"),
                summary_text: r.get("summary_text"),
                triggered_by_version_id: r.get("triggered_by_version_id"),
                triggered_by: r.get("triggered_by"),
            };
            Ok(Some(parse_changelog_record(record)?))
        }
        None => Ok(None),
    }
}

/// Parse a changelog record into a VersionChangelog
fn parse_changelog_record(record: ChangelogRecord) -> Result<VersionChangelog> {
    let bump_type = BumpType::from_db_str(&record.bump_type)
        .ok_or_else(|| anyhow::anyhow!("Invalid bump type: {}", record.bump_type))?;

    let entries: Vec<ChangelogEntry> = serde_json::from_value(record.entries)
        .context("Failed to parse changelog entries")?;

    let summary: ChangelogSummary = serde_json::from_value(record.summary)
        .context("Failed to parse changelog summary")?;

    Ok(VersionChangelog {
        bump_type,
        entries,
        summary,
        summary_text: record.summary_text.unwrap_or_default(),
        triggered_by_version_id: record.triggered_by_version_id,
    })
}

/// List changelogs for a data source (all versions)
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `data_source_id` - The registry entry ID
/// * `limit` - Maximum number of changelogs to return
///
/// # Returns
/// A list of (version_id, version, changelog) tuples ordered by version descending
#[instrument(skip(pool))]
pub async fn list_changelogs_for_data_source(
    pool: &PgPool,
    data_source_id: Uuid,
    limit: i64,
) -> Result<Vec<(Uuid, String, VersionChangelog)>> {
    let rows = sqlx::query(
        r#"
        SELECT
            vc.id,
            vc.version_id,
            v.version,
            vc.bump_type::text as bump_type,
            vc.entries,
            vc.summary,
            vc.summary_text,
            vc.triggered_by_version_id,
            vc.triggered_by::text as triggered_by
        FROM version_changelogs vc
        JOIN versions v ON v.id = vc.version_id
        WHERE v.entry_id = $1
        ORDER BY v.version_major DESC, v.version_minor DESC, v.version_patch DESC
        LIMIT $2
        "#,
    )
    .bind(data_source_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("Failed to list changelogs")?;

    let mut results = Vec::with_capacity(rows.len());

    for r in rows {
        let bump_type_str: String = r.get("bump_type");
        let bump_type = BumpType::from_db_str(&bump_type_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid bump type: {}", bump_type_str))?;

        let entries_val: serde_json::Value = r.get("entries");
        let entries: Vec<ChangelogEntry> = serde_json::from_value(entries_val)
            .context("Failed to parse changelog entries")?;

        let summary_val: serde_json::Value = r.get("summary");
        let summary: ChangelogSummary = serde_json::from_value(summary_val)
            .context("Failed to parse changelog summary")?;

        let changelog = VersionChangelog {
            bump_type,
            entries,
            summary,
            summary_text: r.get::<Option<String>, _>("summary_text").unwrap_or_default(),
            triggered_by_version_id: r.get("triggered_by_version_id"),
        };

        results.push((r.get("version_id"), r.get("version"), changelog));
    }

    debug!(
        data_source_id = %data_source_id,
        changelog_count = results.len(),
        "Listed changelogs for data source"
    );

    Ok(results)
}

/// Find changelogs triggered by a specific version (cascaded changes)
#[instrument(skip(pool))]
pub async fn find_cascaded_changelogs(
    pool: &PgPool,
    trigger_version_id: Uuid,
) -> Result<Vec<(Uuid, String, VersionChangelog)>> {
    let rows = sqlx::query(
        r#"
        SELECT
            vc.id,
            vc.version_id,
            re.slug as entry_slug,
            vc.bump_type::text as bump_type,
            vc.entries,
            vc.summary,
            vc.summary_text,
            vc.triggered_by_version_id,
            vc.triggered_by::text as triggered_by
        FROM version_changelogs vc
        JOIN versions v ON v.id = vc.version_id
        JOIN registry_entries re ON re.id = v.entry_id
        WHERE vc.triggered_by_version_id = $1
        ORDER BY re.slug
        "#,
    )
    .bind(trigger_version_id)
    .fetch_all(pool)
    .await
    .context("Failed to find cascaded changelogs")?;

    let mut results = Vec::with_capacity(rows.len());

    for r in rows {
        let bump_type_str: String = r.get("bump_type");
        let bump_type = BumpType::from_db_str(&bump_type_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid bump type: {}", bump_type_str))?;

        let entries_val: serde_json::Value = r.get("entries");
        let entries: Vec<ChangelogEntry> = serde_json::from_value(entries_val)
            .context("Failed to parse changelog entries")?;

        let summary_val: serde_json::Value = r.get("summary");
        let summary: ChangelogSummary = serde_json::from_value(summary_val)
            .context("Failed to parse changelog summary")?;

        let changelog = VersionChangelog {
            bump_type,
            entries,
            summary,
            summary_text: r.get::<Option<String>, _>("summary_text").unwrap_or_default(),
            triggered_by_version_id: r.get("triggered_by_version_id"),
        };

        results.push((r.get("version_id"), r.get("entry_slug"), changelog));
    }

    debug!(
        trigger_version_id = %trigger_version_id,
        cascaded_count = results.len(),
        "Found cascaded changelogs"
    );

    Ok(results)
}

/// Delete changelog for a version
#[instrument(skip(pool))]
pub async fn delete_changelog(pool: &PgPool, version_id: Uuid) -> Result<bool> {
    let result = sqlx::query(
        r#"
        DELETE FROM version_changelogs
        WHERE version_id = $1
        "#,
    )
    .bind(version_id)
    .execute(pool)
    .await
    .context("Failed to delete changelog")?;

    let deleted = result.rows_affected() > 0;
    debug!(version_id = %version_id, deleted = deleted, "Delete changelog result");

    Ok(deleted)
}

/// Count changelogs by trigger type for statistics
#[instrument(skip(pool))]
pub async fn count_changelogs_by_trigger(pool: &PgPool) -> Result<Vec<(TriggerReason, i64)>> {
    let rows = sqlx::query(
        r#"
        SELECT
            triggered_by::text as trigger_type,
            COUNT(*) as count
        FROM version_changelogs
        WHERE triggered_by IS NOT NULL
        GROUP BY triggered_by
        "#
    )
    .fetch_all(pool)
    .await
    .context("Failed to count changelogs by trigger")?;

    let results: Vec<(TriggerReason, i64)> = rows
        .into_iter()
        .filter_map(|r| {
            let trigger_type: String = r.get("trigger_type");
            let count: i64 = r.get("count");
            TriggerReason::from_db_str(&trigger_type).map(|tr| (tr, count))
        })
        .collect();

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_changelog() -> VersionChangelog {
        VersionChangelog::new(
            BumpType::Minor,
            vec![
                ChangelogEntry::added("proteins", 100, "New proteins"),
                ChangelogEntry::modified("annotations", 50, "Updated annotations", false),
            ],
            ChangelogSummary::new(1000, 1100, 100, 0, 50, TriggerReason::NewRelease),
            "Test changelog",
        )
    }

    #[test]
    fn test_changelog_serialization() {
        let changelog = sample_changelog();

        // Test entries serialization
        let entries_json = serde_json::to_value(&changelog.entries).unwrap();
        assert!(entries_json.is_array());

        // Test summary serialization
        let summary_json = serde_json::to_value(&changelog.summary).unwrap();
        assert!(summary_json.is_object());

        // Test deserialization
        let entries_back: Vec<ChangelogEntry> = serde_json::from_value(entries_json).unwrap();
        assert_eq!(entries_back.len(), 2);

        let summary_back: ChangelogSummary = serde_json::from_value(summary_json).unwrap();
        assert_eq!(summary_back.entries_added, 100);
    }

    #[test]
    fn test_parse_changelog_record() {
        let record = ChangelogRecord {
            id: Uuid::new_v4(),
            version_id: Uuid::new_v4(),
            bump_type: "minor".to_string(),
            entries: serde_json::json!([
                {"change_type": "added", "category": "proteins", "count": 100, "description": "New", "is_breaking": false}
            ]),
            summary: serde_json::json!({
                "total_entries_before": 1000,
                "total_entries_after": 1100,
                "entries_added": 100,
                "entries_removed": 0,
                "entries_modified": 0,
                "triggered_by": "new_release"
            }),
            summary_text: Some("Test".to_string()),
            triggered_by_version_id: None,
            triggered_by: Some("new_release".to_string()),
        };

        let changelog = parse_changelog_record(record).unwrap();
        assert_eq!(changelog.bump_type, BumpType::Minor);
        assert_eq!(changelog.entries.len(), 1);
        assert_eq!(changelog.summary.entries_added, 100);
    }
}
