//! Storage layer for UniProt parsed data
//!
//! Creates individual data sources for each protein with proper schema structure.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::models::UniProtEntry;
use super::taxonomy_helper::TaxonomyHelper;
use crate::ingest::citations::{setup_citation_policy, uniprot_policy};
use crate::storage::Storage;
use std::collections::HashMap;

// ============================================================================
// UniProt Storage Constants
// ============================================================================

/// Number of concurrent S3 uploads to process at once.
/// Higher values improve throughput but increase memory usage.
pub const S3_UPLOAD_BATCH_SIZE: usize = 50;

/// Number of entries to process in each database micro-transaction.
/// Smaller values reduce memory usage and improve failure isolation.
pub const DB_MICRO_BATCH_SIZE: usize = 10;

/// Maximum number of features/cross-references/comments to insert in a single batch.
/// Limited by PostgreSQL parameter count limits.
pub const MAX_INSERT_BATCH_SIZE: usize = 100;

/// Maximum number of publications to insert in a single batch.
/// Smaller than MAX_INSERT_BATCH_SIZE due to more columns per row.
pub const MAX_PUBLICATION_BATCH_SIZE: usize = 50;

/// Maximum number of dependencies to insert in a single batch.
pub const DEPENDENCY_BATCH_SIZE: usize = 1000;

/// Maximum slug length for taxonomy entries.
pub const MAX_SLUG_LENGTH: usize = 100;

/// Convert organism taxonomy to human-readable slug
///
/// Examples:
/// - "Homo sapiens (Human)" → "homo-sapiens"
/// - "Escherichia coli K-12" → "escherichia-coli-k-12"
/// - "Human immunodeficiency virus 1" → "human-immunodeficiency-virus-1"
fn taxonomy_to_slug(organism_name: &str, taxonomy_id: i32) -> String {
    // Remove parenthetical suffix like "(Human)"
    let name = organism_name
        .split('(')
        .next()
        .unwrap_or(organism_name)
        .trim()
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    // Fallback for empty or too long names
    if name.is_empty() || name.len() > MAX_SLUG_LENGTH {
        format!("taxon-{}", taxonomy_id)
    } else {
        name
    }
}

/// Classify organism source type based on taxonomic lineage
///
/// Uses the first element of the taxonomic lineage to determine the source type:
/// - "Viruses" → "virus"
/// - "Bacteria" → "bacteria"
/// - "Archaea" → "archaea"
/// - "Eukaryota" → "organism"
/// - Unknown/empty → "organism" (fallback)
///
/// # Example
/// ```
/// let lineage = vec!["Viruses".to_string(), "Riboviria".to_string()];
/// assert_eq!(classify_source_type(&lineage), "virus");
///
/// let lineage = vec!["Eukaryota".to_string(), "Metazoa".to_string()];
/// assert_eq!(classify_source_type(&lineage), "organism");
/// ```
///
/// Classify source type based on taxonomic lineage
///
/// This is now integrated with NCBI Taxonomy database via TaxonomyHelper
#[allow(dead_code)]
fn classify_source_type(lineage: &[String]) -> &'static str {
    match lineage.first().map(|s| s.as_str()) {
        Some("Viruses") => "virus",
        Some("Bacteria") => "bacteria",
        Some("Archaea") => "archaea",
        Some("Eukaryota") => "organism",
        _ => "organism", // Fallback for unknown/malformed
    }
}

/// Storage handler for UniProt data
///
/// Responsible for persisting parsed UniProt entries to the database
/// and optionally uploading FASTA sequences to S3 storage.
///
/// # Database Schema
///
/// Uses the following tables:
/// - `registry_entries`: Main entry for each protein
/// - `data_sources`: Type-specific metadata (protein, organism)
/// - `protein_metadata`: Protein-specific fields (accession, sequence, etc.)
/// - `taxonomy_metadata`: Organism/taxonomy information
/// - `protein_features`, `cross_references`, `comments`: Related data
/// - `protein_publications`: Literature references
///
/// # Transaction Handling
///
/// Uses micro-transactions for efficient memory usage and failure isolation.
/// S3 uploads happen before database transactions to ensure data consistency.
pub struct UniProtStorage {
    db: PgPool,
    s3: Option<Storage>,
    organization_id: Uuid,
    internal_version: String,
    external_version: String,
}

impl UniProtStorage {
    /// Create a new storage handler (database only, no S3)
    ///
    /// # Arguments
    ///
    /// * `db` - PostgreSQL connection pool
    /// * `organization_id` - UniProt organization ID in the database
    /// * `internal_version` - BDP internal version (semantic version)
    /// * `external_version` - UniProt release version (e.g., "2024_03")
    pub fn new(
        db: PgPool,
        organization_id: Uuid,
        internal_version: String,
        external_version: String,
    ) -> Self {
        Self {
            db,
            s3: None,
            organization_id,
            internal_version,
            external_version,
        }
    }

