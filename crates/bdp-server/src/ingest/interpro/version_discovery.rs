//! InterPro version discovery and tracking
//!
//! Discovers available versions from FTP and tracks what's been ingested

use anyhow::{Context, Result};
use chrono::NaiveDate;
use regex::Regex;
use sqlx::PgPool;
use uuid::Uuid;

use super::config::InterProConfig;
use super::ftp::InterProFtpDownloader;
use crate::ingest::common::version_discovery::{
    DiscoveredVersion as DiscoveredVersionTrait, VersionFilter,
};

/// Discovered InterPro version from FTP
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredVersion {
    /// External version identifier (e.g., "96.0", "97.0")
    pub external_version: String,
    /// Major version number (e.g., 96)
    pub major: u32,
    /// Minor version number (e.g., 0)
    pub minor: u32,
    /// Estimated release date (first day of month for historical versions)
    pub release_date: NaiveDate,
    /// Whether this is the current release
    pub is_current: bool,
    /// FTP directory name
    pub ftp_directory: String,
}

impl DiscoveredVersionTrait for DiscoveredVersion {
    fn external_version(&self) -> &str {
        &self.external_version
    }

    fn release_date(&self) -> NaiveDate {
        self.release_date
    }
}

// InterPro uses custom ordering by major.minor version instead of date
impl PartialOrd for DiscoveredVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DiscoveredVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by major.minor version, then by release date
        // InterPro version numbers are more reliable than estimated dates
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => match self.minor.cmp(&other.minor) {
                std::cmp::Ordering::Equal => self.release_date.cmp(&other.release_date),
                other => other,
            },
            other => other,
        }
    }
}

impl DiscoveredVersion {
    /// Parse version string into major.minor components
    ///
    /// Examples:
    /// - "96.0" -> (96, 0)
    /// - "97.0" -> (97, 0)
    /// - "100.0" -> (100, 0)
    pub fn parse_version(version_str: &str) -> Result<(u32, u32)> {
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid version format: {}. Expected MAJOR.MINOR", version_str);
        }

        let major = parts[0]
            .parse::<u32>()
            .with_context(|| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1]
            .parse::<u32>()
            .with_context(|| format!("Invalid minor version: {}", parts[1]))?;

        Ok((major, minor))
    }

    /// Estimate release date from version number
    ///
    /// InterPro typically releases quarterly, but exact dates vary.
    /// We estimate the first day of the month for historical versions.
    /// For accurate dates, parse release_notes.txt (not implemented for speed).
    pub fn estimate_release_date(major: u32, _minor: u32) -> NaiveDate {
        // InterPro versions are sequential: 96.0, 97.0, 98.0, etc.
        // Releases happen roughly every 2-3 months
        // We estimate based on version number: assume release 1.0 was in 2001-01-01
        // and increment by ~3 months per version

        let base_year = 2001;
        let base_month = 1;
        let months_per_release = 3;

        let total_months = (major as i32 - 1) * months_per_release;
        let years_offset = total_months / 12;
        let months_offset = total_months % 12;

        let year = base_year + years_offset;
        let month = base_month + months_offset;

        // Clamp to valid date range
        let year = year.max(2001).min(2100);
        let month = (month as u32).max(1).min(12);

        NaiveDate::from_ymd_opt(year, month, 1).unwrap_or_else(|| {
            // SAFETY: 2024-01-01 is a valid date, so this will never panic
            #[allow(clippy::expect_used)]
            {
                NaiveDate::from_ymd_opt(2024, 1, 1)
                    .expect("Hardcoded fallback date 2024-01-01 is always valid")
            }
        })
    }
}

/// InterPro version discovery service
pub struct VersionDiscovery {
    config: InterProConfig,
}

impl VersionDiscovery {
    pub fn new(config: InterProConfig) -> Self {
        Self { config }
    }

