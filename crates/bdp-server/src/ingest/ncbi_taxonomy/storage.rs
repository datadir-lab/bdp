//! Storage layer for NCBI Taxonomy data
//!
//! Creates individual data sources for each taxonomy with proper schema structure.

use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::{debug, error, info};
use uuid::Uuid;

use super::models::{DeletedTaxon, MergedTaxon, TaxdumpData, TaxonomyEntry};
use crate::storage::Storage;

/// Storage handler for NCBI Taxonomy data
pub struct NcbiTaxonomyStorage {
    db: PgPool,
    s3: Option<Storage>,
    organization_id: Uuid,
    internal_version: String,
    external_version: String,
}

impl NcbiTaxonomyStorage {
    /// Create a new storage handler
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

    /// Create storage handler with S3 support
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

    /// Store taxdump data to database and S3
    ///
    /// Creates individual data sources for each taxonomy entry with versions
    pub async fn store(&self, taxdump: &TaxdumpData) -> Result<StorageStats> {
        info!(
            "Storing {} taxonomy entries (external: {}, internal: {})",
            taxdump.entries.len(),
            self.external_version,
            self.internal_version
        );

        let mut tx = self.db.begin().await.context("Failed to begin transaction")?;

        let mut stored_count = 0;
        let mut updated_count = 0;
        let mut error_count = 0;
        let mut errors = Vec::new();

        // Store each taxonomy entry as a separate data source
        for entry in &taxdump.entries {
            // Create a savepoint before each entry to isolate failures
            if let Err(e) = sqlx::query("SAVEPOINT entry_savepoint")
                .execute(&mut *tx)
                .await
            {
                error!(
                    error = %e,
                    "Failed to create savepoint, aborting batch"
                );
                break;
            }

            match self.store_entry_tx(&mut tx, entry).await {
                Ok(is_new) => {
                    // Release savepoint on success
                    if let Err(e) = sqlx::query("RELEASE SAVEPOINT entry_savepoint")
                        .execute(&mut *tx)
                        .await
                    {
                        error!(
                            taxonomy_id = %entry.taxonomy_id,
                            error = %e,
                            "Failed to release savepoint"
                        );
                        error_count += 1;
                        continue;
                    }

                    if is_new {
                        stored_count += 1;
                    } else {
                        updated_count += 1;
                    }
                }
                Err(e) => {
                    // Rollback to savepoint on failure
                    if let Err(rollback_err) = sqlx::query("ROLLBACK TO SAVEPOINT entry_savepoint")
                        .execute(&mut *tx)
                        .await
                    {
                        error!(
                            taxonomy_id = %entry.taxonomy_id,
                            rollback_error = %rollback_err,
                            "Failed to rollback savepoint, aborting batch"
                        );
                        break;
                    }

                    error_count += 1;
                    error!(
                        taxonomy_id = %entry.taxonomy_id,
                        error = %e,
                        error_chain = ?e.chain().collect::<Vec<_>>(),
                        "Failed to store entry (isolated with savepoint)"
                    );

                    // Collect first 5 errors for debugging
                    if errors.len() < 5 {
                        errors.push((entry.taxonomy_id, format!("{:#}", e)));
                    }
                }
            }
        }

        // Handle merged and deleted taxa (mark as deprecated)
        if !taxdump.merged.is_empty() {
            info!("Handling {} merged taxa", taxdump.merged.len());
            self.handle_merged_taxa(&mut tx, &taxdump.merged).await?;
        }

        if !taxdump.deleted.is_empty() {
            info!("Handling {} deleted taxa", taxdump.deleted.len());
            self.handle_deleted_taxa(&mut tx, &taxdump.deleted).await?;
        }

        // Commit transaction
        tx.commit().await.context("Failed to commit transaction")?;

        // Log error summary
        if error_count > 0 {
            error!(
                stored = stored_count,
                updated = updated_count,
                failed = error_count,
                total = taxdump.entries.len(),
                "Storage completed with errors"
            );

            for (taxonomy_id, err) in &errors {
                error!(
                    taxonomy_id = %taxonomy_id,
                    error = %err,
                    "Sample error"
                );
            }

            if error_count > 5 {
                error!(
                    additional_errors = error_count - 5,
                    "Additional errors not shown"
                );
            }
        }

        info!(
            "Successfully stored/updated {}/{} entries ({} new, {} updated)",
            stored_count + updated_count,
            taxdump.entries.len(),
            stored_count,
            updated_count
        );

        Ok(StorageStats {
            total: taxdump.entries.len(),
            stored: stored_count,
            updated: updated_count,
            failed: error_count,
        })
    }

