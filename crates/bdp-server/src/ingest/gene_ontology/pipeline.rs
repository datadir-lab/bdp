// Gene Ontology Pipeline Orchestration
//
// Follows BDP ingestion patterns from UniProt, GenBank, and NCBI Taxonomy:
// - Automatic downloads from Zenodo/FTP
// - S3 storage for downloaded files
// - Batch processing with PostgreSQL
// - Version tracking and attribution

use crate::ingest::gene_ontology::{GoDownloader, GoHttpConfig, GoParser, GoStorage, Result};
use crate::storage::Storage;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

/// Gene Ontology ingestion pipeline
pub struct GoPipeline {
    config: GoHttpConfig,
    db: PgPool,
    s3: Storage,
    organization_id: Uuid,
}

impl GoPipeline {
    /// Create new pipeline with S3 storage
    ///
    /// Follows the same pattern as GenBank/UniProt/NCBI Taxonomy pipelines.
    pub fn new(
        config: GoHttpConfig,
        db: PgPool,
        s3: Storage,
        organization_id: Uuid,
    ) -> Self {
        Self {
            config,
            db,
            s3,
            organization_id,
        }
    }

    /// Create pipeline with optional S3 storage (for backward compatibility)
    pub fn with_optional_s3(
        config: GoHttpConfig,
        db: PgPool,
        s3: Option<Storage>,
        organization_id: Uuid,
    ) -> Result<Self> {
        let storage = s3.ok_or_else(|| {
            crate::ingest::gene_ontology::GoError::Validation(
                "S3 storage is required for GO pipeline".to_string(),
            )
        })?;

        Ok(Self {
            config,
            db,
            s3: storage,
            organization_id,
        })
    }

    /// Run full pipeline: ontology + annotations
    pub async fn run_full(&self, internal_version: &str) -> Result<PipelineStats> {
        info!("Starting full GO pipeline (ontology + annotations)");

        let ontology_stats = self.run_ontology(internal_version).await?;
        let annotation_stats = self.run_annotations().await?;

        let total_stats = PipelineStats {
            terms_stored: ontology_stats.terms_stored,
            relationships_stored: ontology_stats.relationships_stored,
            annotations_stored: annotation_stats.annotations_stored,
        };

        info!("Full pipeline completed: {:?}", total_stats);

        Ok(total_stats)
    }

    /// Run ontology ingestion for a specific version
    ///
    /// # Arguments
    /// * `internal_version` - Our internal version format (e.g., "1.0")
    /// * `external_version` - Optional GO version (YYYY-MM-DD format, e.g., "2025-01-01")
    ///
    /// Follows BDP pattern:
    /// 1. Download OBO file from Zenodo/HTTP
    /// 2. Upload to S3 for archival
    /// 3. Parse OBO content
    /// 4. Store to PostgreSQL
    pub async fn run_ontology_version(
        &self,
        internal_version: &str,
        external_version: Option<&str>,
    ) -> Result<PipelineStats> {
        let version_str = external_version.unwrap_or(&self.config.go_release_version);
        info!(
            internal = internal_version,
            external = version_str,
            "Starting GO ontology ingestion for version"
        );

        // Create a version-specific config if external_version is provided
        let config = if let Some(ext_ver) = external_version {
            let mut versioned_config = self.config.clone();
            versioned_config.go_release_version = ext_ver.to_string();
            versioned_config
        } else {
            self.config.clone()
        };

        // Create downloader with version-specific config
        let downloader = GoDownloader::new(config.clone())?;

        // 1. Download OBO file
        info!("Step 1/4: Downloading GO ontology...");
        let obo_content = downloader.download_ontology().await?;
        info!(
            "Downloaded GO ontology: {} bytes ({} KB)",
            obo_content.len(),
            obo_content.len() / 1024
        );

        // 2. Upload to S3
        info!("Step 2/4: Uploading ontology to S3...");
        let s3_key = format!(
            "go/ontology/{}/go-basic.obo",
            config.go_release_version
        );
        self.s3
            .upload(
                &s3_key,
                obo_content.as_bytes().to_vec(),
                Some("text/plain".to_string()),
            )
            .await
            .map_err(|e| {
                crate::ingest::gene_ontology::GoError::Validation(format!(
                    "Failed to upload to S3: {}",
                    e
                ))
            })?;
        info!("Uploaded ontology to S3: {}", s3_key);

        // 3. Parse OBO file
        info!("Step 3/4: Parsing GO ontology...");
        let parsed = GoParser::parse_obo(
            &obo_content,
            &config.go_release_version,
            config.parse_limit,
        )?;

        info!(
            "Parsed {} terms and {} relationships",
            parsed.terms.len(),
            parsed.relationships.len()
        );

        // 4. Store to database
        info!("Step 4/4: Storing GO ontology...");
        let storage = GoStorage::new(self.db.clone(), self.organization_id);
        let storage_stats = storage
            .store_ontology(
                &parsed.terms,
                &parsed.relationships,
                &config.go_release_version,
                internal_version,
            )
            .await?;

        info!("GO ontology ingestion completed");

        Ok(PipelineStats {
            terms_stored: storage_stats.terms_stored,
            relationships_stored: storage_stats.relationships_stored,
            annotations_stored: 0,
        })
    }

    /// Run ontology ingestion only (using configured version)
    ///
    /// Follows BDP pattern:
    /// 1. Download OBO file from Zenodo/HTTP
    /// 2. Upload to S3 for archival
    /// 3. Parse OBO content
    /// 4. Store to PostgreSQL
    pub async fn run_ontology(&self, internal_version: &str) -> Result<PipelineStats> {
        self.run_ontology_version(internal_version, None).await
    }

