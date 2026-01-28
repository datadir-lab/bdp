//! E2E test assertion helpers
//!
//! This module provides high-level assertion helpers for verifying the state
//! of the system after running ingestion jobs or other operations.
//!
//! # Example
//!
//! ```no_run
//! use bdp_server::e2e::E2EEnvironment;
//!
//! #[tokio::test]
//! async fn test_protein_ingestion() {
//!     let env = E2EEnvironment::new().await.unwrap();
//!     let assertions = env.assertions();
//!
//!     // Verify counts
//!     assert_eq!(assertions.count_proteins().await.unwrap(), 100);
//!     assert_eq!(assertions.count_organisms().await.unwrap(), 50);
//!
//!     // Verify specific data
//!     assertions
//!         .verify_protein_exists("P01308", "Insulin")
//!         .await
//!         .unwrap();
//!
//!     env.cleanup().await;
//! }
//! ```

use anyhow::{anyhow, bail, Context, Result};
use aws_sdk_s3::Client as S3Client;
use sqlx::PgPool;
use tracing::info;

/// E2E test assertions helper
///
/// Provides methods for verifying the state of the database and S3 storage
/// after test operations.
pub struct E2EAssertions {
    /// Database connection pool
    db: PgPool,
    /// S3 client
    s3: S3Client,
    /// S3 bucket name
    bucket: String,
}

impl E2EAssertions {
    /// Create a new assertions helper
    pub fn new(db: PgPool, s3: S3Client, bucket: String) -> Self {
        Self { db, s3, bucket }
    }

    /// Count total proteins in the database
    ///
    /// Queries the `protein_metadata` table and returns the total count.
    pub async fn count_proteins(&self) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM protein_metadata
            "#
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to count proteins")?;

