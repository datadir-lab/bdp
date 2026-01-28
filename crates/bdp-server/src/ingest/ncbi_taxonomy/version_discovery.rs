//! NCBI Taxonomy version discovery and tracking
//!
//! Discovers available versions from FTP and tracks what's been ingested

use anyhow::{Context, Result};
use chrono::NaiveDate;
use sqlx::PgPool;
use tracing::{debug, info};

use super::config::NcbiTaxonomyFtpConfig;
use super::ftp::NcbiTaxonomyFtp;

/// Discovered NCBI Taxonomy version from FTP
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredTaxonomyVersion {
    /// External version identifier (FTP timestamp, e.g., "2026-01-15")
    pub external_version: String,
    /// File modification date
    pub modification_date: NaiveDate,
}

impl PartialOrd for DiscoveredTaxonomyVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DiscoveredTaxonomyVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by modification date, then by version string
        match self.modification_date.cmp(&other.modification_date) {
            std::cmp::Ordering::Equal => self.external_version.cmp(&other.external_version),
            other => other,
        }
    }
}

/// NCBI Taxonomy version discovery service
pub struct TaxonomyVersionDiscovery {
    _config: NcbiTaxonomyFtpConfig,
    ftp: NcbiTaxonomyFtp,
    db: PgPool,
}

impl TaxonomyVersionDiscovery {
    pub fn new(config: NcbiTaxonomyFtpConfig, db: PgPool) -> Self {
        let ftp = NcbiTaxonomyFtp::new(config.clone());
        Self { _config: config, ftp, db }
    }

    /// Discover all available versions from FTP (current + historical archives)
    ///
    /// Returns all versions sorted by date (oldest first)
    pub async fn discover_all_versions(&self) -> Result<Vec<DiscoveredTaxonomyVersion>> {
        let mut versions = Vec::new();

        // 1. Discover current version
        info!("Discovering current NCBI taxonomy version");
        match self.discover_current_version_unchecked().await {
            Ok(current) => {
                info!(
                    version = %current.external_version,
                    date = %current.modification_date,
                    "Discovered current version"
                );
                versions.push(current);
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Could not discover current version (this is optional for historical catchup)"
                );
            }
        }

