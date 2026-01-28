// Gene Ontology HTTP Configuration

use serde::{Deserialize, Serialize};

/// Configuration for Gene Ontology HTTP downloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoHttpConfig {
    /// Base URL for GO ontology releases
    pub ontology_base_url: String,

    /// Base URL for GOA annotations
    pub annotation_base_url: String,

    /// GO release version (e.g., "2026-01-01")
    pub go_release_version: String,

    /// GOA release version (e.g., "2026-01-15")
    pub goa_release_version: String,

    /// HTTP timeout in seconds
    pub timeout_secs: u64,

    /// Maximum retries for failed downloads
    pub max_retries: u32,

    /// Parse limit for testing (None = parse all)
    pub parse_limit: Option<usize>,

    /// Local path to ontology file (if using local file instead of download)
    pub local_ontology_path: Option<String>,

    /// Zenodo DOI for this release (for attribution)
    pub zenodo_doi: Option<String>,

    /// Citation text for this release
    pub citation: Option<String>,
}

impl Default for GoHttpConfig {
    fn default() -> Self {
        GoHttpConfig {
            // Using EBI FTP as primary source (HTTP URLs behind Cloudflare return 403)
            ontology_base_url: "ftp://ftp.ebi.ac.uk/pub/databases/GO/goa".to_string(),
            annotation_base_url: "ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/UNIPROT".to_string(),
            go_release_version: "current".to_string(),
            goa_release_version: "current".to_string(),
            timeout_secs: 600, // 10 minutes for large FTP downloads
            max_retries: 3,
            parse_limit: None,
            local_ontology_path: None,
            zenodo_doi: None,
            citation: None,
        }
    }
}

impl GoHttpConfig {
    /// Create new config with builder pattern
    pub fn builder() -> GoHttpConfigBuilder {
        GoHttpConfigBuilder::default()
    }

    /// Get URL for GO ontology OBO file
    ///
    /// # Arguments
    /// * `version` - Optional version override (e.g., "2025-01-01")
    ///
    /// If version is provided, downloads from the versioned release archive.
    /// Otherwise uses the configured go_release_version.
    pub fn ontology_url_for_version(&self, version: Option<&str>) -> String {
        let ver = version.unwrap_or(&self.go_release_version);

        // Use HTTP release archive for all downloads
        // This is the canonical source for GO releases
        if ver == "current" {
            // For "current", use the latest from release archive
            // In practice, we should discover the actual latest version
            format!("http://release.geneontology.org/2025-09-08/ontology/go-basic.obo")
        } else {
            format!("http://release.geneontology.org/{}/ontology/go-basic.obo", ver)
        }
    }

    /// Get URL for GO ontology OBO file (using configured version)
    pub fn ontology_url(&self) -> String {
        self.ontology_url_for_version(None)
    }

    /// Get URL for GOA UniProt annotations (gzipped GAF)
    pub fn goa_uniprot_url(&self) -> String {
        format!("{}/goa_uniprot_all.gaf.gz", self.annotation_base_url)
    }

    /// Get URL for specific organism GOA file
    /// Example: organism = "human" -> goa_human.gaf.gz
    pub fn goa_organism_url(&self, organism: &str) -> String {
        format!("{}/goa_{}.gaf.gz", self.annotation_base_url, organism)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.ontology_base_url.is_empty() {
            return Err("Ontology base URL cannot be empty".to_string());
        }

        if self.annotation_base_url.is_empty() {
            return Err("Annotation base URL cannot be empty".to_string());
        }

        if self.go_release_version.is_empty() {
            return Err("GO release version cannot be empty".to_string());
        }

        if self.goa_release_version.is_empty() {
            return Err("GOA release version cannot be empty".to_string());
        }

        if self.timeout_secs == 0 {
            return Err("Timeout must be greater than 0".to_string());
        }

        Ok(())
    }
}