        Ok(result.count.unwrap_or(0))
    }

    /// Count total organisms in the database
    ///
    /// Queries the `organisms` table and returns the total count.
    pub async fn count_organisms(&self) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM organisms
            "#
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to count organisms")?;

        Ok(result.count.unwrap_or(0))
    }

    /// Count total version files in the database
    ///
    /// Queries the `version_files` table and returns the total count.
    pub async fn count_version_files(&self) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM version_files
            "#
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to count version files")?;

        Ok(result.count.unwrap_or(0))
    }

    /// Count S3 files with a given prefix
    ///
    /// Lists S3 objects with the specified prefix and returns the count.
    ///
    /// # Arguments
    ///
    /// * `prefix` - S3 key prefix to filter objects
    pub async fn count_s3_files(&self, prefix: &str) -> Result<usize> {
        let response = self
            .s3
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await
            .context("Failed to list S3 objects")?;

        let count = response.contents().len();
        Ok(count)
    }

    /// Get a specific protein by accession
    ///
    /// Queries the database for a protein with the given accession number.
    ///
    /// # Arguments
    ///
    /// * `accession` - Protein accession (e.g., "P01308")
    ///
    /// # Returns
    ///
    /// Returns protein information if found.
    pub async fn get_protein(&self, accession: &str) -> Result<ProteinInfo> {
        let result = sqlx::query!(
            r#"
            SELECT
                accession,
                entry_name,
                protein_name,
                gene_name,
                sequence_length,
                mass_da,
                sequence_checksum
            FROM protein_metadata
            WHERE accession = $1
            "#,
            accession
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to fetch protein")?;

        Ok(ProteinInfo {
            accession: result.accession,
            entry_name: result.entry_name,
            protein_name: result.protein_name,
            gene_name: result.gene_name,
            sequence_length: result.sequence_length,
            mass_da: result.mass_da,
            sequence_checksum: result.sequence_checksum,
        })
    }

    /// Get job statistics
    ///
    /// Retrieves ingestion statistics for a specific job from the sync status table.
    ///
    /// # Arguments
    ///
    /// * `job_id` - Job ID (UUID)
    ///
    /// # Returns
    ///
    /// Returns job statistics if the job has completed.
    pub async fn get_job_stats(&self, job_id: &str) -> Result<JobStats> {
        let result = sqlx::query!(
            r#"
            SELECT
                total_entries,
                last_version,
                last_external_version,
                status,
                last_error
            FROM organization_sync_status
            WHERE last_job_id = $1
            "#,
            uuid::Uuid::parse_str(job_id)?
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to fetch job stats")?;

        Ok(JobStats {
            total_entries: result.total_entries.unwrap_or(0),
            last_version: result.last_version,
            last_external_version: result.last_external_version,
            status: result.status,
            last_error: result.last_error,
        })
    }

    /// Verify that a protein exists with expected properties
    ///
    /// Fetches the protein and verifies it has the expected name.
    /// This is a convenience method that combines fetching and assertion.
    ///
    /// # Arguments
    ///
    /// * `accession` - Protein accession
    /// * `expected_name` - Expected protein name (can be partial)
    ///
    /// # Errors
    ///
    /// Returns an error if the protein doesn't exist or the name doesn't match.
    pub async fn verify_protein_exists(&self, accession: &str, expected_name: &str) -> Result<()> {
        let protein = self.get_protein(accession).await?;

        let protein_name = protein
            .protein_name
            .ok_or_else(|| anyhow!("Protein has no name"))?;

        if !protein_name.contains(expected_name) {
            bail!(
                "Protein name mismatch: expected '{}' to contain '{}'",
                protein_name,
                expected_name
            );
        }

        info!("Verified protein exists: accession={}, name={}", accession, protein_name);
        Ok(())
    }

    /// Verify that an S3 file exists with expected checksum
    ///
    /// Checks if an S3 object exists and optionally verifies its ETag/checksum.
    ///
    /// # Arguments
    ///
    /// * `key` - S3 object key
    /// * `expected_checksum` - Optional expected checksum (MD5/SHA256)
    ///
    /// # Errors
    ///
    /// Returns an error if the file doesn't exist or checksum doesn't match.
    pub async fn verify_s3_file_exists(
        &self,
        key: &str,
        expected_checksum: Option<&str>,
    ) -> Result<()> {
        let response = self
            .s3
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context(format!("S3 file not found: {}", key))?;

        if let Some(expected) = expected_checksum {
            let etag = response
                .e_tag()
                .ok_or_else(|| anyhow!("S3 object has no ETag"))?;

            // Remove quotes from ETag if present
            let etag_clean = etag.trim_matches('"');

            if etag_clean != expected {
                bail!(
                    "S3 file checksum mismatch: key={}, expected={}, actual={}",
                    key,
                    expected,
                    etag_clean
                );
            }
        }

        info!("Verified S3 file exists: {}", key);
        Ok(())
    }

    /// Verify total counts match expected values
    ///
    /// Convenience method to check all major table counts at once.
    ///
    /// # Arguments
    ///
    /// * `expected` - Expected counts for each table
    pub async fn verify_counts(&self, expected: ExpectedCounts) -> Result<()> {
        if let Some(proteins) = expected.proteins {
            let actual = self.count_proteins().await?;
            if actual != proteins {
                bail!("Protein count mismatch: expected={}, actual={}", proteins, actual);
            }
            info!("Verified protein count: {}", actual);
        }

        if let Some(organisms) = expected.organisms {
            let actual = self.count_organisms().await?;
            if actual != organisms {
                bail!("Organism count mismatch: expected={}, actual={}", organisms, actual);
            }
            info!("Verified organism count: {}", actual);
        }

        if let Some(version_files) = expected.version_files {
            let actual = self.count_version_files().await?;
            if actual != version_files {
                bail!(
                    "Version files count mismatch: expected={}, actual={}",
                    version_files,
                    actual
                );
            }
            info!("Verified version files count: {}", actual);
        }

        Ok(())
    }

    /// Get organism by taxonomy ID
    ///
    /// Fetches an organism record by its NCBI taxonomy ID.
    pub async fn get_organism_by_taxonomy_id(&self, taxonomy_id: i32) -> Result<OrganismInfo> {
        let result = sqlx::query!(
            r#"
            SELECT
                id,
                ncbi_taxonomy_id,
                scientific_name,
                common_name
            FROM organisms
            WHERE ncbi_taxonomy_id = $1
            "#,
            taxonomy_id
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to fetch organism")?;

        Ok(OrganismInfo {
            id: result.id,
            taxonomy_id: result.ncbi_taxonomy_id,
            scientific_name: result.scientific_name,
            common_name: result.common_name,
        })
    }

    /// List all S3 objects with a prefix
    ///
    /// Returns a list of S3 object keys matching the prefix.
    pub async fn list_s3_objects(&self, prefix: &str) -> Result<Vec<String>> {
        let response = self
            .s3
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await
            .context("Failed to list S3 objects")?;

        let keys = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(|k| k.to_string()))
            .collect();

        Ok(keys)
    }

    /// Verify that the database is empty (useful for setup/teardown tests)
    pub async fn verify_database_empty(&self) -> Result<()> {
        let proteins = self.count_proteins().await?;
        let organisms = self.count_organisms().await?;
        let version_files = self.count_version_files().await?;

        if proteins > 0 || organisms > 0 || version_files > 0 {
            bail!(
                "Database not empty: proteins={}, organisms={}, version_files={}",
                proteins,
                organisms,
                version_files
            );
        }

        Ok(())
    }

    /// Assert that an organization exists
    pub async fn assert_organization_exists(&self, org_id: uuid::Uuid) -> Result<()> {
        let result = sqlx::query!("SELECT id FROM organizations WHERE id = $1", org_id)
            .fetch_optional(&self.db)
            .await
            .context("Failed to query organization")?;

        if result.is_none() {
            bail!("Organization {} does not exist", org_id);
        }

        Ok(())
    }

    /// Assert that data sources exist for an organization
    pub async fn assert_data_sources_exist(
        &self,
        org_id: uuid::Uuid,
        expected_count: usize,
    ) -> Result<Vec<DataSourceInfo>> {
        let result = sqlx::query_as!(
            DataSourceInfo,
            r#"
            SELECT
                re.id,
                re.slug,
                re.name,
                ds.source_type,
                (SELECT v.id FROM versions v WHERE v.entry_id = re.id ORDER BY v.created_at DESC LIMIT 1) as "latest_version?"
            FROM registry_entries re
            JOIN data_sources ds ON ds.id = re.id
            WHERE re.organization_id = $1
            "#,
            org_id
        )
        .fetch_all(&self.db)
        .await
        .context("Failed to query data sources")?;

        if result.len() < expected_count {
            bail!("Expected at least {} data sources, found {}", expected_count, result.len());
        }

        Ok(result)
    }

    /// Assert that versions exist for a data source
    pub async fn assert_versions_exist(
        &self,
        data_source_id: uuid::Uuid,
        expected_count: usize,
    ) -> Result<Vec<VersionInfo>> {
        let result = sqlx::query_as!(
            VersionInfo,
            r#"
            SELECT id, entry_id, version, external_version
            FROM versions
            WHERE entry_id = $1
            "#,
            data_source_id
        )
        .fetch_all(&self.db)
        .await
        .context("Failed to query versions")?;

        if result.len() < expected_count {
            bail!("Expected at least {} versions, found {}", expected_count, result.len());
        }

        Ok(result)
    }

    /// Count proteins for a specific data source and version
    pub async fn count_proteins(
        &self,
        data_source_id: uuid::Uuid,
        version_id: uuid::Uuid,
    ) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM protein_metadata pm
            JOIN version_files vf ON vf.id = pm.version_file_id
            WHERE vf.version_id = $1 AND vf.data_source_id = $2
            "#,
            version_id,
            data_source_id
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to count proteins")?;

        Ok(result.count.unwrap_or(0))
    }

    /// Assert that a protein exists with the given accession
    pub async fn assert_protein_exists(
        &self,
        data_source_id: uuid::Uuid,
        version_id: uuid::Uuid,
        accession: &str,
    ) -> Result<ProteinInfo> {
        let result = sqlx::query_as!(
            ProteinInfo,
            r#"
            SELECT
                pm.accession as "accession!",
                pm.entry_name,
                pm.protein_name,
                pm.gene_name,
                pm.sequence_length,
                pm.mass_da,
                pm.sequence_checksum
            FROM protein_metadata pm
            JOIN version_files vf ON vf.id = pm.version_file_id
            WHERE vf.version_id = $1
              AND vf.data_source_id = $2
              AND pm.accession = $3
            LIMIT 1
            "#,
            version_id,
            data_source_id,
            accession
        )
        .fetch_optional(&self.db)
        .await
        .context("Failed to query protein")?;

        result.ok_or_else(|| anyhow!("Protein {} not found", accession))
    }
}

