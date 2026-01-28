// InterPro Ingestion Pipeline
//
// End-to-end pipeline for ingesting InterPro data

use crate::error::Error;
use crate::ingest::interpro::{
    config::InterProConfig,
    ftp::InterProFtpDownloader,
    helpers::{InterProEntryLookupHelper, ProteinLookupHelper, SignatureLookupHelper},
    models::{InterProEntry, MemberSignatureData, ProteinMatch},
    parser::{EntryListParser, Protein2IprParser},
    storage::*,
    version_discovery::{DiscoveredVersion, VersionDiscovery},
};
use sqlx::PgPool;
use std::path::PathBuf;
use tracing::{info, warn};

/// InterPro ingestion pipeline
pub struct InterProPipeline {
    pool: PgPool,
    config: InterProConfig,
    download_dir: PathBuf,
}

impl InterProPipeline {
    /// Create a new pipeline
    pub fn new(pool: PgPool, config: InterProConfig, download_dir: PathBuf) -> Self {
        Self {
            pool,
            config,
            download_dir,
        }
    }

    /// Run the full ingestion pipeline
    pub async fn run(&self, version: &str) -> Result<PipelineStats, Error> {
        info!("Starting InterPro ingestion pipeline for version {}", version);

        let mut stats = PipelineStats::default();

        // Step 1: Download files
        info!("Step 1: Downloading files from FTP");
        let (protein2ipr_path, entry_list_path) = self.download_files(version).await?;
        stats.files_downloaded = 2;

        // Step 2: Parse entry list
        info!("Step 2: Parsing entry list");
        let entries = self.parse_entry_list(&entry_list_path).await?;
        stats.entries_parsed = entries.len();
        info!("Parsed {} InterPro entries", entries.len());

        // Step 3: Store entries
        info!("Step 3: Storing InterPro entries");
        let entry_map = store_interpro_entries_batch(&self.pool, &entries, version).await?;
        stats.entries_stored = entry_map.len();
        info!("Stored {} InterPro entries", entry_map.len());

        // Step 4: Parse protein matches (this can be huge)
        info!("Step 4: Parsing protein matches (streaming)");
        let matches = self.parse_protein2ipr(&protein2ipr_path).await?;
        stats.matches_parsed = matches.len();
        info!("Parsed {} protein matches", matches.len());

        // Step 5: Extract and store signatures
        info!("Step 5: Extracting and storing signatures");
        let signatures = self.extract_signatures(&matches);
        let sig_map = store_signatures_batch(&self.pool, &signatures).await?;
        stats.signatures_stored = sig_map.len();
        info!("Stored {} unique signatures", sig_map.len());

        // Step 6: Store protein matches (critical path - use helpers)
        info!("Step 6: Storing protein matches with batch optimization");
        let mut protein_helper = ProteinLookupHelper::new();
        let mut interpro_helper = InterProEntryLookupHelper::new();
        let mut signature_helper = SignatureLookupHelper::new();

        let matches_stored = store_protein_matches_batch(
            &self.pool,
            &matches,
            &mut protein_helper,
            &mut interpro_helper,
            &mut signature_helper,
        )
        .await?;
        stats.matches_stored = matches_stored;
        info!("Stored {} protein matches", matches_stored);

        info!("InterPro ingestion complete! Stats: {:?}", stats);

        Ok(stats)
    }

    /// Download files from FTP
    async fn download_files(&self, version: &str) -> Result<(PathBuf, PathBuf), Error> {
        let mut downloader = InterProFtpDownloader::new(self.config.clone());

        downloader.connect()?;

        let result = downloader.download_all(version, &self.download_dir);

        downloader.disconnect()?;

        result
    }

    /// Parse entry list file
    async fn parse_entry_list(&self, path: &PathBuf) -> Result<Vec<InterProEntry>, Error> {
        let mut parser = EntryListParser::new();

        parser
            .parse_file(path)
            .map_err(|e| Error::Other(format!("Failed to parse entry list: {}", e)))
    }

    /// Parse protein2ipr file
    async fn parse_protein2ipr(&self, path: &PathBuf) -> Result<Vec<ProteinMatch>, Error> {
        let mut parser = Protein2IprParser::new();

        parser
            .parse_file(path)
            .map_err(|e| Error::Other(format!("Failed to parse protein2ipr: {}", e)))
    }

    /// Extract unique signatures from matches
    fn extract_signatures(&self, matches: &[ProteinMatch]) -> Vec<MemberSignatureData> {
        use std::collections::HashMap;

        let mut unique: HashMap<(String, String), MemberSignatureData> = HashMap::new();

        for match_data in matches {
            let key = (
                match_data.signature_database.to_string(),
                match_data.signature_accession.clone(),
            );

            unique.entry(key).or_insert(MemberSignatureData {
                database: match_data.signature_database,
                accession: match_data.signature_accession.clone(),
                name: match_data.signature_name.clone(),
                description: None,
                is_primary: false, // Will be updated later based on InterPro metadata
            });
        }

        unique.into_values().collect()
    }

