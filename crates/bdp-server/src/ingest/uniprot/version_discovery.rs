//! UniProt version discovery and tracking
//!
//! Discovers available versions from FTP and tracks what's been ingested

use anyhow::{Context, Result};
use chrono::NaiveDate;
use regex::Regex;
use sqlx::PgPool;
use uuid::Uuid;

use super::config::UniProtFtpConfig;
use super::ftp::UniProtFtp;

/// Discovered UniProt version from FTP
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredVersion {
    /// External version identifier (e.g., "2025_01")
    pub external_version: String,
    /// Release date
    pub release_date: NaiveDate,
    /// FTP path type
    pub is_current: bool,
    /// FTP directory path
    pub ftp_path: String,
}

impl PartialOrd for DiscoveredVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DiscoveredVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by release date, then by version string
        match self.release_date.cmp(&other.release_date) {
            std::cmp::Ordering::Equal => self.external_version.cmp(&other.external_version),
            other => other,
        }
    }
}

/// UniProt version discovery service
pub struct VersionDiscovery {
    config: UniProtFtpConfig,
    ftp: UniProtFtp,
}

impl VersionDiscovery {
    pub fn new(config: UniProtFtpConfig) -> Self {
        let ftp = UniProtFtp::new(config.clone());
        Self { config, ftp }
    }

    /// Discover all available versions from FTP
    pub async fn discover_all_versions(&self) -> Result<Vec<DiscoveredVersion>> {
        let mut versions = Vec::new();

        // 1. Check current release - MUST succeed
        match self.discover_current_version().await {
            Ok(current) => {
                tracing::info!(
                    version = %current.external_version,
                    date = %current.release_date,
                    "Discovered current release"
                );
                versions.push(current);
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to discover current release");
                return Err(e).context(
                    "Current release must be accessible. \
                    This may be due to FTP passive mode issues. \
                    Please check network/firewall configuration."
                );
            }
        }

        // 2. Check previous releases (optional, for backfill)
        match self.discover_previous_versions().await {
            Ok(mut previous) => {
                tracing::info!(count = previous.len(), "Discovered previous releases");
                versions.append(&mut previous);
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Could not discover previous releases (this is optional)"
                );
            }
        }

        // Sort by date (oldest first)
        versions.sort();

