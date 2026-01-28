// GenBank/RefSeq ingestion pipeline
//
// Orchestrates the complete ingestion process for a single GenBank file or division:
// 1. Download file(s) via FTP
// 2. Parse GenBank records
// 3. Store metadata in PostgreSQL (batch operations)
// 4. Upload sequences to S3
// 5. Create protein mappings

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::time::Instant;
use tracing::info;
use uuid::Uuid;

use super::config::GenbankFtpConfig;
use super::ftp::GenbankFtp;
use super::models::{Division, PipelineResult};
use super::parser::GenbankParser;
use super::storage::GenbankStorage;
use crate::storage::Storage;

pub struct GenbankPipeline {
    config: GenbankFtpConfig,
    db: PgPool,
    s3: Storage,
}

impl GenbankPipeline {
    /// Create a new pipeline
    pub fn new(config: GenbankFtpConfig, db: PgPool, s3: Storage) -> Self {
        Self { config, db, s3 }
    }

    /// Create a new pipeline with version discovery support
    pub fn with_version_discovery(config: GenbankFtpConfig, db: PgPool, s3: Storage) -> Self {
        Self { config, db, s3 }
    }

    /// Run pipeline for a single division
    ///
    /// Downloads all files for the division, parses them, and stores the data.
    pub async fn run_division(
        &self,
        organization_id: Uuid,
        division: Division,
        release: &str,
    ) -> Result<PipelineResult> {
        let start_time = Instant::now();

        info!(
            "Starting GenBank pipeline for division {} (release: {})",
            division.as_str(),
            release
        );

        // Step 1: Download division files
        let ftp = GenbankFtp::new(self.config.clone());
        let files = ftp
            .download_division(&division)
            .await
            .context("Failed to download division files")?;

        info!(
            "Downloaded {} files for division {}",
            files.len(),
            division.as_str()
        );

        // Step 2: Parse all files
        let parser = GenbankParser::new(self.config.source_database);
        let mut all_records = Vec::new();

        for (filename, data) in files {
            info!("Parsing file: {} ({} bytes)", filename, data.len());

            let records = if let Some(limit) = self.config.parse_limit {
                parser.parse_with_limit(data.as_slice(), limit)?
            } else {
                parser.parse_all(data.as_slice())?
            };

            info!("Parsed {} records from {}", records.len(), filename);
            all_records.extend(records);

            // Check if we've hit the parse limit
            if let Some(limit) = self.config.parse_limit {
                if all_records.len() >= limit {
                    info!("Reached parse limit of {}, stopping", limit);
                    all_records.truncate(limit);
                    break;
                }
            }
        }

        info!(
            "Parsed total of {} records for division {}",
            all_records.len(),
            division.as_str()
        );

        // Step 3: Store records
        let storage = GenbankStorage::new(
            self.db.clone(),
            self.s3.clone(),
            organization_id,
            "1.0".to_string(), // Internal version
            release.to_string(), // External version
            release.to_string(),
        );

        // Set up citation policy (idempotent)
        storage.setup_citations().await.context("Failed to setup citation policy")?;

        let stats = storage
            .store_records(&all_records)
            .await
            .context("Failed to store records")?;

        let duration = start_time.elapsed();

        info!(
            "Pipeline complete for division {}: {} records, {} stored, {} mappings, {} bytes uploaded in {:.2}s",
            division.as_str(),
            stats.total,
            stats.stored,
            stats.mappings_created,
            stats.bytes_uploaded,
            duration.as_secs_f64()
        );

        Ok(PipelineResult {
            data_source_id: Uuid::new_v4(), // Not used for division-level results
            release: release.to_string(),
            division: division.as_str().to_string(),
            records_processed: stats.total,
            sequences_inserted: stats.stored,
            mappings_created: stats.mappings_created,
            bytes_uploaded: stats.bytes_uploaded,
            duration_seconds: duration.as_secs_f64(),
        })
    }

    /// Run pipeline for a single file (for testing)
    pub async fn run_file(
        &self,
        organization_id: Uuid,
        filename: &str,
        release: &str,
    ) -> Result<PipelineResult> {
        let start_time = Instant::now();

        info!("Starting GenBank pipeline for file: {}", filename);

        // Step 1: Download file
        let ftp = GenbankFtp::new(self.config.clone());
        let data = ftp
            .download_and_decompress(filename)
            .await
            .context("Failed to download file")?;

        info!("Downloaded {} ({} bytes)", filename, data.len());

        // Step 2: Parse file
        let parser = GenbankParser::new(self.config.source_database);
        let records = if let Some(limit) = self.config.parse_limit {
            parser.parse_with_limit(data.as_slice(), limit)?
        } else {
            parser.parse_all(data.as_slice())?
        };

        info!("Parsed {} records from {}", records.len(), filename);

        // Step 3: Determine division from filename
        let division = Self::extract_division_from_filename(filename);

        // Step 4: Store records
        let storage = GenbankStorage::new(
            self.db.clone(),
            self.s3.clone(),
            organization_id,
            "1.0".to_string(),
            release.to_string(),
            release.to_string(),
        );

        // Set up citation policy (idempotent)
        storage.setup_citations().await.context("Failed to setup citation policy")?;

        let stats = storage
            .store_records(&records)
            .await
            .context("Failed to store records")?;

        let duration = start_time.elapsed();

        info!(
            "Pipeline complete for file {}: {} records, {} stored, {} mappings, {} bytes uploaded in {:.2}s",
            filename,
            stats.total,
            stats.stored,
            stats.mappings_created,
            stats.bytes_uploaded,
            duration.as_secs_f64()
        );

        Ok(PipelineResult {
            data_source_id: Uuid::new_v4(),
            release: release.to_string(),
            division,
            records_processed: stats.total,
            sequences_inserted: stats.stored,
            mappings_created: stats.mappings_created,
            bytes_uploaded: stats.bytes_uploaded,
            duration_seconds: duration.as_secs_f64(),
        })
    }

    /// Extract division name from filename
    /// e.g., "gbvrl1.seq.gz" -> "viral"
    fn extract_division_from_filename(filename: &str) -> String {
        if filename.starts_with("gbvrl") {
            "viral".to_string()
        } else if filename.starts_with("gbbct") {
            "bacterial".to_string()
        } else if filename.starts_with("gbphg") {
            "phage".to_string()
        } else if filename.starts_with("gbpln") {
            "plant".to_string()
        } else if filename.starts_with("gbmam") {
            "mammalian".to_string()
        } else if filename.starts_with("gbpri") {
            "primate".to_string()
        } else if filename.starts_with("gbrod") {
            "rodent".to_string()
        } else if filename.starts_with("gbvrt") {
            "vertebrate".to_string()
        } else if filename.starts_with("gbinv") {
            "invertebrate".to_string()
        } else {
            "unknown".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_division_from_filename() {
        assert_eq!(
            GenbankPipeline::extract_division_from_filename("gbvrl1.seq.gz"),
            "viral"
        );
        assert_eq!(
            GenbankPipeline::extract_division_from_filename("gbbct1.seq.gz"),
            "bacterial"
        );
        assert_eq!(
            GenbankPipeline::extract_division_from_filename("gbphg1.seq.gz"),
            "phage"
        );
    }
}