/// Data source information
#[derive(Debug, Clone)]
pub struct DataSourceInfo {
    pub id: uuid::Uuid,
    pub slug: String,
    pub name: String,
    pub source_type: String,
    pub latest_version: Option<uuid::Uuid>,
}

/// Version information
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub id: uuid::Uuid,
    pub entry_id: uuid::Uuid,
    pub version: String,
    pub external_version: Option<String>,
}

/// Protein information
#[derive(Debug, Clone)]
pub struct ProteinInfo {
    pub accession: String,
    pub entry_name: Option<String>,
    pub protein_name: Option<String>,
    pub gene_name: Option<String>,
    pub sequence_length: Option<i32>,
    pub mass_da: Option<i64>,
    pub sequence_checksum: Option<String>,
}

/// Job statistics
#[derive(Debug, Clone)]
pub struct JobStats {
    pub total_entries: i64,
    pub last_version: Option<String>,
    pub last_external_version: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
}

/// Organism information
#[derive(Debug, Clone)]
pub struct OrganismInfo {
    pub id: uuid::Uuid,
    pub taxonomy_id: i32,
    pub scientific_name: String,
    pub common_name: Option<String>,
}

/// Expected counts for verification
#[derive(Debug, Default)]
pub struct ExpectedCounts {
    pub proteins: Option<i64>,
    pub organisms: Option<i64>,
    pub version_files: Option<i64>,
}

impl ExpectedCounts {
    /// Create a new expected counts builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set expected protein count
    pub fn proteins(mut self, count: i64) -> Self {
        self.proteins = Some(count);
        self
    }

    /// Set expected organism count
    pub fn organisms(mut self, count: i64) -> Self {
        self.organisms = Some(count);
        self
    }

    /// Set expected version files count
    pub fn version_files(mut self, count: i64) -> Self {
        self.version_files = Some(count);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expected_counts_builder() {
        let counts = ExpectedCounts::new()
            .proteins(100)
            .organisms(50)
            .version_files(200);

        assert_eq!(counts.proteins, Some(100));
        assert_eq!(counts.organisms, Some(50));
        assert_eq!(counts.version_files, Some(200));
    }
}