/// Builder for GoHttpConfig
#[derive(Debug, Default)]
pub struct GoHttpConfigBuilder {
    ontology_base_url: Option<String>,
    annotation_base_url: Option<String>,
    go_release_version: Option<String>,
    goa_release_version: Option<String>,
    timeout_secs: Option<u64>,
    max_retries: Option<u32>,
    parse_limit: Option<usize>,
    local_ontology_path: Option<String>,
    zenodo_doi: Option<String>,
    citation: Option<String>,
}

impl GoHttpConfigBuilder {
    pub fn ontology_base_url(mut self, url: String) -> Self {
        self.ontology_base_url = Some(url);
        self
    }

    pub fn annotation_base_url(mut self, url: String) -> Self {
        self.annotation_base_url = Some(url);
        self
    }

    pub fn go_release_version(mut self, version: String) -> Self {
        self.go_release_version = Some(version);
        self
    }

    pub fn goa_release_version(mut self, version: String) -> Self {
        self.goa_release_version = Some(version);
        self
    }

    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    pub fn parse_limit(mut self, limit: usize) -> Self {
        self.parse_limit = Some(limit);
        self
    }

    pub fn local_ontology_path(mut self, path: String) -> Self {
        self.local_ontology_path = Some(path);
        self
    }

    pub fn zenodo_doi(mut self, doi: String) -> Self {
        self.zenodo_doi = Some(doi);
        self
    }

    pub fn citation(mut self, citation: String) -> Self {
        self.citation = Some(citation);
        self
    }

    pub fn build(self) -> GoHttpConfig {
        let default = GoHttpConfig::default();

        GoHttpConfig {
            ontology_base_url: self.ontology_base_url.unwrap_or(default.ontology_base_url),
            annotation_base_url: self
                .annotation_base_url
                .unwrap_or(default.annotation_base_url),
            go_release_version: self
                .go_release_version
                .unwrap_or(default.go_release_version),
            goa_release_version: self
                .goa_release_version
                .unwrap_or(default.goa_release_version),
            timeout_secs: self.timeout_secs.unwrap_or(default.timeout_secs),
            max_retries: self.max_retries.unwrap_or(default.max_retries),
            parse_limit: self.parse_limit,
            local_ontology_path: self.local_ontology_path,
            zenodo_doi: self.zenodo_doi,
            citation: self.citation,
        }
    }
}

// ============================================================================
// Preset Configurations
// ============================================================================

impl GoHttpConfig {
    /// Configuration for testing with sample data (limited parse)
    pub fn test_config() -> Self {
        GoHttpConfig {
            ontology_base_url: "ftp://ftp.ebi.ac.uk/pub/databases/GO/goa".to_string(),
            annotation_base_url: "ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/UNIPROT".to_string(),
            go_release_version: "current".to_string(),
            goa_release_version: "current".to_string(),
            timeout_secs: 600,
            max_retries: 3,
            parse_limit: Some(1000), // Only parse first 1000 entries
            local_ontology_path: None,
            zenodo_doi: None,
            citation: None,
        }
    }

    /// Configuration for Zenodo archive with local ontology file
    pub fn zenodo_config(
        local_ontology_path: String,
        release_date: &str,
        zenodo_doi: &str,
    ) -> Self {
        GoHttpConfig {
            ontology_base_url: String::new(), // Not used with local file
            annotation_base_url: "ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/UNIPROT".to_string(),
            go_release_version: release_date.to_string(),
            goa_release_version: "current".to_string(),
            timeout_secs: 600,
            max_retries: 3,
            parse_limit: None,
            local_ontology_path: Some(local_ontology_path),
            zenodo_doi: Some(zenodo_doi.to_string()),
            citation: Some(format!(
                "Gene Ontology data from the {} release (DOI: {}) is made available under the terms of the Creative Commons Attribution 4.0 International license (CC BY 4.0).",
                release_date, zenodo_doi
            )),
        }
    }

    /// Configuration for production (full dataset)
    pub fn production_config() -> Self {
        GoHttpConfig::default()
    }