    /// Run annotations ingestion only (full GOA dataset)
    ///
    /// Follows BDP pattern:
    /// 1. Download compressed GAF file from FTP
    /// 2. Upload to S3 for archival
    /// 3. Parse annotations
    /// 4. Store to PostgreSQL
    pub async fn run_annotations(&self) -> Result<PipelineStats> {
        info!("Starting GO annotations ingestion (full dataset)");

        // Create instances
        let downloader = GoDownloader::new(self.config.clone())?;
        let storage = GoStorage::new(self.db.clone(), self.organization_id);

        // 1. Build protein lookup map
        info!("Step 1/5: Building protein lookup map...");
        let protein_lookup = storage.build_protein_lookup().await?;
        info!("Built lookup map with {} proteins", protein_lookup.len());

        // 2. Download GAF file
        info!("Step 2/5: Downloading GOA annotations...");
        let gaf_content = downloader.download_goa_uniprot().await?;
        info!(
            "Downloaded GAF: {} bytes ({} MB)",
            gaf_content.len(),
            gaf_content.len() / (1024 * 1024)
        );

        // 3. Upload to S3
        info!("Step 3/5: Uploading annotations to S3...");
        let s3_key = format!("go/annotations/{}/goa_uniprot_all.gaf", self.config.goa_release_version);
        self.s3
            .upload(
                &s3_key,
                gaf_content.as_bytes().to_vec(),
                Some("text/plain".to_string()),
            )
            .await
            .map_err(|e| {
                crate::ingest::gene_ontology::GoError::Validation(format!(
                    "Failed to upload to S3: {}",
                    e
                ))
            })?;
        info!("Uploaded annotations to S3: {}", s3_key);

        // 4. Parse GAF file
        info!("Step 4/5: Parsing GOA annotations...");
        let annotations = GoParser::parse_gaf(
            &gaf_content,
            &self.config.goa_release_version,
            &protein_lookup,
            self.config.parse_limit,
        )?;

        info!("Parsed {} annotations", annotations.len());

        // 5. Store to database
        info!("Step 5/5: Storing GOA annotations...");
        let stored = storage
            .store_annotations(&annotations, &self.config.goa_release_version)
            .await?;

        info!("GO annotations ingestion completed");

        Ok(PipelineStats {
            terms_stored: 0,
            relationships_stored: 0,
            annotations_stored: stored,
        })
    }

    /// Run annotations for specific organism
    ///
    /// Follows BDP pattern:
    /// 1. Download organism-specific GAF file from FTP
    /// 2. Upload to S3 for archival
    /// 3. Parse annotations
    /// 4. Store to PostgreSQL
    pub async fn run_organism_annotations(&self, organism: &str) -> Result<PipelineStats> {
        info!("Starting GO annotations ingestion for organism: {}", organism);

        // Create instances
        let downloader = GoDownloader::new(self.config.clone())?;
        let storage = GoStorage::new(self.db.clone(), self.organization_id);

        // 1. Build protein lookup map
        info!("Step 1/5: Building protein lookup map...");
        let protein_lookup = storage.build_protein_lookup().await?;
        info!("Built lookup map with {} proteins", protein_lookup.len());

        // 2. Download organism-specific GAF file
        info!(
            "Step 2/5: Downloading GOA {} annotations...",
            organism
        );
        let gaf_content = downloader.download_goa_organism(organism).await?;
        info!(
            "Downloaded GAF: {} bytes ({} MB)",
            gaf_content.len(),
            gaf_content.len() / (1024 * 1024)
        );

        // 3. Upload to S3
        info!("Step 3/5: Uploading annotations to S3...");
        let s3_key = format!(
            "go/annotations/{}/goa_{}.gaf",
            self.config.goa_release_version, organism
        );
        self.s3
            .upload(
                &s3_key,
                gaf_content.as_bytes().to_vec(),
                Some("text/plain".to_string()),
            )
            .await
            .map_err(|e| {
                crate::ingest::gene_ontology::GoError::Validation(format!(
                    "Failed to upload to S3: {}",
                    e
                ))
            })?;
        info!("Uploaded annotations to S3: {}", s3_key);

        // 4. Parse GAF file
        info!("Step 4/5: Parsing GOA annotations...");
        let annotations = GoParser::parse_gaf(
            &gaf_content,
            &self.config.goa_release_version,
            &protein_lookup,
            self.config.parse_limit,
        )?;

        info!("Parsed {} annotations for {}", annotations.len(), organism);

        // 5. Store to database
        info!("Step 5/5: Storing GOA annotations...");
        let stored = storage
            .store_annotations(&annotations, &self.config.goa_release_version)
            .await?;

        info!(
            "GO annotations ingestion completed for {}",
            organism
        );

        Ok(PipelineStats {
            terms_stored: 0,
            relationships_stored: 0,
            annotations_stored: stored,
        })
    }

    /// Get pipeline configuration
    pub fn config(&self) -> &GoHttpConfig {
        &self.config
    }

    /// Get S3 storage reference
    pub fn s3_storage(&self) -> &Storage {
        &self.s3
    }

    /// Create a new GoStorage instance for database operations
    pub fn create_storage(&self) -> GoStorage {
        GoStorage::new(self.db.clone(), self.organization_id)
    }
}

/// Pipeline statistics
#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub terms_stored: usize,
    pub relationships_stored: usize,
    pub annotations_stored: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = GoHttpConfig::default();
        assert!(!config.obo_url.is_empty());
        assert!(!config.gaf_url.is_empty());
    }

    #[test]
    fn test_config_test_config() {
        let config = GoHttpConfig::test_config();
        assert!(config.obo_url.contains("test") || !config.obo_url.is_empty());
    }
}
