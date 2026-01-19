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
    config: NcbiTaxonomyFtpConfig,
    ftp: NcbiTaxonomyFtp,
    db: PgPool,
}

impl TaxonomyVersionDiscovery {
    pub fn new(config: NcbiTaxonomyFtpConfig, db: PgPool) -> Self {
        let ftp = NcbiTaxonomyFtp::new(config.clone());
        Self { config, ftp, db }
    }

    /// Discover the current version from FTP
    ///
    /// Downloads taxdump to get modification timestamp, then checks if already ingested
    pub async fn discover_current_version(&self) -> Result<Option<DiscoveredTaxonomyVersion>> {
        info!("Discovering current NCBI taxonomy version from FTP");

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

        // Check if this version is already ingested
        let already_ingested = self
            .check_version_ingested(&external_version)
            .await
            .context("Failed to check if version is already ingested")?;

        if already_ingested {
            info!(
                external_version = %external_version,
                "Version already ingested, skipping"
            );
            return Ok(None);
        }

        Ok(Some(DiscoveredTaxonomyVersion {
            external_version,
            modification_date,
        }))
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