    /// Create storage handler with S3 support for FASTA uploads
    ///
    /// When S3 is configured, FASTA sequences are uploaded alongside
    /// database records for efficient file serving.
    pub fn with_s3(
        db: PgPool,
        s3: Storage,
        organization_id: Uuid,
        internal_version: String,
        external_version: String,
    ) -> Self {
        Self {
            db,
            s3: Some(s3),
            organization_id,
            internal_version,
            external_version,
        }
    }

    /// Set up citation policy for UniProt organization (idempotent)
    ///
    /// This should be called once during pipeline initialization to ensure
    /// citation policy is properly configured for the organization.
    pub async fn setup_citations(&self) -> Result<()> {
        let policy_config = uniprot_policy(self.organization_id, None);
        setup_citation_policy(&self.db, &policy_config).await?;
        info!("UniProt citation policy configured");
        Ok(())
    }

    /// Store a batch of parsed entries
    ///
    /// Processes entries in two phases:
    /// 1. Upload FASTA files to S3 (if configured) in parallel batches
    /// 2. Store metadata in the database using micro-transactions
    ///
    /// # Arguments
    ///
    /// * `entries` - Parsed UniProt entries to store
    ///
    /// # Returns
    ///
    /// Returns the number of entries successfully stored.
    ///
    /// # Errors
    ///
    /// Returns an error if S3 uploads or database transactions fail.
    pub async fn store_entries(&self, entries: &[UniProtEntry]) -> Result<usize> {
        info!("Storing {} UniProt entries", entries.len());

        // STEP 1: Upload ALL entries to S3 FIRST (before any database transaction!)
        // Use parallel uploads with batching for 10-20x speedup
        if let Some(ref s3) = self.s3 {
            let internal_ver = self.internal_version.clone();
            let org_id = self.organization_id;

            // Process in batches of 50 concurrent uploads
            let chunks: Vec<_> = entries.chunks(S3_UPLOAD_BATCH_SIZE).collect();
            for chunk in chunks {
                let mut upload_futures = Vec::new();

                for entry in chunk {
                    let s3_clone = s3.clone();
                    let int_ver_clone = internal_ver.clone();
                    let entry_clone = entry.clone();

                    let upload_future = async move {
                        let result = Self::upload_entry_to_s3_static(
                            &s3_clone,
                            &entry_clone,
                            org_id,
                            &int_ver_clone,
                            "",
                        )
                        .await;
                        (entry_clone.accession.clone(), result)
                    };

                    upload_futures.push(upload_future);
                }

                // Wait for this batch to complete
                let results = futures::future::join_all(upload_futures).await;

                // Log any failures
                for (accession, result) in results {
                    if let Err(e) = result {
                        warn!(accession = %accession, error = %e, "S3 upload failed (non-fatal)");
                    }
                }
            }
        }

        // STEP 2: Now do database work - process in smaller sub-batches
        // Use mini-transactions instead of savepoints for better performance
        let mut stored_count = 0;
        let mut error_count = 0;
        let mut errors = Vec::new();

        // Process entries in micro-batches (10 at a time) with separate transactions
        // This balances between transaction overhead and failure isolation
        for chunk in entries.chunks(DB_MICRO_BATCH_SIZE) {
            let mut tx = self
                .db
                .begin()
                .await
                .context("Failed to begin transaction")?;

            let mut chunk_success = true;
            for entry in chunk {
                match self.store_entry_tx(&mut tx, entry).await {
                    Ok(_) => {
                        stored_count += 1;
                    },
                    Err(e) => {
                        // On error, rollback this transaction and retry entries individually
                        chunk_success = false;
                        error!(
                            accession = %entry.accession,
                            error = %e,
                            "Failed to store entry in batch, will retry individually"
                        );
                        break;
                    },
                }
            }

            if chunk_success {
                // Commit the successful batch
                tx.commit()
                    .await
                    .context("Failed to commit batch transaction")?;
            } else {
                // Rollback and retry failed entries one-by-one
                drop(tx); // Explicit rollback by dropping

                for entry in chunk {
                    let mut retry_tx = self
                        .db
                        .begin()
                        .await
                        .context("Failed to begin retry transaction")?;
                    match self.store_entry_tx(&mut retry_tx, entry).await {
                        Ok(_) => {
                            retry_tx
                                .commit()
                                .await
                                .context("Failed to commit retry transaction")?;
                            stored_count += 1;
                        },
                        Err(e) => {
                            error_count += 1;
                            error!(
                                accession = %entry.accession,
                                error = %e,
                                error_chain = ?e.chain().collect::<Vec<_>>(),
                                "Failed to store entry after retry"
                            );

                            // Collect first 5 errors for debugging
                            const MAX_ERRORS_TO_COLLECT: usize = 5;
                            #[allow(clippy::excessive_nesting)]
                            if errors.len() < MAX_ERRORS_TO_COLLECT {
                                errors.push((entry.accession.clone(), format!("{:#}", e)));
                            }
                        },
                    }
                }
            }
        }

        // Log error summary
        if error_count > 0 {
            error!(
                stored = stored_count,
                failed = error_count,
                total = entries.len(),
                "Storage completed with errors"
            );

            for (accession, err) in &errors {
                error!(
                    accession = %accession,
                    error = %err,
                    "Sample error"
                );
            }

            if error_count > 5 {
                error!(additional_errors = error_count - 5, "Additional errors not shown");
            }
        }

        info!("Successfully stored {}/{} entries", stored_count, entries.len());
        Ok(stored_count)
    }

