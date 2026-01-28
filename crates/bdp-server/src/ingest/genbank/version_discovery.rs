//! GenBank/RefSeq version discovery and tracking
//!
//! Discovers available versions from FTP and tracks what's been ingested

use anyhow::{Context, Result};
use chrono::NaiveDate;
use regex::Regex;
use sqlx::PgPool;
use uuid::Uuid;

use super::config::GenbankFtpConfig;
use super::ftp::GenbankFtp;
use super::models::SourceDatabase;
use crate::ingest::common::version_discovery::{
    DiscoveredVersion as DiscoveredVersionTrait, VersionFilter,
};

/// Discovered GenBank/RefSeq version from FTP
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredVersion {
    /// External version identifier (e.g., "GB_Release_257.0" or "RefSeq-117")
    pub external_version: String,
    /// Release date (estimated from release number for GenBank)
    pub release_date: NaiveDate,
    /// Release number (e.g., 257 for GenBank, 117 for RefSeq)
    pub release_number: i32,
    /// Source database type (GenBank or RefSeq)
    pub source_database: SourceDatabase,
}

impl DiscoveredVersionTrait for DiscoveredVersion {
    fn external_version(&self) -> &str {
        &self.external_version
    }

    fn release_date(&self) -> NaiveDate {
        self.release_date
    }
}

// GenBank uses custom ordering by release number instead of date
impl PartialOrd for DiscoveredVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DiscoveredVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by release number (oldest first)
        // GenBank release numbers are more reliable than estimated dates
        self.release_number.cmp(&other.release_number)
    }
}

/// GenBank/RefSeq version discovery service
pub struct VersionDiscovery {
    config: GenbankFtpConfig,
    ftp: GenbankFtp,
}

impl VersionDiscovery {
    pub fn new(config: GenbankFtpConfig) -> Self {
        let ftp = GenbankFtp::new(config.clone());
        Self { config, ftp }
    }

    /// Discover all available versions from FTP
    pub async fn discover_all_versions(&self) -> Result<Vec<DiscoveredVersion>> {
        match self.config.source_database {
            SourceDatabase::Genbank => self.discover_genbank_versions().await,
            SourceDatabase::Refseq => self.discover_refseq_versions().await,
        }
    }

    /// Discover current GenBank release
    async fn discover_current_genbank(&self) -> Result<DiscoveredVersion> {
        let release_str = self
            .ftp
            .get_current_release()
            .await
            .context("Failed to get current GenBank release number")?;

        let release_number = self.parse_genbank_release_number(&release_str)?;

        // GenBank releases are published approximately every 2 months
        // Estimate date based on release number (starting from Release 1 in 1982)
        let release_date = self.estimate_genbank_release_date(release_number);

        Ok(DiscoveredVersion {
            external_version: release_str,
            release_date,
            release_number,
            source_database: SourceDatabase::Genbank,
        })
    }

    /// Discover all GenBank versions (current only, no historical archive available)
    async fn discover_genbank_versions(&self) -> Result<Vec<DiscoveredVersion>> {
        tracing::info!("Discovering GenBank versions from FTP");

        // GenBank only publishes the current release, not historical ones
        // Previous releases are not available on the FTP server
        let current = self.discover_current_genbank().await?;

        tracing::info!(
            version = %current.external_version,
            release_number = current.release_number,
            date = %current.release_date,
            "Discovered current GenBank release"
        );

        Ok(vec![current])
    }