    /// Run a test ingestion with small sample data
    pub async fn run_test(&self) -> Result<PipelineStats, Error> {
        info!("Running InterPro test ingestion with sample data");

        let mut stats = PipelineStats::default();

        // Create test entry
        let test_entry = InterProEntry {
            interpro_id: "IPR000001".to_string(),
            entry_type: crate::ingest::interpro::models::EntryType::Domain,
            name: "Kringle".to_string(),
            short_name: Some("Kringle".to_string()),
            description: Some("Test InterPro domain".to_string()),
        };

        // Store test entry with test version
        let test_version = "1.0"; // Test version
        let (ds_id, ver_id) = store_interpro_entry(&self.pool, &test_entry, test_version).await?;
        stats.entries_stored = 1;

        info!("Test entry stored: {} (ds: {}, ver: {})", test_entry.interpro_id, ds_id, ver_id);

        Ok(stats)
    }

    // ========================================================================
    // Version Discovery and Historical Ingestion
    // ========================================================================

    /// Discover all available InterPro versions from FTP
    pub async fn discover_versions(&self) -> Result<Vec<DiscoveredVersion>, Error> {
        let discovery = VersionDiscovery::new(self.config.clone());

        discovery
            .discover_all_versions()
            .await
            .map_err(|e| Error::Other(format!("Version discovery failed: {}", e)))
    }

    /// Get versions that haven't been ingested yet
    pub async fn discover_new_versions(&self) -> Result<Vec<DiscoveredVersion>, Error> {
        let discovery = VersionDiscovery::new(self.config.clone());

        // 1. Discover all available versions
        let all_versions = discovery
            .discover_all_versions()
            .await
            .map_err(|e| Error::Other(format!("Version discovery failed: {}", e)))?;

        // 2. Get already ingested versions
        let ingested = discovery
            .get_ingested_versions(&self.pool)
            .await
            .map_err(|e| Error::Other(format!("Failed to get ingested versions: {}", e)))?;

        // 3. Filter to only new versions
        let new_versions = discovery.filter_new_versions(all_versions, ingested);

        info!("Found {} new InterPro versions to ingest", new_versions.len());

        Ok(new_versions)
    }

    /// Ingest all versions starting from a specific version
    ///
    /// # Arguments
    /// * `start_version` - Minimum version to ingest (e.g., "96.0")
    /// * `skip_existing` - If true, skip versions already in database
    ///
    /// # Returns
    /// Statistics for all ingested versions
    pub async fn ingest_from_version(
        &self,
        start_version: &str,
        skip_existing: bool,
    ) -> Result<Vec<(String, PipelineStats)>, Error> {
        info!("Starting historical ingestion from version {}", start_version);

        let discovery = VersionDiscovery::new(self.config.clone());

        // 1. Discover all versions
        let all_versions = discovery
            .discover_all_versions()
            .await
            .map_err(|e| Error::Other(format!("Version discovery failed: {}", e)))?;

        // 2. Filter to versions >= start_version
        let mut versions = discovery
            .filter_from_version(all_versions, start_version)
            .map_err(|e| Error::Other(format!("Version filtering failed: {}", e)))?;

        // 3. Optionally filter out already-ingested versions
        if skip_existing {
            let ingested = discovery
                .get_ingested_versions(&self.pool)
                .await
                .map_err(|e| Error::Other(format!("Failed to get ingested versions: {}", e)))?;

            versions = discovery.filter_new_versions(versions, ingested);
        }

        info!(
            "Will ingest {} InterPro versions (from {} onwards)",
            versions.len(),
            start_version
        );

        // 4. Ingest each version in order
        let mut results = Vec::new();
        let total_versions = versions.len();
        for version in versions {
            info!(
                "Ingesting InterPro version {} ({}/{})",
                version.external_version,
                results.len() + 1,
                total_versions
            );

            match self.run(&version.external_version).await {
                Ok(stats) => {
                    info!(
                        "Successfully ingested version {}: {:?}",
                        version.external_version, stats
                    );
                    results.push((version.external_version.clone(), stats));
                },
                Err(e) => {
                    warn!("Failed to ingest version {}: {}", version.external_version, e);
                    // Continue with next version instead of failing entire process
                    continue;
                },
            }
        }

        info!(
            "Historical ingestion complete: {} versions successfully ingested",
            results.len()
        );

        Ok(results)
    }

    /// Check if a newer version is available and ingest it
    ///
    /// Returns Some(stats) if a new version was ingested, None if already up-to-date
    pub async fn ingest_latest(&self) -> Result<Option<(String, PipelineStats)>, Error> {
        let discovery = VersionDiscovery::new(self.config.clone());

        // Get InterPro organization ID
        let org_id = discovery
            .get_organization_id(&self.pool)
            .await
            .map_err(|e| Error::Other(format!("Failed to get organization ID: {}", e)))?;

        // Check for newer version
        match discovery
            .check_for_newer_version(&self.pool, org_id)
            .await
            .map_err(|e| Error::Other(format!("Failed to check for newer version: {}", e)))?
        {
            Some(version) => {
                info!("New InterPro version available: {}", version.external_version);

                let stats = self.run(&version.external_version).await?;
                Ok(Some((version.external_version, stats)))
            },
            None => {
                info!("InterPro is already up-to-date");
                Ok(None)
            },
        }
    }
}

/// Pipeline statistics
#[derive(Debug, Default, Clone)]
pub struct PipelineStats {
    pub files_downloaded: usize,
    pub entries_parsed: usize,
    pub entries_stored: usize,
    pub signatures_stored: usize,
    pub matches_parsed: usize,
    pub matches_stored: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_stats_default() {
        let stats = PipelineStats::default();
        assert_eq!(stats.files_downloaded, 0);
        assert_eq!(stats.entries_parsed, 0);
    }
}