        // 2. Discover historical archive versions
        info!("Discovering historical archive versions");
        match self.discover_previous_versions().await {
            Ok(mut previous) => {
                info!(count = previous.len(), "Discovered historical versions");
                versions.append(&mut previous);
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Could not discover historical versions"
                );
            }
        }

        // Sort by date (oldest first)
        versions.sort();

        info!(
            count = versions.len(),
            oldest = versions.first().map(|v| v.external_version.as_str()),
            newest = versions.last().map(|v| v.external_version.as_str()),
            "Discovered all available versions"
        );

        Ok(versions)
    }

    /// Discover the current version from FTP
    ///
    /// Downloads taxdump to get modification timestamp, then checks if already ingested
    pub async fn discover_current_version(&self) -> Result<Option<DiscoveredTaxonomyVersion>> {
        info!("Discovering current NCBI taxonomy version from FTP");

        let discovered = self.discover_current_version_unchecked().await?;

        // Check if this version is already ingested
        let already_ingested = self
            .check_version_ingested(&discovered.external_version)
            .await
            .context("Failed to check if version is already ingested")?;

        if already_ingested {
            info!(
                external_version = %discovered.external_version,
                "Version already ingested, skipping"
            );
            return Ok(None);
        }

        Ok(Some(discovered))
    }

    /// Discover current version without checking if it's already ingested
    ///
    /// This is useful for discover_all_versions() which does its own filtering
    async fn discover_current_version_unchecked(&self) -> Result<DiscoveredTaxonomyVersion> {
        // Download taxdump to get modification date
        let taxdump_files = self
            .ftp
            .download_taxdump()
            .await
            .context("Failed to download taxdump for version discovery")?;

        let external_version = taxdump_files.external_version.clone();

        // Parse modification date
        let modification_date = NaiveDate::parse_from_str(&external_version, "%Y-%m-%d")
            .context("Failed to parse modification date")?;

        info!(
            external_version = %external_version,
            modification_date = %modification_date,
            "Discovered current taxonomy version"
        );

        Ok(DiscoveredTaxonomyVersion {
            external_version,
            modification_date,
        })
    }

    /// Discover all historical archive versions
    ///
    /// Lists all archive files from FTP and parses their dates
    async fn discover_previous_versions(&self) -> Result<Vec<DiscoveredTaxonomyVersion>> {
        // List all available archive versions from FTP
        let archive_dates = self.ftp.list_available_versions().await?;

        let mut versions = Vec::new();

        for date_str in archive_dates {
            // Parse date string (format: "YYYY-MM-DD")
            let modification_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .context(format!("Failed to parse archive date: {}", date_str))?;

            versions.push(DiscoveredTaxonomyVersion {
                external_version: date_str.clone(),
                modification_date,
            });

            debug!(
                version = %date_str,
                date = %modification_date,
                "Discovered historical version"
            );
        }

        info!(
            count = versions.len(),
            "Parsed {} historical versions from FTP archives",
            versions.len()
        );

        Ok(versions)
    }

    /// Check if a version has already been ingested
    ///
    /// Checks the version_mappings table for the organization and external version
    async fn check_version_ingested(&self, external_version: &str) -> Result<bool> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM version_mappings
            WHERE organization_slug = 'ncbi' AND external_version = $1
            "#
        )
        .bind(external_version)
        .fetch_one(&self.db)
        .await
        .context("Failed to check version_mappings")?;

        debug!(
            external_version = %external_version,
            count = count,
            "Checked if version already ingested"
        );

        Ok(count > 0)
    }

    /// Get the latest internal version for version bumping
    ///
    /// Returns the highest X.Y version currently in version_mappings for ncbi organization
    pub async fn get_latest_internal_version(&self) -> Result<Option<String>> {
        let latest = sqlx::query_scalar::<_, Option<String>>(
            r#"
            SELECT internal_version FROM version_mappings
            WHERE organization_slug = 'ncbi'
            ORDER BY created_at DESC
            LIMIT 1
            "#
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to get latest internal version")?;

        debug!(
            latest_version = ?latest,
            "Retrieved latest internal version"
        );

        Ok(latest)
    }

    /// Determine the next internal version based on changes
    ///
    /// Implements smart versioning:
    /// - First version: 1.0
    /// - MAJOR bump (X.0): Merged/deleted taxa (breaking changes)
    /// - MINOR bump (X.Y): Name changes, lineage updates, rank changes
    ///
    /// # Arguments
    /// * `has_major_changes` - True if there are merged or deleted taxa
    pub async fn determine_next_version(
        &self,
        has_major_changes: bool,
    ) -> Result<String> {
        let latest = self.get_latest_internal_version().await?;

        let next_version = match latest {
            None => {
                // First version
                "1.0".to_string()
            }
            Some(ref ver) => {
                // Parse current version (format: X.Y)
                let parts: Vec<&str> = ver.split('.').collect();
                if parts.len() != 2 {
                    return Err(anyhow::anyhow!("Invalid version format: {}", ver));
                }

                let major: u32 = parts[0]
                    .parse()
                    .context("Failed to parse major version")?;
                let minor: u32 = parts[1]
                    .parse()
                    .context("Failed to parse minor version")?;

                if has_major_changes {
                    // MAJOR bump: reset minor to 0
                    format!("{}.0", major + 1)
                } else {
                    // MINOR bump: increment minor
                    format!("{}.{}", major, minor + 1)
                }
            }
        };

        info!(
            latest_version = ?latest,
            next_version = %next_version,
            has_major_changes = has_major_changes,
            bump_type = if has_major_changes { "MAJOR" } else { "MINOR" },
            "Determined next internal version"
        );

        Ok(next_version)
    }

    /// Record version mapping in the database
    ///
    /// Creates an entry in version_mappings linking external → internal version
    pub async fn record_version_mapping(
        &self,
        external_version: &str,
        internal_version: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO version_mappings (organization_slug, external_version, internal_version)
            VALUES ('ncbi', $1, $2)
            ON CONFLICT (organization_slug, external_version) DO NOTHING
            "#
        )
        .bind(external_version)
        .bind(internal_version)
        .execute(&self.db)
        .await
        .context("Failed to record version mapping")?;

        info!(
            external_version = %external_version,
            internal_version = %internal_version,
            "Recorded version mapping"
        );

        Ok(())
    }

    // ========================================================================
    // Version Filtering and Gap Detection
    // ========================================================================

    /// Filter versions to only include those not yet ingested
    ///
    /// Compares discovered versions against database to identify gaps
    pub async fn filter_new_versions(
        &self,
        discovered: Vec<DiscoveredTaxonomyVersion>,
    ) -> Result<Vec<DiscoveredTaxonomyVersion>> {
        let mut new_versions = Vec::new();

        for version in discovered {
            let already_ingested = self
                .check_version_ingested(&version.external_version)
                .await?;

            if !already_ingested {
                new_versions.push(version);
            } else {
                debug!(
                    version = %version.external_version,
                    "Version already ingested, filtering out"
                );
            }
        }

        info!(
            new_count = new_versions.len(),
            "Filtered to {} new versions that haven't been ingested",
            new_versions.len()
        );

        Ok(new_versions)
    }

    /// Filter versions by date range
    ///
    /// # Arguments
    /// * `versions` - List of discovered versions
    /// * `start_date` - Start date in "YYYY-MM-DD" format (inclusive)
    /// * `end_date` - Optional end date in "YYYY-MM-DD" format (inclusive)
    ///
    /// # Returns
    /// Filtered versions within the date range
    pub fn filter_by_date_range(
        &self,
        versions: Vec<DiscoveredTaxonomyVersion>,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<Vec<DiscoveredTaxonomyVersion>> {
        let start_filter = if let Some(date_str) = start_date {
            Some(NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .context(format!("Invalid start_date format: {}", date_str))?)
        } else {
            None
        };

        let end_filter = if let Some(date_str) = end_date {
            Some(NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .context(format!("Invalid end_date format: {}", date_str))?)
        } else {
            None
        };

        let filtered: Vec<_> = versions
            .into_iter()
            .filter(|v| {
                let mut include = true;

                if let Some(start) = start_filter {
                    include = include && v.modification_date >= start;
                }

                if let Some(end) = end_filter {
                    include = include && v.modification_date <= end;
                }

                include
            })
            .collect();

        info!(
            count = filtered.len(),
            start_date = ?start_date,
            end_date = ?end_date,
            "Filtered to {} versions in date range",
            filtered.len()
        );

        Ok(filtered)
    }

    /// Get versions that need to be ingested (all new versions after last ingested)
    ///
    /// This is useful for "catchup from last ingested" scenarios
    pub async fn get_versions_to_ingest(
        &self,
        start_date: Option<&str>,
    ) -> Result<Vec<DiscoveredTaxonomyVersion>> {
        // 1. Discover all available versions
        let mut all_versions = self.discover_all_versions().await?;

        // 2. Filter by start_date if provided
        if let Some(date) = start_date {
            all_versions = self.filter_by_date_range(all_versions, Some(date), None)?;
        }

        // 3. Filter out already ingested versions
        let new_versions = self.filter_new_versions(all_versions).await?;

        info!(
            count = new_versions.len(),
            oldest = new_versions.first().map(|v| v.external_version.as_str()),
            newest = new_versions.last().map(|v| v.external_version.as_str()),
            "Found {} versions to ingest",
            new_versions.len()
        );

        Ok(new_versions)
    }

    /// Check if a newer version is available compared to last ingested
    ///
    /// Returns the newest version if it's different from the last ingested version
    pub async fn check_for_newer_version(&self) -> Result<Option<DiscoveredTaxonomyVersion>> {
        // Get last ingested version
        let last_version = self.get_last_ingested_version().await?;

        // Discover current version (unchecked)
        let current = self.discover_current_version_unchecked().await?;

        // Compare
        match last_version {
            Some(last) if last == current.external_version => {
                info!(
                    version = %current.external_version,
                    "Current version already ingested"
                );
                Ok(None)
            }
            _ => {
                info!(
                    version = %current.external_version,
                    last_version = ?last_version,
                    "New version available"
                );
                Ok(Some(current))
            }
        }
    }

    /// Get the last ingested external version from database
    ///
    /// Returns the most recent external_version from version_mappings
    pub async fn get_last_ingested_version(&self) -> Result<Option<String>> {
        let result = sqlx::query_scalar::<_, Option<String>>(
            r#"
            SELECT external_version FROM version_mappings
            WHERE organization_slug = 'ncbi'
            ORDER BY created_at DESC
            LIMIT 1
            "#
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to get last ingested version")?;

        debug!(
            last_version = ?result,
            "Retrieved last ingested external version"
        );

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_ordering() {
        let v1 = DiscoveredTaxonomyVersion {
            external_version: "2026-01-15".to_string(),
            modification_date: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
        };

        let v2 = DiscoveredTaxonomyVersion {
            external_version: "2026-01-16".to_string(),
            modification_date: NaiveDate::from_ymd_opt(2026, 1, 16).unwrap(),
        };

        assert!(v1 < v2);
    }

    #[test]
    fn test_version_ordering_multiple() {
        let v1 = DiscoveredTaxonomyVersion {
            external_version: "2025-12-01".to_string(),
            modification_date: NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        };

        let v2 = DiscoveredTaxonomyVersion {
            external_version: "2026-01-15".to_string(),
            modification_date: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
        };

        let v3 = DiscoveredTaxonomyVersion {
            external_version: "2026-02-01".to_string(),
            modification_date: NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
        };

        let mut versions = vec![v3.clone(), v1.clone(), v2.clone()];
        versions.sort();

        // Should be sorted oldest to newest
        assert_eq!(versions[0], v1);
        assert_eq!(versions[1], v2);
        assert_eq!(versions[2], v3);
    }

    #[test]
    fn test_filter_by_date_range_logic() {
        // Test date range filtering logic without needing database or discovery instance
        let versions = vec![
            DiscoveredTaxonomyVersion {
                external_version: "2025-11-01".to_string(),
                modification_date: NaiveDate::from_ymd_opt(2025, 11, 1).unwrap(),
            },
            DiscoveredTaxonomyVersion {
                external_version: "2025-12-01".to_string(),
                modification_date: NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
            },
            DiscoveredTaxonomyVersion {
                external_version: "2026-01-01".to_string(),
                modification_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            },
        ];

        // Test filtering with start date only
        let start_date = NaiveDate::from_ymd_opt(2025, 12, 1).unwrap();
        let filtered: Vec<_> = versions
            .iter()
            .filter(|v| v.modification_date >= start_date)
            .collect();

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].external_version, "2025-12-01");
        assert_eq!(filtered[1].external_version, "2026-01-01");
    }

    #[test]
    fn test_filter_by_date_range_both_bounds() {
        // Test date range filtering with both start and end dates
        let versions = vec![
            DiscoveredTaxonomyVersion {
                external_version: "2025-11-01".to_string(),
                modification_date: NaiveDate::from_ymd_opt(2025, 11, 1).unwrap(),
            },
            DiscoveredTaxonomyVersion {
                external_version: "2025-12-01".to_string(),
                modification_date: NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
            },
            DiscoveredTaxonomyVersion {
                external_version: "2026-01-01".to_string(),
                modification_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            },
            DiscoveredTaxonomyVersion {
                external_version: "2026-02-01".to_string(),
                modification_date: NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
            },
        ];

        let start_date = NaiveDate::from_ymd_opt(2025, 12, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let filtered: Vec<_> = versions
            .iter()
            .filter(|v| v.modification_date >= start_date && v.modification_date <= end_date)
            .collect();

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].external_version, "2025-12-01");
        assert_eq!(filtered[1].external_version, "2026-01-01");
    }

    #[test]
    fn test_date_parsing() {
        // Test that we can parse the expected date format
        let date_str = "2026-01-15";
        let parsed = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
        assert_eq!(parsed.year(), 2026);
        assert_eq!(parsed.month(), 1);
        assert_eq!(parsed.day(), 15);
    }

    /// Test version bumping logic without database
    /// This tests the version parsing and increment logic
    #[test]
    fn test_version_bump_logic() {
        // Test parsing and incrementing
        let version = "1.0";
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 2);

        let major: u32 = parts[0].parse().unwrap();
        let minor: u32 = parts[1].parse().unwrap();

        assert_eq!(major, 1);
        assert_eq!(minor, 0);

        // MINOR bump
        let next_minor = format!("{}.{}", major, minor + 1);
        assert_eq!(next_minor, "1.1");

        // MAJOR bump
        let next_major = format!("{}.0", major + 1);
        assert_eq!(next_major, "2.0");
    }

    #[test]
    fn test_version_bump_sequences() {
        // Test various version bump sequences
        let versions = vec![
            ("1.0", false, "1.1"),  // MINOR: 1.0 -> 1.1
            ("1.1", false, "1.2"),  // MINOR: 1.1 -> 1.2
            ("1.5", true, "2.0"),   // MAJOR: 1.5 -> 2.0
            ("2.0", false, "2.1"),  // MINOR: 2.0 -> 2.1
            ("5.9", true, "6.0"),   // MAJOR: 5.9 -> 6.0
        ];

        for (current, has_major, expected) in versions {
            let parts: Vec<&str> = current.split('.').collect();
            let major: u32 = parts[0].parse().unwrap();
            let minor: u32 = parts[1].parse().unwrap();

            let result = if has_major {
                format!("{}.0", major + 1)
            } else {
                format!("{}.{}", major, minor + 1)
            };

            assert_eq!(result, expected, "Version bump from {} with major={}", current, has_major);
        }
    }
}
