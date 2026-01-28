//! Gene Ontology version discovery and tracking
//!
//! Discovers available versions from HTTP release archive and tracks what's been ingested.
//! GO releases are date-based (YYYY-MM-DD format) and available at http://release.geneontology.org/

use chrono::NaiveDate;
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use sqlx::PgPool;
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::config::GoHttpConfig;
use super::{GoError, Result};

/// Discovered Gene Ontology version from HTTP archive
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredVersion {
    /// External version identifier (e.g., "2025-01-01")
    pub external_version: String,
    /// Release date parsed from version string
    pub release_date: NaiveDate,
    /// HTTP URL for this release
    pub release_url: String,
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

/// Gene Ontology version discovery service
pub struct VersionDiscovery {
    _config: GoHttpConfig,
    client: Client,
}

impl VersionDiscovery {
    pub fn new(config: GoHttpConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .user_agent("BDP-Gene-Ontology-Ingester/1.0")
            .build()?;

        Ok(Self { _config: config, client })
    }

    /// Discover all available versions from HTTP release archive
    ///
    /// Gene Ontology releases are hosted at http://release.geneontology.org/
    /// with date-based directories in YYYY-MM-DD format.
    pub async fn discover_all_versions(&self) -> Result<Vec<DiscoveredVersion>> {
        info!("Discovering GO versions from HTTP release archive");

        let base_url = "http://release.geneontology.org/";

        // Fetch the directory listing HTML
        let html = self.fetch_directory_listing(base_url).await?;

        // Parse the HTML to extract dated directories
        let versions = self.parse_directory_listing(&html, base_url)?;

        info!(
            count = versions.len(),
            "Discovered {} GO release versions",
            versions.len()
        );

        Ok(versions)
    }

    /// Fetch HTML directory listing from URL
    async fn fetch_directory_listing(&self, url: &str) -> Result<String> {
        debug!("Fetching directory listing from: {}", url);

        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(GoError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("HTTP error: {}", response.status()),
            )));
        }

        let html = response.text().await?;

        Ok(html)
    }

    /// Parse HTML directory listing to extract dated releases
    ///
    /// Looks for links matching the YYYY-MM-DD pattern
    fn parse_directory_listing(&self, html: &str, base_url: &str) -> Result<Vec<DiscoveredVersion>> {
        let document = Html::parse_document(html);

        // Try multiple selectors to handle different HTML structures
        // AWS S3, Apache, nginx all have different HTML formats
        let link_selector = Selector::parse("a").unwrap();

        // Regex for matching YYYY-MM-DD format
        let date_pattern = Regex::new(r"^(\d{4})-(\d{2})-(\d{2})/?$")?;

        let mut versions = Vec::new();

        for element in document.select(&link_selector) {
            // Get the href attribute
            if let Some(href) = element.value().attr("href") {
                // Clean up the href (remove trailing slashes)
                let href_clean = href.trim_end_matches('/');

                // Check if it matches the date pattern
                if let Some(captures) = date_pattern.captures(href_clean) {
                    let year: i32 = captures[1].parse()?;
                    let month: u32 = captures[2].parse()?;
                    let day: u32 = captures[3].parse()?;

                    // Validate the date
                    if let Some(release_date) = NaiveDate::from_ymd_opt(year, month, day) {
                        let external_version = format!("{:04}-{:02}-{:02}", year, month, day);
                        let release_url = format!("{}{}/", base_url, external_version);

                        versions.push(DiscoveredVersion {
                            external_version: external_version.clone(),
                            release_date,
                            release_url,
                        });

                        debug!(
                            version = %external_version,
                            date = %release_date,
                            "Discovered GO version"
                        );
                    } else {
                        warn!(
                            year = year,
                            month = month,
                            day = day,
                            "Invalid date in directory listing"
                        );
                    }
                }
            }
        }

        // Sort by date (oldest first)
        versions.sort();

        Ok(versions)
    }

    /// Discover versions with a cutoff date
    ///
    /// Only returns versions released on or after the specified date.
    /// Useful for limiting historical ingestion.
    pub async fn discover_versions_since(&self, cutoff_date: NaiveDate) -> Result<Vec<DiscoveredVersion>> {
        let all_versions = self.discover_all_versions().await?;

        let filtered: Vec<_> = all_versions
            .into_iter()
            .filter(|v| v.release_date >= cutoff_date)
            .collect();

        info!(
            count = filtered.len(),
            cutoff = %cutoff_date,
            "Filtered to {} versions since {}",
            filtered.len(),
            cutoff_date
        );

        Ok(filtered)
    }

    /// Filter out versions that have already been ingested
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

    // ========================================================================
    // Database Integration Methods
    // ========================================================================

    /// Get all ingested GO versions from database
    ///
    /// Queries the versions table for GO term data sources
    pub async fn get_ingested_versions(&self, pool: &PgPool, entry_id: Uuid) -> Result<Vec<String>> {
        let records = sqlx::query!(
            r#"
            SELECT external_version
            FROM versions
            WHERE entry_id = $1
              AND external_version IS NOT NULL
            ORDER BY release_date ASC
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

    /// Get the last ingested version from database
    pub async fn get_last_ingested_version(
        &self,
        pool: &PgPool,
        entry_id: Uuid,
    ) -> Result<Option<String>> {
        let result = sqlx::query!(
            r#"
            SELECT external_version
            FROM versions
            WHERE entry_id = $1
              AND external_version IS NOT NULL
            ORDER BY release_date DESC
            LIMIT 1
            "#,
            entry_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(result.and_then(|r| r.external_version))
    }

    /// Check if a specific version exists in database
    pub async fn version_exists_in_db(
        &self,
        pool: &PgPool,
        entry_id: Uuid,
        external_version: &str,
    ) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM versions
                WHERE entry_id = $1
                  AND external_version = $2
            ) as "exists!"
            "#,
            entry_id,
            external_version
        )
        .fetch_one(pool)
        .await?;

        Ok(result.exists)
    }

    /// Check if newer version is available
    pub async fn check_for_newer_version(
        &self,
        pool: &PgPool,
        entry_id: Uuid,
    ) -> Result<Option<DiscoveredVersion>> {
        // 1. Get last ingested version
        let last_version = self.get_last_ingested_version(pool, entry_id).await?;

        // 2. Discover all available versions
        let mut available = self.discover_all_versions().await?;

        // Sort by release date (newest first)
        available.sort();
        available.reverse();

        // 3. Return newest if different from last ingested
        match (last_version, available.first()) {
            (Some(last), Some(newest)) if newest.external_version != last => {
                info!(
                    last = %last,
                    newest = %newest.external_version,
                    "Newer GO version available"
                );
                Ok(Some(newest.clone()))
            }
            (None, Some(newest)) => {
                info!(
                    version = %newest.external_version,
                    "First GO ingestion"
                );
                Ok(Some(newest.clone()))
            }
            _ => {
                info!("GO is up-to-date");
                Ok(None)
            }
        }
    }

    /// Get versions to ingest for historical backfill
    ///
    /// Returns versions that:
    /// 1. Are within the specified date range
    /// 2. Haven't been ingested yet
    /// 3. Are sorted chronologically (oldest first)
    pub async fn get_versions_for_backfill(
        &self,
        pool: &PgPool,
        entry_id: Uuid,
        start_date: NaiveDate,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<DiscoveredVersion>> {
        // 1. Discover all versions since start_date
        let mut discovered = self.discover_versions_since(start_date).await?;

        // 2. Filter by end_date if provided
        if let Some(end) = end_date {
            discovered.retain(|v| v.release_date <= end);
        }

        // 3. Get already ingested versions
        let ingested = self.get_ingested_versions(pool, entry_id).await?;

        // 4. Filter out already ingested
        let to_ingest = self.filter_new_versions(discovered, ingested);

        info!(
            count = to_ingest.len(),
            start = %start_date,
            end = ?end_date,
            "Found {} GO versions to backfill",
            to_ingest.len()
        );

        Ok(to_ingest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_ordering() {
        let v1 = DiscoveredVersion {
            external_version: "2024-12-01".to_string(),
            release_date: NaiveDate::from_ymd_opt(2024, 12, 1).unwrap(),
            release_url: "http://release.geneontology.org/2024-12-01/".to_string(),
        };

        let v2 = DiscoveredVersion {
            external_version: "2025-01-01".to_string(),
            release_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            release_url: "http://release.geneontology.org/2025-01-01/".to_string(),
        };

        let mut versions = vec![v2.clone(), v1.clone()];
        versions.sort();

        // Oldest first
        assert_eq!(versions[0], v1);
        assert_eq!(versions[1], v2);
    }

    #[test]
    fn test_filter_new_versions() {
        let config = GoHttpConfig::default();
        let discovery = VersionDiscovery::new(config).unwrap();

        let discovered = vec![
            DiscoveredVersion {
                external_version: "2024-11-01".to_string(),
                release_date: NaiveDate::from_ymd_opt(2024, 11, 1).unwrap(),
                release_url: "http://release.geneontology.org/2024-11-01/".to_string(),
            },
            DiscoveredVersion {
                external_version: "2024-12-01".to_string(),
                release_date: NaiveDate::from_ymd_opt(2024, 12, 1).unwrap(),
                release_url: "http://release.geneontology.org/2024-12-01/".to_string(),
            },
            DiscoveredVersion {
                external_version: "2025-01-01".to_string(),
                release_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                release_url: "http://release.geneontology.org/2025-01-01/".to_string(),
            },
        ];

        let ingested = vec!["2024-11-01".to_string()];

        let new_versions = discovery.filter_new_versions(discovered, ingested);

        assert_eq!(new_versions.len(), 2);
        assert_eq!(new_versions[0].external_version, "2024-12-01");
        assert_eq!(new_versions[1].external_version, "2025-01-01");
    }

    #[test]
    fn test_parse_date_from_version() {
        // Test valid date formats
        let date_pattern = Regex::new(r"^(\d{4})-(\d{2})-(\d{2})/?$").unwrap();

        assert!(date_pattern.is_match("2025-01-01"));
        assert!(date_pattern.is_match("2025-01-01/"));
        assert!(!date_pattern.is_match("2025-1-1"));
        assert!(!date_pattern.is_match("25-01-01"));
        assert!(!date_pattern.is_match("2025_01_01"));
    }

    #[test]
    fn test_parse_directory_listing_simple() {
        let config = GoHttpConfig::default();
        let discovery = VersionDiscovery::new(config).unwrap();

        let html = r#"
            <html>
            <body>
                <a href="2024-11-01/">2024-11-01/</a>
                <a href="2024-12-01/">2024-12-01/</a>
                <a href="2025-01-01/">2025-01-01/</a>
                <a href="other-file.txt">other-file.txt</a>
            </body>
            </html>
        "#;

        let versions = discovery
            .parse_directory_listing(html, "http://release.geneontology.org/")
            .unwrap();

        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0].external_version, "2024-11-01");
        assert_eq!(versions[1].external_version, "2024-12-01");
        assert_eq!(versions[2].external_version, "2025-01-01");
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_discover_all_versions() {
        let config = GoHttpConfig::default();
        let discovery = VersionDiscovery::new(config).unwrap();

        let versions = discovery.discover_all_versions().await.unwrap();

        // Should find multiple versions
        assert!(!versions.is_empty());

        // Versions should be sorted chronologically
        for window in versions.windows(2) {
            assert!(window[0].release_date <= window[1].release_date);
        }

        // Check format of a version
        if let Some(first) = versions.first() {
            assert!(first.external_version.contains('-'));
            assert!(first.release_url.starts_with("http://release.geneontology.org/"));
        }
    }
}