    /// Store a single entry within a transaction
    ///
    /// Creates: registry_entry -> data_source -> protein_metadata -> version -> version_file
    async fn store_entry_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry: &UniProtEntry,
    ) -> Result<()> {
        debug!("Storing protein: {}", entry.accession);

        // 1. Get or create organism
        let organism_id = self.get_or_create_organism_tx(tx, entry).await?;

        // 2. Create registry entry (each protein is its own entry)
        let entry_id = self.create_registry_entry_tx(tx, entry).await?;

        // 3. Create data source with validation
        self.create_data_source_tx(tx, entry_id, entry, organism_id)
            .await?;

        // 4. Create protein metadata (with sequence deduplication and organism reference)
        self.create_protein_metadata_tx(tx, entry_id, entry, organism_id)
            .await?;

        // 5. Create version with semantic versioning
        let version_id = self.create_version_tx(tx, entry_id).await?;

        // 6. Create version files for multiple formats (DAT, FASTA, JSON)
        self.create_version_files_tx(tx, entry, version_id).await?;

        debug!("Successfully stored protein: {}", entry.accession);
        Ok(())
    }

    /// Get or create organism as a data source (within transaction)
    ///
    /// Organisms are now data sources with taxonomy_metadata, not a separate organisms table
    /// Get or create organism (taxonomy) entry using TaxonomyHelper
    ///
    /// This method integrates with NCBI Taxonomy database:
    /// - First checks if taxonomy entry exists within the current transaction
    /// - If not found, uses TaxonomyHelper with pool to create stub (outside transaction)
    /// - Returns the data_source_id for use as a foreign key
    ///
    /// Note: Taxonomy stub creation happens outside the transaction for isolation
    async fn get_or_create_organism_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry: &UniProtEntry,
    ) -> Result<Uuid> {
        let ncbi_taxonomy_id = entry.taxonomy_id;

        // Check if taxonomy already exists (most common case - fast path)
        let existing = sqlx::query_scalar::<_, Uuid>(
            "SELECT data_source_id FROM taxonomy_metadata WHERE taxonomy_id = $1",
        )
        .bind(ncbi_taxonomy_id)
        .fetch_optional(&mut **tx)
        .await?;

        if let Some(id) = existing {
            return Ok(id);
        }

        // Not found - use TaxonomyHelper to create stub using the pool
        // (This happens outside the current transaction for isolation)
        let mut taxonomy_helper = TaxonomyHelper::new(self.db.clone(), self.organization_id);
        let data_source_id = taxonomy_helper
            .get_or_create_taxonomy(ncbi_taxonomy_id, &entry.organism_name, &entry.taxonomy_lineage)
            .await
            .context("Failed to get or create taxonomy via TaxonomyHelper")?;

        Ok(data_source_id)
    }

    /// Create registry entry for the protein (within transaction)
    async fn create_registry_entry_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry: &UniProtEntry,
    ) -> Result<Uuid> {
        let slug = &entry.accession;
        let name = format!("{} [{}]", entry.protein_name, entry.organism_name);
        let description = format!("UniProt protein: {}", entry.protein_name);

        let entry_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            ON CONFLICT (slug) DO UPDATE SET updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(self.organization_id)
        .bind(slug)
        .bind(&name)
        .bind(&description)
        .fetch_one(&mut **tx)
        .await
        .context("Failed to create registry_entry")?;

        Ok(entry_id)
    }

    /// Create data source record (within transaction)
    async fn create_data_source_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry_id: Uuid,
        entry: &UniProtEntry,
        _taxonomy_id: Uuid,
    ) -> Result<()> {
        debug!(
            "Creating data_source for protein: {} with entry_id: {}",
            entry.accession, entry_id
        );
        let source_type = "protein";

        // Note: organism_id column was removed in migration 20260119000003
        // The relationship is now through protein_metadata.organism_id foreign key
        sqlx::query(
            r#"
            INSERT INTO data_sources (id, source_type, external_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(entry_id)
        .bind(source_type)
        .bind(&entry.accession)
        .execute(&mut **tx)
        .await
        .with_context(|| {
            format!(
                "Failed to create data_source for accession {} with id {}",
                entry.accession, entry_id
            )
        })?;

        debug!("Successfully created data_source for protein: {}", entry.accession);
        Ok(())
    }

    /// Create protein_metadata record with deduplicated sequence (within transaction)
    async fn create_protein_metadata_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        data_source_id: Uuid,
        entry: &UniProtEntry,
        taxonomy_id: Uuid,
    ) -> Result<()> {
        // 1. Get or create deduplicated sequence
        let sequence_id = self.get_or_create_sequence_tx(tx, entry).await?;

        // 2. Compute sequence checksum for metadata
        let mut hasher = Sha256::new();
        hasher.update(entry.sequence.as_bytes());
        let sequence_checksum = format!("{:x}", hasher.finalize());

        // 3. Insert protein metadata (with extended metadata and dates)
        sqlx::query(
            r#"
            INSERT INTO protein_metadata (
                data_source_id, accession, entry_name, protein_name, gene_name,
                sequence_length, mass_da, sequence_checksum,
                sequence_id, taxonomy_id, uniprot_version,
                alternative_names, ec_numbers, protein_existence, keywords,
                organelle, organism_hosts,
                entry_created, sequence_updated, annotation_updated
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
            ON CONFLICT (data_source_id) DO UPDATE SET
                alternative_names = EXCLUDED.alternative_names,
                ec_numbers = EXCLUDED.ec_numbers,
                protein_existence = EXCLUDED.protein_existence,
                keywords = EXCLUDED.keywords,
                organelle = EXCLUDED.organelle,
                organism_hosts = EXCLUDED.organism_hosts,
                entry_created = EXCLUDED.entry_created,
                sequence_updated = EXCLUDED.sequence_updated,
                annotation_updated = EXCLUDED.annotation_updated
            "#
        )
        .bind(data_source_id)
        .bind(&entry.accession)
        .bind(&entry.entry_name)
        .bind(&entry.protein_name)
        .bind(&entry.gene_name)
        .bind(entry.sequence_length)
        .bind(entry.mass_da)
        .bind(&sequence_checksum)
        .bind(sequence_id)
        .bind(taxonomy_id)
        .bind(&self.external_version)
        .bind(&entry.alternative_names)
        .bind(&entry.ec_numbers)
        .bind(entry.protein_existence)
        .bind(&entry.keywords)
        .bind(&entry.organelle)
        .bind(&entry.organism_hosts)
        .bind(entry.entry_created)
        .bind(entry.sequence_updated)
        .bind(entry.annotation_updated)
        .execute(&mut **tx)
        .await
        .context("Failed to create protein_metadata")?;

        // 4. Insert protein features
        self.store_features_tx(tx, data_source_id, entry).await?;

        // 5. Insert cross-references
        self.store_cross_references_tx(tx, data_source_id, entry)
            .await?;

        // 6. Insert comments
        self.store_comments_tx(tx, data_source_id, entry).await?;

        // 7. Insert publications
        self.store_publications_tx(tx, data_source_id, entry)
            .await?;

        Ok(())
    }

    /// Store protein features (within transaction)
    async fn store_features_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        protein_id: Uuid,
        entry: &UniProtEntry,
    ) -> Result<()> {
        if entry.features.is_empty() {
            return Ok(());
        }

        // Delete existing features for this protein (for updates)
        sqlx::query("DELETE FROM protein_features WHERE protein_id = $1")
            .bind(protein_id)
            .execute(&mut **tx)
            .await?;

        // Batch insert features (max 100 at a time to avoid parameter limit)
        for chunk in entry.features.chunks(MAX_INSERT_BATCH_SIZE) {
            let mut query_builder = sqlx::QueryBuilder::new(
                "INSERT INTO protein_features (protein_id, feature_type, start_pos, end_pos, description) "
            );

            query_builder.push_values(chunk, |mut b, feature| {
                b.push_bind(protein_id)
                    .push_bind(&feature.feature_type)
                    .push_bind(feature.start_pos)
                    .push_bind(feature.end_pos)
                    .push_bind(&feature.description);
            });

            query_builder.build().execute(&mut **tx).await?;
        }

        Ok(())
    }

    /// Store protein cross-references (within transaction)
    async fn store_cross_references_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        protein_id: Uuid,
        entry: &UniProtEntry,
    ) -> Result<()> {
        if entry.cross_references.is_empty() {
            return Ok(());
        }

        // Delete existing cross-references for this protein
        sqlx::query("DELETE FROM protein_cross_references WHERE protein_id = $1")
            .bind(protein_id)
            .execute(&mut **tx)
            .await?;

        // Batch insert cross-references (max 100 at a time)
        for chunk in entry.cross_references.chunks(MAX_INSERT_BATCH_SIZE) {
            let mut query_builder = sqlx::QueryBuilder::new(
                "INSERT INTO protein_cross_references (protein_id, database, database_id, metadata) "
            );

            query_builder.push_values(chunk, |mut b, xref| {
                let metadata_json =
                    serde_json::to_value(&xref.metadata).unwrap_or(serde_json::json!([]));
                b.push_bind(protein_id)
                    .push_bind(&xref.database)
                    .push_bind(&xref.database_id)
                    .push_bind(metadata_json);
            });

            query_builder.build().execute(&mut **tx).await?;
        }

        Ok(())
    }

    /// Store protein comments (within transaction)
    async fn store_comments_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        protein_id: Uuid,
        entry: &UniProtEntry,
    ) -> Result<()> {
        if entry.comments.is_empty() {
            return Ok(());
        }

        // Delete existing comments for this protein
        sqlx::query("DELETE FROM protein_comments WHERE protein_id = $1")
            .bind(protein_id)
            .execute(&mut **tx)
            .await?;

        // Batch insert comments (max 100 at a time)
        for chunk in entry.comments.chunks(MAX_INSERT_BATCH_SIZE) {
            let mut query_builder =
                sqlx::QueryBuilder::new("INSERT INTO protein_comments (protein_id, topic, text) ");

            query_builder.push_values(chunk, |mut b, comment| {
                b.push_bind(protein_id)
                    .push_bind(&comment.topic)
                    .push_bind(&comment.text);
            });

            query_builder.build().execute(&mut **tx).await?;
        }

        Ok(())
    }

    /// Store protein publications (within transaction)
    async fn store_publications_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        protein_id: Uuid,
        entry: &UniProtEntry,
    ) -> Result<()> {
        if entry.publications.is_empty() {
            return Ok(());
        }

        // Delete existing publications for this protein
        sqlx::query("DELETE FROM protein_publications WHERE protein_id = $1")
            .bind(protein_id)
            .execute(&mut **tx)
            .await?;

        // Batch insert publications (max 50 at a time due to many columns)
        for chunk in entry.publications.chunks(MAX_PUBLICATION_BATCH_SIZE) {
            let mut query_builder = sqlx::QueryBuilder::new(
                "INSERT INTO protein_publications (protein_id, reference_number, position, comments, pubmed_id, doi, author_group, authors, title, location) "
            );

            query_builder.push_values(chunk, |mut b, pub_ref| {
                b.push_bind(protein_id)
                    .push_bind(pub_ref.reference_number)
                    .push_bind(&pub_ref.position)
                    .push_bind(&pub_ref.comments)
                    .push_bind(&pub_ref.pubmed_id)
                    .push_bind(&pub_ref.doi)
                    .push_bind(&pub_ref.author_group)
                    .push_bind(&pub_ref.authors)
                    .push_bind(&pub_ref.title)
                    .push_bind(&pub_ref.location);
            });

            query_builder.build().execute(&mut **tx).await?;
        }

        Ok(())
    }

    /// Get or create deduplicated protein sequence (within transaction)
    async fn get_or_create_sequence_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry: &UniProtEntry,
    ) -> Result<Uuid> {
        // Compute sequence hash (SHA256)
        let mut hasher = Sha256::new();
        hasher.update(entry.sequence.as_bytes());
        let sequence_hash = format!("{:x}", hasher.finalize());

        // Check if sequence already exists
        let existing = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM protein_sequences WHERE sequence_hash = $1",
        )
        .bind(&sequence_hash)
        .fetch_optional(&mut **tx)
        .await?;

        if let Some(id) = existing {
            debug!("Reusing existing sequence (hash: {})", &sequence_hash[..16]);
            return Ok(id);
        }

        // Compute MD5 for backward compatibility
        let sequence_md5 = format!("{:x}", md5::compute(entry.sequence.as_bytes()));

        // Create new sequence
        let sequence_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO protein_sequences (sequence, sequence_hash, sequence_length, sequence_md5)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (sequence_hash) DO UPDATE SET sequence_hash = EXCLUDED.sequence_hash
            RETURNING id
            "#,
        )
        .bind(&entry.sequence)
        .bind(&sequence_hash)
        .bind(entry.sequence_length)
        .bind(&sequence_md5)
        .fetch_one(&mut **tx)
        .await
        .context("Failed to create protein_sequence")?;

        debug!("Created new deduplicated sequence (hash: {})", &sequence_hash[..16]);
        Ok(sequence_id)
    }

    /// Create version record with semantic versioning (within transaction)
    async fn create_version_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry_id: Uuid,
    ) -> Result<Uuid> {
        // Parse internal version (e.g., "1.0" → major=1, minor=0, patch=0)
        let version_parts: Vec<&str> = self.internal_version.split('.').collect();
        let version_major = version_parts
            .first()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1);
        let version_minor = version_parts
            .get(1)
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        let version_patch = version_parts
            .get(2)
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);

        let version_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO versions (
                entry_id, version, external_version,
                version_major, version_minor, version_patch
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (entry_id, version) DO UPDATE
            SET external_version = EXCLUDED.external_version,
                version_major = EXCLUDED.version_major,
                version_minor = EXCLUDED.version_minor,
                version_patch = EXCLUDED.version_patch,
                updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(entry_id)
        .bind(&self.internal_version)
        .bind(&self.external_version)
        .bind(version_major)
        .bind(version_minor)
        .bind(version_patch)
        .fetch_one(&mut **tx)
        .await
        .context("Failed to create version")?;

        Ok(version_id)
    }

    /// Upload entry files to S3 BEFORE transaction (non-blocking for database)
    ///
    /// This MUST be called BEFORE starting any database transaction.
    #[allow(dead_code)]
    async fn upload_entry_to_s3(&self, entry: &UniProtEntry) -> Result<()> {
        if let Some(ref s3) = self.s3 {
            Self::upload_entry_to_s3_static(
                s3,
                entry,
                self.organization_id,
                &self.internal_version,
                &self.external_version,
            )
            .await
        } else {
            Ok(())
        }
    }

    /// Static version of S3 upload for parallel processing
    async fn upload_entry_to_s3_static(
        s3: &Storage,
        entry: &UniProtEntry,
        _org_id: Uuid,
        internal_version: &str,
        _external_version: &str,
    ) -> Result<()> {
        let base_path = format!("proteins/uniprot/{}/{}", entry.accession, internal_version);

        // Upload DAT
        let dat_content = entry.sequence.as_bytes().to_vec();
        let dat_key = format!("{}/{}.dat", base_path, entry.accession);
        if let Err(e) = s3
            .upload(&dat_key, dat_content, Some("text/plain".to_string()))
            .await
        {
            warn!(accession = %entry.accession, error = %e, "Failed to upload DAT to S3");
        }

        // Upload FASTA
        let fasta_content = entry.to_fasta();
        let fasta_key = format!("{}/{}.fasta", base_path, entry.accession);
        if let Err(e) = s3
            .upload(&fasta_key, fasta_content.as_bytes().to_vec(), Some("text/plain".to_string()))
            .await
        {
            warn!(accession = %entry.accession, error = %e, "Failed to upload FASTA to S3");
        }

        // Upload JSON
        let json_content = entry.to_json()?;
        let json_key = format!("{}/{}.json", base_path, entry.accession);
        if let Err(e) = s3
            .upload(
                &json_key,
                json_content.as_bytes().to_vec(),
                Some("application/json".to_string()),
            )
            .await
        {
            warn!(accession = %entry.accession, error = %e, "Failed to upload JSON to S3");
        }

        Ok(())
    }

    /// Create version_file records for multiple formats (within transaction)
    ///
    /// NOTE: S3 uploads MUST be done BEFORE calling this (see upload_entry_to_s3)
    async fn create_version_files_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry: &UniProtEntry,
        version_id: Uuid,
    ) -> Result<()> {
        let base_path = format!("proteins/uniprot/{}/{}", entry.accession, self.internal_version);

        // 1. DAT format - just create DB record (S3 upload already done)
        let dat_content = entry.sequence.as_bytes().to_vec();
        let dat_checksum = self.compute_checksum(&dat_content);
        let dat_key = format!("{}/{}.dat", base_path, entry.accession);

        self.insert_version_file_tx(
            tx,
            version_id,
            "dat",
            &dat_key,
            &dat_checksum,
            dat_content.len() as i64,
        )
        .await?;

        // 2. FASTA format - just create DB record
        let fasta_content = entry.to_fasta();
        let fasta_checksum = self.compute_checksum(fasta_content.as_bytes());
        let fasta_key = format!("{}/{}.fasta", base_path, entry.accession);

        self.insert_version_file_tx(
            tx,
            version_id,
            "fasta",
            &fasta_key,
            &fasta_checksum,
            fasta_content.len() as i64,
        )
        .await?;

        // 3. JSON format - just create DB record
        let json_content = entry.to_json()?;
        let json_checksum = self.compute_checksum(json_content.as_bytes());
        let json_key = format!("{}/{}.json", base_path, entry.accession);

        self.insert_version_file_tx(
            tx,
            version_id,
            "json",
            &json_key,
            &json_checksum,
            json_content.len() as i64,
        )
        .await?;

        Ok(())
    }

    /// Insert a single version_file record (within transaction)
    async fn insert_version_file_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        version_id: Uuid,
        format: &str,
        s3_key: &str,
        checksum: &str,
        size_bytes: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (version_id, format) DO NOTHING
            "#,
        )
        .bind(version_id)
        .bind(format)
        .bind(s3_key)
        .bind(checksum)
        .bind(size_bytes)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("Failed to create version_file for format: {}", format))?;

        Ok(())
    }

    /// Compute SHA-256 checksum
    fn compute_checksum(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Create an aggregate source that depends on all stored proteins
    ///
    /// This creates a special registry entry (slug: "uniprot-all") that has dependencies
    /// on all individual protein entries for this version.
    pub async fn create_aggregate_source(&self, protein_count: usize) -> Result<Uuid> {
        info!("Creating aggregate source for {} proteins", protein_count);

        // 1. Create registry entry for aggregate
        let slug = "uniprot-all";
        let name = format!("All UniProt Proteins ({})", self.external_version);
        let description = format!(
            "Complete UniProt {} release with {} proteins",
            self.external_version, protein_count
        );

        let entry_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            ON CONFLICT (slug) DO UPDATE
            SET name = EXCLUDED.name,
                description = EXCLUDED.description,
                updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(self.organization_id)
        .bind(slug)
        .bind(&name)
        .bind(&description)
        .fetch_one(&self.db)
        .await
        .context("Failed to create aggregate registry_entry")?;

        debug!("Created aggregate registry entry: {}", entry_id);

        // 2. Create data source with 'bundle' type
        sqlx::query(
            r#"
            INSERT INTO data_sources (id, source_type)
            VALUES ($1, 'bundle')
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(entry_id)
        .execute(&self.db)
        .await
        .context("Failed to create aggregate data_source")?;

        // 3. Create version with semantic versioning
        let version_parts: Vec<&str> = self.internal_version.split('.').collect();
        let version_major = version_parts
            .first()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1);
        let version_minor = version_parts
            .get(1)
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        let version_patch = version_parts
            .get(2)
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);

        let version_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO versions (
                entry_id, version, external_version, dependency_count,
                version_major, version_minor, version_patch
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (entry_id, version) DO UPDATE
            SET external_version = EXCLUDED.external_version,
                dependency_count = EXCLUDED.dependency_count,
                version_major = EXCLUDED.version_major,
                version_minor = EXCLUDED.version_minor,
                version_patch = EXCLUDED.version_patch,
                updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(entry_id)
        .bind(&self.internal_version)
        .bind(&self.external_version)
        .bind(protein_count as i32)
        .bind(version_major)
        .bind(version_minor)
        .bind(version_patch)
        .fetch_one(&self.db)
        .await
        .context("Failed to create aggregate version")?;

        debug!("Created aggregate version: {}", version_id);

        // 4. Create dependencies to all proteins
        self.create_aggregate_dependencies(version_id).await?;

        info!("✅ Created aggregate source: {}", slug);
        Ok(entry_id)
    }

    /// Create dependencies from aggregate to all individual proteins
    async fn create_aggregate_dependencies(&self, aggregate_version_id: Uuid) -> Result<()> {
        info!("Creating dependencies for aggregate");

        // Get all protein registry entries for this organization
        let protein_entries = sqlx::query_as::<_, (Uuid,)>(
            r#"
            SELECT DISTINCT re.id
            FROM registry_entries re
            JOIN data_sources ds ON ds.id = re.id
            JOIN protein_metadata pm ON pm.data_source_id = ds.id
            WHERE re.organization_id = $1
              AND re.entry_type = 'data_source'
              AND re.slug != 'uniprot-all'
            "#,
        )
        .bind(self.organization_id)
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch protein entries")?;

        let protein_count = protein_entries.len();
        info!("Found {} proteins to link", protein_count);

        if protein_count == 0 {
            return Ok(());
        }

        // Batch insert dependencies (1000 at a time for performance)
        for chunk in protein_entries.chunks(DEPENDENCY_BATCH_SIZE) {
            let mut query_builder = sqlx::QueryBuilder::new(
                "INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version, dependency_type) "
            );

            query_builder.push_values(chunk, |mut b, (protein_entry_id,)| {
                b.push_bind(aggregate_version_id)
                    .push_bind(protein_entry_id)
                    .push_bind(&self.internal_version)
                    .push_bind("required");
            });

            query_builder.push(" ON CONFLICT DO NOTHING");

            query_builder
                .build()
                .execute(&self.db)
                .await
                .context("Failed to insert dependencies")?;
        }

        info!("✅ Created {} dependencies", protein_count);
        Ok(())
    }

    /// Create bundles after all proteins have been stored
    ///
    /// Creates organism-specific bundles and the swissprot bundle (all proteins)
    pub async fn create_bundles(&self, entries: &[UniProtEntry]) -> Result<()> {
        info!("Creating bundles for {} entries", entries.len());

        // Group proteins by organism (taxonomy_id)
        let mut by_organism: HashMap<i32, (String, Vec<String>)> = HashMap::new();
        let mut all_protein_slugs = Vec::new();

        for entry in entries {
            // Get protein slug (accession)
            let protein_slug = entry.accession.clone();
            all_protein_slugs.push(protein_slug.clone());

            // Group by organism
            by_organism
                .entry(entry.taxonomy_id)
                .or_insert_with(|| (entry.organism_name.clone(), Vec::new()))
                .1
                .push(protein_slug);
        }

        info!(
            "Found {} organisms, {} total proteins",
            by_organism.len(),
            all_protein_slugs.len()
        );

        // Create organism bundles (no minimum threshold)
        for (taxonomy_id, (organism_name, protein_slugs)) in by_organism {
            let slug = taxonomy_to_slug(&organism_name, taxonomy_id);

            self.create_bundle(
                &slug,
                &format!("{} (UniProt Proteins)", organism_name),
                &protein_slugs,
                "bundle",
            )
            .await?;

            info!(
                organism = %organism_name,
                slug = %slug,
                protein_count = protein_slugs.len(),
                "Created organism bundle"
            );
        }

        // Create swissprot bundle (all proteins)
        self.create_bundle(
            "swissprot",
            "UniProt Swiss-Prot (Reviewed Proteins)",
            &all_protein_slugs,
            "bundle",
        )
        .await?;

        info!(
            bundle = "swissprot",
            protein_count = all_protein_slugs.len(),
            "Created swissprot bundle"
        );

        Ok(())
    }

    /// Create a single bundle with dependencies
    ///
    /// Generic method to create any bundle type (organism or database)
    async fn create_bundle(
        &self,
        slug: &str,
        name: &str,
        protein_slugs: &[String],
        source_type: &str,
    ) -> Result<Uuid> {
        debug!(
            slug = %slug,
            protein_count = protein_slugs.len(),
            "Creating bundle"
        );

        // 1. Create registry entry for bundle
        let entry_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            ON CONFLICT (slug) DO UPDATE
            SET name = EXCLUDED.name,
                description = EXCLUDED.description,
                updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(self.organization_id)
        .bind(slug)
        .bind(name)
        .bind(format!("Bundle: {}", name))
        .fetch_one(&self.db)
        .await
        .context("Failed to create bundle registry_entry")?;

        debug!("Created bundle registry entry: {}", entry_id);

        // 2. Create data source with bundle type
        sqlx::query(
            r#"
            INSERT INTO data_sources (id, source_type)
            VALUES ($1, $2)
            ON CONFLICT (id) DO UPDATE SET source_type = EXCLUDED.source_type
            "#,
        )
        .bind(entry_id)
        .bind(source_type)
        .execute(&self.db)
        .await
        .context("Failed to create bundle data_source")?;

        // 3. Create version with semantic versioning
        let version_parts: Vec<&str> = self.internal_version.split('.').collect();
        let version_major = version_parts
            .first()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1);
        let version_minor = version_parts
            .get(1)
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        let version_patch = version_parts
            .get(2)
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);

        let version_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO versions (
                entry_id, version, external_version, dependency_count,
                version_major, version_minor, version_patch
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (entry_id, version) DO UPDATE
            SET external_version = EXCLUDED.external_version,
                dependency_count = EXCLUDED.dependency_count,
                version_major = EXCLUDED.version_major,
                version_minor = EXCLUDED.version_minor,
                version_patch = EXCLUDED.version_patch,
                updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(entry_id)
        .bind(&self.internal_version)
        .bind(&self.external_version)
        .bind(protein_slugs.len() as i32)
        .bind(version_major)
        .bind(version_minor)
        .bind(version_patch)
        .fetch_one(&self.db)
        .await
        .context("Failed to create bundle version")?;

        debug!("Created bundle version: {}", version_id);

        // 4. Create dependencies to all proteins
        self.create_bundle_dependencies(version_id, protein_slugs)
            .await?;

        info!("Created bundle: {}", slug);
        Ok(entry_id)
    }

    /// Create dependencies from bundle to individual proteins by slug
    async fn create_bundle_dependencies(
        &self,
        bundle_version_id: Uuid,
        protein_slugs: &[String],
    ) -> Result<()> {
        debug!(
            bundle_version_id = %bundle_version_id,
            protein_count = protein_slugs.len(),
            "Creating bundle dependencies"
        );

        // Get registry entry IDs for all protein slugs
        if protein_slugs.is_empty() {
            return Ok(());
        }

        // Process in batches to avoid parameter limits
        for chunk in protein_slugs.chunks(DEPENDENCY_BATCH_SIZE) {
            // Build query to get entry IDs for these slugs
            let mut query_builder =
                sqlx::QueryBuilder::new("SELECT id FROM registry_entries WHERE organization_id = ");
            query_builder.push_bind(self.organization_id);
            query_builder.push(" AND slug IN (");

            let mut separated = query_builder.separated(", ");
            for slug in chunk {
                separated.push_bind(slug);
            }
            query_builder.push(")");

            let protein_entry_ids: Vec<(Uuid,)> = query_builder
                .build_query_as()
                .fetch_all(&self.db)
                .await
                .context("Failed to fetch protein entry IDs")?;

            if protein_entry_ids.is_empty() {
                continue;
            }

            // Batch insert dependencies
            let mut dep_query_builder = sqlx::QueryBuilder::new(
                "INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version, dependency_type) "
            );

            dep_query_builder.push_values(&protein_entry_ids, |mut b, (protein_entry_id,)| {
                b.push_bind(bundle_version_id)
                    .push_bind(protein_entry_id)
                    .push_bind(&self.internal_version)
                    .push_bind("required");
            });

            dep_query_builder.push(" ON CONFLICT DO NOTHING");

            dep_query_builder
                .build()
                .execute(&self.db)
                .await
                .context("Failed to insert bundle dependencies")?;
        }

        info!(
            bundle_version_id = %bundle_version_id,
            dependency_count = protein_slugs.len(),
            "Created bundle dependencies"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_source_type_viruses() {
        let lineage = vec!["Viruses".to_string(), "Riboviria".to_string()];
        assert_eq!(classify_source_type(&lineage), "virus");
    }

    #[test]
    fn test_classify_source_type_bacteria() {
        let lineage = vec!["Bacteria".to_string(), "Proteobacteria".to_string()];
        assert_eq!(classify_source_type(&lineage), "bacteria");
    }

    #[test]
    fn test_classify_source_type_archaea() {
        let lineage = vec!["Archaea".to_string(), "Euryarchaeota".to_string()];
        assert_eq!(classify_source_type(&lineage), "archaea");
    }

    #[test]
    fn test_classify_source_type_eukaryota() {
        let lineage = vec!["Eukaryota".to_string(), "Metazoa".to_string()];
        assert_eq!(classify_source_type(&lineage), "organism");
    }

    #[test]
    fn test_classify_source_type_empty() {
        let lineage: Vec<String> = vec![];
        assert_eq!(classify_source_type(&lineage), "organism");
    }

    #[test]
    fn test_classify_source_type_unknown() {
        let lineage = vec!["UnknownDomain".to_string()];
        assert_eq!(classify_source_type(&lineage), "organism");
    }
}