        Ok(versions)
    }

    /// Discover the current release version
    async fn discover_current_version(&self) -> Result<DiscoveredVersion> {
        // Download and parse release notes from current/
        let release_notes = self
            .ftp
            .download_release_notes(None)
            .await
            .context("Failed to download current release notes")?;

        let release_info = self
            .ftp
            .parse_release_notes(&release_notes)
            .context("Failed to parse release notes")?;

        Ok(DiscoveredVersion {
            external_version: release_info.external_version,
            release_date: release_info.release_date,
            is_current: true,
            ftp_path: "current_release".to_string(),
        })
    }

    /// Discover all previous release versions
    async fn discover_previous_versions(&self) -> Result<Vec<DiscoveredVersion>> {
        // List directories in previous_releases/
        let listing = self.list_previous_releases().await?;

        let mut versions = Vec::new();
        let release_pattern = Regex::new(r"release-(\d{4})_(\d{2})")?;

        for dir_name in listing {
            if let Some(captures) = release_pattern.captures(&dir_name) {
                let version = format!("{}_{}", &captures[1], &captures[2]);

                // Try to get release notes for this version
                if let Ok(release_notes) = self.ftp.download_release_notes(Some(&version)).await {
                    if let Ok(release_info) = self.ftp.parse_release_notes(&release_notes) {
                        versions.push(DiscoveredVersion {
                            external_version: release_info.external_version,
                            release_date: release_info.release_date,
                            is_current: false,
                            ftp_path: format!("previous_releases/{}", dir_name),
                        });
                    }
                }
            }
        }

        Ok(versions)
    }

    /// List previous release directories using FTP LIST command
    async fn list_previous_releases(&self) -> Result<Vec<String>> {
        let path = format!("{}/previous_releases", self.config.ftp_base_path);

        // Use FTP LIST command to get actual directory listing
        let directories = self
            .ftp
            .list_directories(&path)
            .await
            .context("Failed to list previous releases from FTP")?;

        // Filter to only include directories matching the release pattern
        let release_pattern = Regex::new(r"^release-\d{4}_\d{2}$")?;
        let mut releases: Vec<String> = directories
            .into_iter()
            .filter(|name| release_pattern.is_match(name))
            .collect();

        // Sort chronologically (oldest to newest)
        releases.sort();

        Ok(releases)
    }

    /// Filter versions that haven't been ingested yet
    pub fn filter_new_versions(
        &self,
        discovered: Vec<DiscoveredVersion>,
        ingested_versions: Vec<String>,
    ) -> Vec<DiscoveredVersion> {
        discovered
            .into_iter()
            .filter(|v| !ingested_versions.contains(&v.external_version))
            .collect()
    }

    /// Check if a version should be re-ingested (e.g., current became versioned)
    pub fn should_reingest(
        &self,
        discovered: &DiscoveredVersion,
        ingested_external_version: &str,
        ingested_as_current: bool,
    ) -> bool {
        // If we previously ingested as "current" but now it's in previous_releases,
        // and it's the SAME version, we should NOT re-ingest (same data, just moved)
        if ingested_as_current
            && !discovered.is_current
            && discovered.external_version == ingested_external_version
        {
            return false; // Don't re-ingest migrated version
        }

        // For all other cases, default to not re-ingesting
        // (would be filtered out by filter_new_versions anyway)
        false
    }

    // ========================================================================
    // Database Integration Methods
    // ========================================================================

    /// Check if newer version available compared to last ingested
    pub async fn check_for_newer_version(
        &self,
        pool: &PgPool,
        organization_id: Uuid,
    ) -> Result<Option<DiscoveredVersion>> {
        // 1. Get last ingested version from organization_sync_status
        let last_version = self.get_last_ingested_version(pool, organization_id).await?;

        // 2. Discover all available versions from FTP
        let mut available = self.discover_all_versions().await?;

        // Sort by release date (newest first)
        available.sort();
        available.reverse();

        // 3. Return newest if different from last ingested, None if up-to-date
        match (last_version, available.first()) {
            (Some(last), Some(newest)) if newest.external_version != last => {
                Ok(Some(newest.clone()))
            }
            (None, Some(newest)) => Ok(Some(newest.clone())), // First ingestion
            _ => Ok(None),                                      // Up-to-date
        }
    }

    /// Get last ingested version from database
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

    /// Check if version exists in database
    pub async fn version_exists_in_db(
        &self,
        pool: &PgPool,
        external_version: &str,
    ) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM versions
                WHERE external_version = $1
            ) as "exists!"
            "#,
            external_version
        )
        .fetch_one(pool)
        .await?;

        Ok(result.exists)
    }

    /// Check if version was previously ingested as "current"
    pub async fn was_ingested_as_current(
        &self,
        pool: &PgPool,
        external_version: &str,
    ) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM ingestion_jobs
                WHERE external_version = $1
                  AND source_metadata->>'is_current' = 'true'
            ) as "exists!"
            "#,
            external_version
        )
        .fetch_one(pool)
        .await?;

        Ok(result.exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_ordering() {
        let v1 = DiscoveredVersion {
            external_version: "2024_12".to_string(),
            release_date: NaiveDate::from_ymd_opt(2024, 12, 15).unwrap(),
            is_current: false,
            ftp_path: "previous_releases/release-2024_12".to_string(),
        };

        let v2 = DiscoveredVersion {
            external_version: "2025_01".to_string(),
            release_date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            is_current: true,
            ftp_path: "current_release".to_string(),
        };

        let mut versions = vec![v2.clone(), v1.clone()];
        versions.sort();

        // Oldest first
        assert_eq!(versions[0], v1);
        assert_eq!(versions[1], v2);
    }

    #[test]
    fn test_filter_new_versions() {
        let config = UniProtFtpConfig::new();
        let discovery = VersionDiscovery::new(config);

        let discovered = vec![
            DiscoveredVersion {
                external_version: "2024_11".to_string(),
                release_date: NaiveDate::from_ymd_opt(2024, 11, 15).unwrap(),
                is_current: false,
                ftp_path: "previous_releases/release-2024_11".to_string(),
            },
            DiscoveredVersion {
                external_version: "2024_12".to_string(),
                release_date: NaiveDate::from_ymd_opt(2024, 12, 15).unwrap(),
                is_current: false,
                ftp_path: "previous_releases/release-2024_12".to_string(),
            },
            DiscoveredVersion {
                external_version: "2025_01".to_string(),
                release_date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
                is_current: true,
                ftp_path: "current_release".to_string(),
            },
        ];

        let ingested = vec!["2024_11".to_string()];

        let new_versions = discovery.filter_new_versions(discovered, ingested);

        assert_eq!(new_versions.len(), 2);
        assert_eq!(new_versions[0].external_version, "2024_12");
        assert_eq!(new_versions[1].external_version, "2025_01");
    }

    #[test]
    fn test_should_not_reingest_migrated() {
        let config = UniProtFtpConfig::new();
        let discovery = VersionDiscovery::new(config);

        // Version we ingested as "current"
        let ingested_version = "2025_01";
        let was_current = true;

        // Now it's in previous_releases
        let discovered = DiscoveredVersion {
            external_version: "2025_01".to_string(),
            release_date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            is_current: false,
            ftp_path: "previous_releases/release-2025_01".to_string(),
        };

        let should_reingest =
            discovery.should_reingest(&discovered, ingested_version, was_current);

        // Should NOT re-ingest - it's the same version, just moved
        assert!(!should_reingest);
    }
}