    /// Store a single taxonomy entry within a transaction
    ///
    /// Creates: registry_entry -> data_source -> taxonomy_metadata -> version -> version_files
    ///
    /// Returns: true if new entry was created, false if existing entry was updated
    async fn store_entry_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry: &TaxonomyEntry,
    ) -> Result<bool> {
        debug!("Storing taxonomy: {}", entry.taxonomy_id);

        // Check if taxonomy already exists
        let existing = sqlx::query_scalar::<_, Uuid>(
            "SELECT data_source_id FROM taxonomy_metadata WHERE taxonomy_id = $1"
        )
        .bind(entry.taxonomy_id)
        .fetch_optional(&mut **tx)
        .await?;

        let is_new = existing.is_none();

        // 1. Create or update registry entry
        let entry_id = self.create_registry_entry_tx(tx, entry).await?;

        // 2. Create data source
        self.create_data_source_tx(tx, entry_id).await?;

        // 3. Create or update taxonomy metadata
        self.create_taxonomy_metadata_tx(tx, entry_id, entry).await?;

        // 4. Create version (only if new or version changed)
        let version_id = self.create_version_tx(tx, entry_id).await?;

        // 5. Create version files for JSON and TSV formats
        self.create_version_files_tx(tx, entry, version_id).await?;

        debug!("Successfully stored taxonomy: {}", entry.taxonomy_id);
        Ok(is_new)
    }

    /// Create registry entry for the taxonomy (within transaction)
    async fn create_registry_entry_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry: &TaxonomyEntry,
    ) -> Result<Uuid> {
        let slug = format!("{}", entry.taxonomy_id);
        let name = &entry.scientific_name;
        let description = format!(
            "NCBI Taxonomy: {} ({})",
            entry.scientific_name, entry.rank
        );

        let entry_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            ON CONFLICT (slug) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                updated_at = NOW()
            RETURNING id
            "#
        )
        .bind(self.organization_id)
        .bind(&slug)
        .bind(name)
        .bind(&description)
        .fetch_one(&mut **tx)
        .await
        .context("Failed to create registry_entry")?;

        Ok(entry_id)
    }

    /// Create data source (within transaction)
    async fn create_data_source_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry_id: Uuid,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO data_sources (id, source_type)
            VALUES ($1, 'taxonomy')
            ON CONFLICT (id) DO NOTHING
            "#
        )
        .bind(entry_id)
        .execute(&mut **tx)
        .await
        .context("Failed to create data_source")?;

        Ok(())
    }

    /// Create or update taxonomy metadata (within transaction)
    async fn create_taxonomy_metadata_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry_id: Uuid,
        entry: &TaxonomyEntry,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO taxonomy_metadata (
                data_source_id,
                taxonomy_id,
                scientific_name,
                common_name,
                rank,
                lineage,
                ncbi_tax_version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (data_source_id) DO UPDATE SET
                scientific_name = EXCLUDED.scientific_name,
                common_name = EXCLUDED.common_name,
                rank = EXCLUDED.rank,
                lineage = EXCLUDED.lineage,
                ncbi_tax_version = EXCLUDED.ncbi_tax_version,
                parsed_at = NOW()
            "#
        )
        .bind(entry_id)
        .bind(entry.taxonomy_id)
        .bind(&entry.scientific_name)
        .bind(&entry.common_name)
        .bind(&entry.rank)
        .bind(&entry.lineage)
        .bind(&self.external_version)
        .execute(&mut **tx)
        .await
        .context("Failed to create taxonomy_metadata")?;

        Ok(())
    }

    /// Create version (within transaction)
    async fn create_version_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry_id: Uuid,
    ) -> Result<Uuid> {
        let version_id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO versions (id, registry_entry_id, version_string, status)
            VALUES ($1, $2, $3, 'published')
            ON CONFLICT (registry_entry_id, version_string) DO NOTHING
            "#
        )
        .bind(version_id)
        .bind(entry_id)
        .bind(&self.internal_version)
        .execute(&mut **tx)
        .await
        .context("Failed to create version")?;

        // Get the actual version_id (in case of conflict)
        let actual_version_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT id FROM versions
            WHERE registry_entry_id = $1 AND version_string = $2
            "#
        )
        .bind(entry_id)
        .bind(&self.internal_version)
        .fetch_one(&mut **tx)
        .await
        .context("Failed to fetch version_id")?;

        Ok(actual_version_id)
    }

    /// Create version files for JSON and TSV formats (within transaction)
    async fn create_version_files_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry: &TaxonomyEntry,
        version_id: Uuid,
    ) -> Result<()> {
        // Generate JSON content
        let json_content = entry.to_json()?;
        let json_size = json_content.len() as i64;
        let json_checksum = format!("{:x}", md5::compute(&json_content));

        // Generate TSV content
        let tsv_content = format!("{}\n{}", TaxonomyEntry::tsv_header(), entry.to_tsv());
        let tsv_size = tsv_content.len() as i64;
        let tsv_checksum = format!("{:x}", md5::compute(&tsv_content));

        // S3 keys (if S3 is configured)
        let s3_key_json = format!(
            "ncbi/{}/{}/taxonomy.json",
            entry.taxonomy_id, self.internal_version
        );
        let s3_key_tsv = format!(
            "ncbi/{}/{}/taxonomy.tsv",
            entry.taxonomy_id, self.internal_version
        );

        // Upload to S3 if configured
        if let Some(s3) = &self.s3 {
            debug!(
                taxonomy_id = entry.taxonomy_id,
                "Uploading JSON and TSV to S3"
            );

            s3.upload(&s3_key_json, json_content.as_bytes().to_vec(), Some("application/json".to_string()))
                .await
                .context("Failed to upload JSON to S3")?;

            s3.upload(&s3_key_tsv, tsv_content.as_bytes().to_vec(), Some("text/tab-separated-values".to_string()))
                .await
                .context("Failed to upload TSV to S3")?;

            debug!(
                taxonomy_id = entry.taxonomy_id,
                json_key = %s3_key_json,
                tsv_key = %s3_key_tsv,
                "Successfully uploaded files to S3"
            );
        }

        // Create version_files records for JSON
        sqlx::query(
            r#"
            INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (version_id, format) DO UPDATE SET
                s3_key = EXCLUDED.s3_key,
                checksum = EXCLUDED.checksum,
                size_bytes = EXCLUDED.size_bytes
            "#
        )
        .bind(version_id)
        .bind("json")
        .bind(&s3_key_json)
        .bind(&json_checksum)
        .bind(json_size)
        .execute(&mut **tx)
        .await
        .context("Failed to create version_file for JSON")?;

        // Create version_files records for TSV
        sqlx::query(
            r#"
            INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (version_id, format) DO UPDATE SET
                s3_key = EXCLUDED.s3_key,
                checksum = EXCLUDED.checksum,
                size_bytes = EXCLUDED.size_bytes
            "#
        )
        .bind(version_id)
        .bind("tsv")
        .bind(&s3_key_tsv)
        .bind(&tsv_checksum)
        .bind(tsv_size)
        .execute(&mut **tx)
        .await
        .context("Failed to create version_file for TSV")?;

        Ok(())
    }

    /// Handle merged taxa by marking old taxonomy IDs as deprecated
    ///
    /// For each merged taxon, we add a note to the taxonomy_metadata indicating
    /// that it has been merged into a new taxonomy ID
    async fn handle_merged_taxa(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        merged: &[MergedTaxon],
    ) -> Result<()> {
        for merged_taxon in merged {
            // Check if old taxonomy ID exists in our database
            let old_entry = sqlx::query_scalar::<_, Option<Uuid>>(
                "SELECT data_source_id FROM taxonomy_metadata WHERE taxonomy_id = $1"
            )
            .bind(merged_taxon.old_taxonomy_id)
            .fetch_optional(&mut **tx)
            .await?;

            if let Some(old_entry_id) = old_entry {
                // Update the taxonomy_metadata with a note about the merge
                let lineage_note = format!(
                    "[MERGED INTO {}] Previous lineage recorded here",
                    merged_taxon.new_taxonomy_id
                );

                sqlx::query(
                    r#"
                    UPDATE taxonomy_metadata
                    SET lineage = $1,
                        parsed_at = NOW()
                    WHERE data_source_id = $2
                    "#
                )
                .bind(&lineage_note)
                .bind(old_entry_id)
                .execute(&mut **tx)
                .await
                .context("Failed to mark merged taxon")?;

                debug!(
                    old_id = merged_taxon.old_taxonomy_id,
                    new_id = merged_taxon.new_taxonomy_id,
                    "Marked taxonomy as merged"
                );
            }
        }

        Ok(())
    }

    /// Handle deleted taxa by marking them as deleted
    ///
    /// For each deleted taxon, we update the taxonomy_metadata to indicate
    /// that this taxonomy ID has been deleted from NCBI
    async fn handle_deleted_taxa(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        deleted: &[DeletedTaxon],
    ) -> Result<()> {
        for deleted_taxon in deleted {
            // Check if deleted taxonomy ID exists in our database
            let entry = sqlx::query_scalar::<_, Option<Uuid>>(
                "SELECT data_source_id FROM taxonomy_metadata WHERE taxonomy_id = $1"
            )
            .bind(deleted_taxon.taxonomy_id)
            .fetch_optional(&mut **tx)
            .await?;

            if let Some(entry_id) = entry {
                // Update the taxonomy_metadata with a note about deletion
                let lineage_note = "[DELETED FROM NCBI] This taxonomy ID is no longer valid";

                sqlx::query(
                    r#"
                    UPDATE taxonomy_metadata
                    SET lineage = $1,
                        parsed_at = NOW()
                    WHERE data_source_id = $2
                    "#
                )
                .bind(lineage_note)
                .bind(entry_id)
                .execute(&mut **tx)
                .await
                .context("Failed to mark deleted taxon")?;

                debug!(
                    taxonomy_id = deleted_taxon.taxonomy_id,
                    "Marked taxonomy as deleted"
                );
            }
        }

        Ok(())
    }
}

/// Statistics from storage operation
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Total entries attempted
    pub total: usize,
    /// New entries created
    pub stored: usize,
    /// Existing entries updated
    pub updated: usize,
    /// Failed entries
    pub failed: usize,
}