    /// Discover all available versions from FTP
    ///
    /// This discovers both the current release and all historical releases.
    /// Historical versions are in numbered directories (e.g., /96.0/, /97.0/).
    pub async fn discover_all_versions(&self) -> Result<Vec<DiscoveredVersion>> {
        let mut versions = Vec::new();

        // 1. Discover current release
        match self.discover_current_version().await {
            Ok(current) => {
                tracing::info!(
                    version = %current.external_version,
                    date = %current.release_date,
                    "Discovered current InterPro release"
                );
                versions.push(current);
            },
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Could not discover current release (will try historical versions)"
                );
            },
        }

        // 2. Discover all historical versions
        match self.discover_historical_versions().await {
            Ok(mut historical) => {
                tracing::info!(
                    count = historical.len(),
                    "Discovered {} historical InterPro releases",
                    historical.len()
                );
                versions.append(&mut historical);
            },
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Could not discover historical releases"
                );
            },
        }

        // Sort by version (oldest first)
        versions.sort();

        if versions.is_empty() {
            anyhow::bail!(
                "No InterPro versions found on FTP. \
                Please check FTP configuration and network connectivity."
            );
        }

        Ok(versions)
    }

    /// Discover the current release version
    ///
    /// The current release is in the /current/ or /current_release/ directory.
    /// We need to determine the version number from the directory listing or metadata.
    async fn discover_current_version(&self) -> Result<DiscoveredVersion> {
        let mut downloader = InterProFtpDownloader::new(self.config.clone());
        downloader.connect()?;

        // Get current version (this reads from FTP)
        let version_str = downloader
            .get_current_version()
            .context("Failed to get current version from FTP")?;

        downloader.disconnect()?;

        let (major, minor) = DiscoveredVersion::parse_version(&version_str)?;
        let release_date = DiscoveredVersion::estimate_release_date(major, minor);

        Ok(DiscoveredVersion {
            external_version: version_str.clone(),
            major,
            minor,
            release_date,
            is_current: true,
            ftp_directory: "current".to_string(),
        })
    }

    /// Discover all historical release versions
    ///
    /// Historical releases are in numbered directories:
    /// - /pub/databases/interpro/96.0/
    /// - /pub/databases/interpro/97.0/
    /// - /pub/databases/interpro/98.0/
    async fn discover_historical_versions(&self) -> Result<Vec<DiscoveredVersion>> {
        let mut downloader = InterProFtpDownloader::new(self.config.clone());
        downloader.connect()?;

        // List all version directories
        let version_dirs = downloader
            .list_versions()
            .context("Failed to list version directories from FTP")?;

        downloader.disconnect()?;

        tracing::info!(
            count = version_dirs.len(),
            "Found {} version directories on FTP",
            version_dirs.len()
        );

        let mut versions = Vec::new();
        let version_pattern = Regex::new(r"^(\d+)\.(\d+)$")?;

        for dir_name in version_dirs {
            if let Some(captures) = version_pattern.captures(&dir_name) {
                let major: u32 = captures[1].parse()?;
                let minor: u32 = captures[2].parse()?;
                let version_str = format!("{}.{}", major, minor);

                // Estimate release date
                let release_date = DiscoveredVersion::estimate_release_date(major, minor);

                versions.push(DiscoveredVersion {
                    external_version: version_str.clone(),
                    major,
                    minor,
                    release_date,
                    is_current: false,
                    ftp_directory: dir_name.clone(),
                });

                tracing::debug!(
                    version = %version_str,
                    date = %release_date,
                    directory = %dir_name,
                    "Discovered historical version"
                );
            } else {
                tracing::debug!(
                    directory = %dir_name,
                    "Skipping non-version directory"
                );
            }
        }

        // Sort chronologically (oldest to newest)
        versions.sort();

        tracing::info!(
            count = versions.len(),
            "Parsed {} valid InterPro versions from directory listing",
            versions.len()
        );

        Ok(versions)
    }

    /// Filter versions that haven't been ingested yet
    pub fn filter_new_versions(
        &self,
        discovered: Vec<DiscoveredVersion>,
        ingested_versions: Vec<String>,
    ) -> Vec<DiscoveredVersion> {
        VersionFilter::filter_new_versions(discovered, &ingested_versions)
    }

    /// Filter versions starting from a specific version (inclusive)
    ///
    /// # Arguments
    /// * `discovered` - All discovered versions
    /// * `start_version` - Minimum version to include (e.g., "96.0")
    ///
    /// # Returns
    /// Versions >= start_version, sorted oldest to newest
    pub fn filter_from_version(
        &self,
        mut discovered: Vec<DiscoveredVersion>,
        start_version: &str,
    ) -> Result<Vec<DiscoveredVersion>> {
        let (start_major, start_minor) = DiscoveredVersion::parse_version(start_version)?;

        discovered.retain(|v| {
            v.major > start_major || (v.major == start_major && v.minor >= start_minor)
        });

        // Sort to ensure chronological order
        discovered.sort();

        Ok(discovered)
    }

    // ========================================================================
    // Database Integration Methods
    // ========================================================================

    /// Get all ingested versions from database
    ///
    /// Returns external version identifiers (e.g., ["96.0", "97.0"])
    pub async fn get_ingested_versions(&self, pool: &PgPool) -> Result<Vec<String>> {
        let records = sqlx::query!(
            r#"
            SELECT DISTINCT external_version
            FROM versions v
            JOIN registry_entries re ON v.entry_id = re.id
            JOIN organizations o ON re.organization_id = o.id
            WHERE o.name = 'InterPro'
              AND re.entry_type = 'data_source'
              AND external_version IS NOT NULL
            ORDER BY external_version
            "#
        )
        .fetch_all(pool)
        .await?;

        Ok(records
            .into_iter()
            .filter_map(|r| r.external_version)
            .collect())
    }

    /// Get last ingested version for InterPro organization
    pub async fn get_last_ingested_version(
        &self,
        pool: &PgPool,
        organization_id: Uuid,
    ) -> Result<Option<String>> {
        let result = sqlx::query!(
            r#"
            SELECT last_external_version
            FROM organization_sync_status
            WHERE organization_id = $1
            "#,
            organization_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(result.and_then(|r| r.last_external_version))
    }

    /// Check if newer version available compared to last ingested
    pub async fn check_for_newer_version(
        &self,
        pool: &PgPool,
        organization_id: Uuid,
    ) -> Result<Option<DiscoveredVersion>> {
        // 1. Get last ingested version
        let last_version = self
            .get_last_ingested_version(pool, organization_id)
            .await?;

        // 2. Discover all available versions
        let mut available = self.discover_all_versions().await?;

        // Sort by version (newest first)
        available.sort();
        available.reverse();

        // 3. Return newest if different from last ingested
        match (last_version, available.first()) {
            (Some(last), Some(newest)) if newest.external_version != last => {
                Ok(Some(newest.clone()))
            },
            (None, Some(newest)) => Ok(Some(newest.clone())), // First ingestion
            _ => Ok(None),                                    // Up-to-date
        }
    }

    /// Check if version exists in database
    pub async fn version_exists_in_db(
        &self,
        pool: &PgPool,
        external_version: &str,
    ) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM versions v
                JOIN registry_entries re ON v.entry_id = re.id
                JOIN organizations o ON re.organization_id = o.id
                WHERE o.name = 'InterPro'
                  AND re.entry_type = 'data_source'
                  AND v.external_version = $1
            ) as "exists!"
            "#,
            external_version
        )
        .fetch_one(pool)
        .await?;

        Ok(result.exists)
    }

    /// Get InterPro organization ID
    pub async fn get_organization_id(&self, pool: &PgPool) -> Result<Uuid> {
        let result = sqlx::query!(
            r#"
            SELECT id
            FROM organizations
            WHERE name = 'InterPro'
            "#
        )
        .fetch_one(pool)
        .await
        .context("InterPro organization not found in database")?;

        Ok(result.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_version() {
        let (major, minor) = DiscoveredVersion::parse_version("96.0").unwrap();
        assert_eq!(major, 96);
        assert_eq!(minor, 0);

        let (major, minor) = DiscoveredVersion::parse_version("100.0").unwrap();
        assert_eq!(major, 100);
        assert_eq!(minor, 0);

        // Invalid formats
        assert!(DiscoveredVersion::parse_version("96").is_err());
        assert!(DiscoveredVersion::parse_version("96.0.1").is_err());
        assert!(DiscoveredVersion::parse_version("invalid").is_err());
    }

    #[test]
    fn test_version_ordering() {
        let v1 = DiscoveredVersion {
            external_version: "96.0".to_string(),
            major: 96,
            minor: 0,
            release_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            is_current: false,
            ftp_directory: "96.0".to_string(),
        };

        let v2 = DiscoveredVersion {
            external_version: "97.0".to_string(),
            major: 97,
            minor: 0,
            release_date: NaiveDate::from_ymd_opt(2024, 4, 15).unwrap(),
            is_current: false,
            ftp_directory: "97.0".to_string(),
        };

        let v3 = DiscoveredVersion {
            external_version: "98.0".to_string(),
            major: 98,
            minor: 0,
            release_date: NaiveDate::from_ymd_opt(2024, 7, 15).unwrap(),
            is_current: true,
            ftp_directory: "current".to_string(),
        };

        let mut versions = vec![v3.clone(), v1.clone(), v2.clone()];
        versions.sort();

        // Should be sorted oldest to newest: 96.0, 97.0, 98.0
        assert_eq!(versions[0].major, 96);
        assert_eq!(versions[1].major, 97);
        assert_eq!(versions[2].major, 98);
    }

    #[test]
    fn test_filter_new_versions() {
        let config = InterProConfig::default();
        let discovery = VersionDiscovery::new(config);

        let discovered = vec![
            DiscoveredVersion {
                external_version: "96.0".to_string(),
                major: 96,
                minor: 0,
                release_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
                is_current: false,
                ftp_directory: "96.0".to_string(),
            },
            DiscoveredVersion {
                external_version: "97.0".to_string(),
                major: 97,
                minor: 0,
                release_date: NaiveDate::from_ymd_opt(2024, 4, 15).unwrap(),
                is_current: false,
                ftp_directory: "97.0".to_string(),
            },
            DiscoveredVersion {
                external_version: "98.0".to_string(),
                major: 98,
                minor: 0,
                release_date: NaiveDate::from_ymd_opt(2024, 7, 15).unwrap(),
                is_current: true,
                ftp_directory: "current".to_string(),
            },
        ];

        let ingested = vec!["96.0".to_string()];

        let new_versions = discovery.filter_new_versions(discovered, ingested);

        assert_eq!(new_versions.len(), 2);
        assert_eq!(new_versions[0].external_version, "97.0");
        assert_eq!(new_versions[1].external_version, "98.0");
    }

    #[test]
    fn test_filter_from_version() {
        let config = InterProConfig::default();
        let discovery = VersionDiscovery::new(config);

        let discovered = vec![
            DiscoveredVersion {
                external_version: "95.0".to_string(),
                major: 95,
                minor: 0,
                release_date: NaiveDate::from_ymd_opt(2023, 10, 15).unwrap(),
                is_current: false,
                ftp_directory: "95.0".to_string(),
            },
            DiscoveredVersion {
                external_version: "96.0".to_string(),
                major: 96,
                minor: 0,
                release_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
                is_current: false,
                ftp_directory: "96.0".to_string(),
            },
            DiscoveredVersion {
                external_version: "97.0".to_string(),
                major: 97,
                minor: 0,
                release_date: NaiveDate::from_ymd_opt(2024, 4, 15).unwrap(),
                is_current: false,
                ftp_directory: "97.0".to_string(),
            },
            DiscoveredVersion {
                external_version: "98.0".to_string(),
                major: 98,
                minor: 0,
                release_date: NaiveDate::from_ymd_opt(2024, 7, 15).unwrap(),
                is_current: true,
                ftp_directory: "current".to_string(),
            },
        ];

        // Filter from version 96.0 onwards
        let filtered = discovery.filter_from_version(discovered, "96.0").unwrap();

        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].external_version, "96.0");
        assert_eq!(filtered[1].external_version, "97.0");
        assert_eq!(filtered[2].external_version, "98.0");
    }

    #[test]
    fn test_estimate_release_date() {
        // Test that estimate produces reasonable dates
        let date = DiscoveredVersion::estimate_release_date(96, 0);
        assert!(date.year() >= 2001);
        assert!(date.year() <= 2100);
        assert!(date.month() >= 1);
        assert!(date.month() <= 12);
    }
}