    /// Discover RefSeq versions (current and historical if available)
    async fn discover_refseq_versions(&self) -> Result<Vec<DiscoveredVersion>> {
        tracing::info!("Discovering RefSeq versions from FTP");

        let mut versions = Vec::new();

        // 1. Try to get current release
        match self.discover_current_refseq().await {
            Ok(current) => {
                tracing::info!(
                    version = %current.external_version,
                    release_number = current.release_number,
                    date = %current.release_date,
                    "Discovered current RefSeq release"
                );
                versions.push(current);
            },
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Could not discover current RefSeq release (this may be expected)"
                );
            },
        }

        // 2. Try to list historical releases
        match self.discover_historical_refseq().await {
            Ok(mut historical) => {
                tracing::info!(count = historical.len(), "Discovered historical RefSeq releases");
                versions.append(&mut historical);
            },
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Could not discover historical RefSeq releases (this may be expected)"
                );
            },
        }

        // Sort by release number (oldest first)
        versions.sort();
        versions.dedup_by_key(|v| v.release_number);

        Ok(versions)
    }

    /// Discover current RefSeq release
    async fn discover_current_refseq(&self) -> Result<DiscoveredVersion> {
        let release_str = self
            .ftp
            .get_current_release()
            .await
            .context("Failed to get current RefSeq release number")?;

        let release_number = self.parse_refseq_release_number(&release_str)?;

        // RefSeq releases are published approximately every 2 months
        // Estimate date based on release number
        let release_date = self.estimate_refseq_release_date(release_number);

        Ok(DiscoveredVersion {
            external_version: format!("RefSeq-{}", release_number),
            release_date,
            release_number,
            source_database: SourceDatabase::Refseq,
        })
    }

    /// Discover historical RefSeq releases by listing FTP directories
    async fn discover_historical_refseq(&self) -> Result<Vec<DiscoveredVersion>> {
        // RefSeq may have historical releases in numbered directories
        // e.g., /refseq/release/100/, /refseq/release/101/, etc.
        let _base_path = self.config.get_base_path();

        // List directories in RefSeq release folder
        let directories = self
            .ftp
            .list_release_directories()
            .await
            .context("Failed to list RefSeq release directories")?;

        tracing::info!(
            count = directories.len(),
            "Found {} potential RefSeq release directories",
            directories.len()
        );

        let mut versions = Vec::new();
        let release_pattern = Regex::new(r"^(\d+)$")?;

        for dir_name in directories {
            if let Some(captures) = release_pattern.captures(&dir_name) {
                if let Ok(release_number) = captures[1].parse::<i32>() {
                    let release_date = self.estimate_refseq_release_date(release_number);

                    versions.push(DiscoveredVersion {
                        external_version: format!("RefSeq-{}", release_number),
                        release_date,
                        release_number,
                        source_database: SourceDatabase::Refseq,
                    });

                    tracing::debug!(
                        version = %release_number,
                        date = %release_date,
                        "Discovered historical RefSeq version"
                    );
                }
            }
        }

        tracing::info!(
            count = versions.len(),
            "Parsed {} valid RefSeq versions from directory names",
            versions.len()
        );

        Ok(versions)
    }

    /// Parse GenBank release number from string
    /// Format: "257.0" -> 257
    fn parse_genbank_release_number(&self, release_str: &str) -> Result<i32> {
        // Remove "GB_Release_" prefix if present
        let release_str = release_str
            .strip_prefix("GB_Release_")
            .unwrap_or(release_str);

        // Parse number before decimal point
        let parts: Vec<&str> = release_str.split('.').collect();
        let release_number = parts
            .first()
            .context("Invalid GenBank release format")?
            .parse::<i32>()
            .context("Failed to parse GenBank release number")?;

        Ok(release_number)
    }

    /// Parse RefSeq release number from string
    /// Format: "117" -> 117 or "RefSeq-117" -> 117
    fn parse_refseq_release_number(&self, release_str: &str) -> Result<i32> {
        // Remove "RefSeq-" prefix if present
        let release_str = release_str.strip_prefix("RefSeq-").unwrap_or(release_str);

        release_str
            .trim()
            .parse::<i32>()
            .context("Failed to parse RefSeq release number")
    }

    /// Estimate GenBank release date based on release number
    ///
    /// GenBank started in 1982 and releases approximately every 2 months (6 per year)
    /// This is an approximation - actual dates may vary
    fn estimate_genbank_release_date(&self, release_number: i32) -> NaiveDate {
        // GenBank Release 1 was in 1982
        let base_year = 1982;
        let releases_per_year = 6;

        let years_since_start = release_number / releases_per_year;
        let release_in_year = release_number % releases_per_year;

        let year = base_year + years_since_start;
        let month = ((release_in_year * 2) + 1).min(12) as u32;

        // Use the 15th of the month as a reasonable estimate
        NaiveDate::from_ymd_opt(year, month, 15).unwrap_or_else(|| {
            NaiveDate::from_ymd_opt(year, 1, 1).unwrap_or_else(|| NaiveDate::MIN)
        })
    }

    /// Estimate RefSeq release date based on release number
    ///
    /// RefSeq started around 2000 and releases approximately every 2 months (6 per year)
    /// This is an approximation - actual dates may vary
    fn estimate_refseq_release_date(&self, release_number: i32) -> NaiveDate {
        // RefSeq started around 2000
        let base_year = 2000;
        let releases_per_year = 6;

        let years_since_start = release_number / releases_per_year;
        let release_in_year = release_number % releases_per_year;

        let year = base_year + years_since_start;
        let month = ((release_in_year * 2) + 1).min(12) as u32;

        // Use the 15th of the month as a reasonable estimate
        NaiveDate::from_ymd_opt(year, month, 15).unwrap_or_else(|| {
            NaiveDate::from_ymd_opt(year, 1, 1).unwrap_or_else(|| NaiveDate::MIN)
        })
    }

    /// Filter versions that haven't been ingested yet
    pub fn filter_new_versions(
        &self,
        discovered: Vec<DiscoveredVersion>,
        ingested_versions: Vec<String>,
    ) -> Vec<DiscoveredVersion> {
        VersionFilter::filter_new_versions(discovered, &ingested_versions)
    }

    /// Filter versions starting from a specific release number
    pub fn filter_from_release(
        &self,
        discovered: Vec<DiscoveredVersion>,
        start_release: i32,
    ) -> Vec<DiscoveredVersion> {
        discovered
            .into_iter()
            .filter(|v| v.release_number >= start_release)
            .collect()
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
        let last_version = self
            .get_last_ingested_version(pool, organization_id)
            .await?;

        // 2. Discover all available versions from FTP
        let mut available = self.discover_all_versions().await?;

        // Sort by release number (newest first)
        available.sort();
        available.reverse();

        // 3. Return newest if different from last ingested, None if up-to-date
        match (last_version, available.first()) {
            (Some(last), Some(newest)) if newest.external_version != last => {
                Ok(Some(newest.clone()))
            },
            (None, Some(newest)) => Ok(Some(newest.clone())), // First ingestion
            _ => Ok(None),                                    // Up-to-date
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

    /// Get all ingested versions for a data source
    pub async fn get_ingested_versions(
        &self,
        pool: &PgPool,
        entry_id: Uuid,
    ) -> Result<Vec<String>> {
        let records = sqlx::query!(
            r#"
            SELECT external_version
            FROM versions
            WHERE entry_id = $1
              AND external_version IS NOT NULL
            ORDER BY external_version
            "#,
            entry_id
        )
        .fetch_all(pool)
        .await?;

        Ok(records
            .into_iter()
            .filter_map(|r| r.external_version)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_version_ordering() {
        let v1 = DiscoveredVersion {
            external_version: "GB_Release_256.0".to_string(),
            release_date: NaiveDate::from_ymd_opt(2024, 11, 15).unwrap(),
            release_number: 256,
            source_database: SourceDatabase::Genbank,
        };

        let v2 = DiscoveredVersion {
            external_version: "GB_Release_257.0".to_string(),
            release_date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            release_number: 257,
            source_database: SourceDatabase::Genbank,
        };

        let mut versions = vec![v2.clone(), v1.clone()];
        versions.sort();

        // Oldest first
        assert_eq!(versions[0], v1);
        assert_eq!(versions[1], v2);
    }

    #[test]
    fn test_parse_genbank_release_number() {
        let config = GenbankFtpConfig::new().with_genbank();
        let discovery = VersionDiscovery::new(config);

        assert_eq!(discovery.parse_genbank_release_number("257.0").unwrap(), 257);
        assert_eq!(
            discovery
                .parse_genbank_release_number("GB_Release_257.0")
                .unwrap(),
            257
        );
        assert_eq!(discovery.parse_genbank_release_number("256.0").unwrap(), 256);
    }

    #[test]
    fn test_parse_refseq_release_number() {
        let config = GenbankFtpConfig::new().with_refseq();
        let discovery = VersionDiscovery::new(config);

        assert_eq!(discovery.parse_refseq_release_number("117").unwrap(), 117);
        assert_eq!(discovery.parse_refseq_release_number("RefSeq-117").unwrap(), 117);
        assert_eq!(discovery.parse_refseq_release_number("100").unwrap(), 100);
    }

    #[test]
    fn test_estimate_genbank_release_date() {
        let config = GenbankFtpConfig::new().with_genbank();
        let discovery = VersionDiscovery::new(config);

        // Release 257 (approx. 2025)
        let date = discovery.estimate_genbank_release_date(257);
        assert_eq!(date.year(), 2024); // 1982 + (257 / 6) = 2024
        assert!(date.month() >= 1 && date.month() <= 12);

        // Release 1 should be in 1982
        let date = discovery.estimate_genbank_release_date(1);
        assert_eq!(date.year(), 1982);
    }

    #[test]
    fn test_estimate_refseq_release_date() {
        let config = GenbankFtpConfig::new().with_refseq();
        let discovery = VersionDiscovery::new(config);

        // Release 117 (approx. 2019)
        let date = discovery.estimate_refseq_release_date(117);
        assert_eq!(date.year(), 2019); // 2000 + (117 / 6) = 2019
        assert!(date.month() >= 1 && date.month() <= 12);

        // Release 6 should be in 2001
        let date = discovery.estimate_refseq_release_date(6);
        assert_eq!(date.year(), 2001);
    }

    #[test]
    fn test_filter_new_versions() {
        let config = GenbankFtpConfig::new().with_genbank();
        let discovery = VersionDiscovery::new(config);

        let discovered = vec![
            DiscoveredVersion {
                external_version: "GB_Release_255.0".to_string(),
                release_date: NaiveDate::from_ymd_opt(2024, 9, 15).unwrap(),
                release_number: 255,
                source_database: SourceDatabase::Genbank,
            },
            DiscoveredVersion {
                external_version: "GB_Release_256.0".to_string(),
                release_date: NaiveDate::from_ymd_opt(2024, 11, 15).unwrap(),
                release_number: 256,
                source_database: SourceDatabase::Genbank,
            },
            DiscoveredVersion {
                external_version: "GB_Release_257.0".to_string(),
                release_date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
                release_number: 257,
                source_database: SourceDatabase::Genbank,
            },
        ];

        let ingested = vec!["GB_Release_255.0".to_string()];

        let new_versions = discovery.filter_new_versions(discovered, ingested);

        assert_eq!(new_versions.len(), 2);
        assert_eq!(new_versions[0].external_version, "GB_Release_256.0");
        assert_eq!(new_versions[1].external_version, "GB_Release_257.0");
    }

    #[test]
    fn test_filter_from_release() {
        let config = GenbankFtpConfig::new().with_genbank();
        let discovery = VersionDiscovery::new(config);

        let discovered = vec![
            DiscoveredVersion {
                external_version: "GB_Release_255.0".to_string(),
                release_date: NaiveDate::from_ymd_opt(2024, 9, 15).unwrap(),
                release_number: 255,
                source_database: SourceDatabase::Genbank,
            },
            DiscoveredVersion {
                external_version: "GB_Release_256.0".to_string(),
                release_date: NaiveDate::from_ymd_opt(2024, 11, 15).unwrap(),
                release_number: 256,
                source_database: SourceDatabase::Genbank,
            },
            DiscoveredVersion {
                external_version: "GB_Release_257.0".to_string(),
                release_date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
                release_number: 257,
                source_database: SourceDatabase::Genbank,
            },
        ];

        let filtered = discovery.filter_from_release(discovered, 256);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].release_number, 256);
        assert_eq!(filtered[1].release_number, 257);
    }
}