    /// Configuration for specific organism
    pub fn organism_config(_organism: &str, go_version: &str, goa_version: &str) -> Self {
        GoHttpConfig {
            ontology_base_url: "http://release.geneontology.org".to_string(),
            annotation_base_url: "http://geneontology.org/gene-associations".to_string(),
            go_release_version: go_version.to_string(),
            goa_release_version: goa_version.to_string(),
            timeout_secs: 300,
            max_retries: 3,
            parse_limit: None,
            local_ontology_path: None,
            zenodo_doi: None,
            citation: None,
        }
    }
}

// ============================================================================
// Environment Variable Support
// ============================================================================

impl GoHttpConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        GoHttpConfig {
            ontology_base_url: std::env::var("GO_ONTOLOGY_BASE_URL")
                .unwrap_or_else(|_| "ftp://ftp.ebi.ac.uk/pub/databases/GO/goa".to_string()),
            annotation_base_url: std::env::var("GOA_BASE_URL")
                .unwrap_or_else(|_| "ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/UNIPROT".to_string()),
            go_release_version: std::env::var("GO_RELEASE_VERSION")
                .unwrap_or_else(|_| "current".to_string()),
            goa_release_version: std::env::var("GOA_RELEASE_VERSION")
                .unwrap_or_else(|_| "current".to_string()),
            timeout_secs: std::env::var("GO_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(600),
            max_retries: std::env::var("GO_MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            parse_limit: std::env::var("GO_PARSE_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok()),
            local_ontology_path: std::env::var("GO_LOCAL_ONTOLOGY_PATH").ok(),
            zenodo_doi: std::env::var("GO_ZENODO_DOI").ok(),
            citation: std::env::var("GO_CITATION").ok(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GoHttpConfig::default();
        assert_eq!(config.ontology_base_url, "ftp://ftp.ebi.ac.uk/pub/databases/GO/goa");
        assert_eq!(config.annotation_base_url, "ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/UNIPROT");
        assert_eq!(config.go_release_version, "current");
        assert_eq!(config.timeout_secs, 600);
        assert_eq!(config.max_retries, 3);
        assert!(config.parse_limit.is_none());
    }

    #[test]
    fn test_ontology_url() {
        let config = GoHttpConfig::default();
        // With FTP base URL, falls back to HTTP release URL
        assert_eq!(
            config.ontology_url(),
            "http://release.geneontology.org/2025-09-08/ontology/go-basic.obo"
        );
    }

    #[test]
    fn test_ontology_url_specific_version() {
        let config = GoHttpConfig::builder()
            .go_release_version("2025-09-08".to_string())
            .build();
        assert_eq!(
            config.ontology_url(),
            "http://release.geneontology.org/2025-09-08/ontology/go-basic.obo"
        );
    }

    #[test]
    fn test_goa_uniprot_url() {
        let config = GoHttpConfig::default();
        assert_eq!(
            config.goa_uniprot_url(),
            "ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/UNIPROT/goa_uniprot_all.gaf.gz"
        );
    }

    #[test]
    fn test_goa_organism_url() {
        let config = GoHttpConfig::default();
        assert_eq!(
            config.goa_organism_url("human"),
            "ftp://ftp.ebi.ac.uk/pub/databases/GO/goa/UNIPROT/goa_human.gaf.gz"
        );
    }

    #[test]
    fn test_builder_pattern() {
        let config = GoHttpConfig::builder()
            .go_release_version("2025-12-01".to_string())
            .timeout_secs(600)
            .parse_limit(100)
            .build();

        assert_eq!(config.go_release_version, "2025-12-01");
        assert_eq!(config.timeout_secs, 600);
        assert_eq!(config.parse_limit, Some(100));
    }

    #[test]
    fn test_validate() {
        let config = GoHttpConfig::default();
        assert!(config.validate().is_ok());

        let mut invalid_config = config.clone();
        invalid_config.ontology_base_url = "".to_string();
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_test_config() {
        let config = GoHttpConfig::test_config();
        assert_eq!(config.parse_limit, Some(1000));
    }

    #[test]
    fn test_production_config() {
        let config = GoHttpConfig::production_config();
        assert!(config.parse_limit.is_none());
    }
}
