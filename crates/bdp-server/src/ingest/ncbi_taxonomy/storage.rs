//! Storage layer for NCBI Taxonomy data
//!
//! Creates individual data sources for each taxonomy with proper schema structure.

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::collections::HashMap;
use tracing::{debug, info};
use uuid::Uuid;

use super::models::{DeletedTaxon, MergedTaxon, TaxdumpData, TaxonomyEntry};
use crate::ingest::citations::{ncbi_taxonomy_policy, setup_citation_policy};
use crate::storage::Storage;

/// Storage handler for NCBI Taxonomy data
pub struct NcbiTaxonomyStorage {
    db: PgPool,
    s3: Option<Storage>,
    organization_id: Uuid,
    internal_version: String,
    external_version: String,
    chunk_size: usize,
    transaction_batch_size: Option<usize>,
}

impl NcbiTaxonomyStorage {
    /// Default chunk size for batch operations
    /// 500 entries provides good balance between performance and parameter limits
    /// (500 entries × ~10 parameters × 6 queries = ~30,000 parameters, well under PostgreSQL's 65,535 limit)
    pub const DEFAULT_CHUNK_SIZE: usize = 500;

    /// Create a new storage handler with default chunk size
    pub fn new(
        db: PgPool,
        organization_id: Uuid,
        internal_version: String,
        external_version: String,
    ) -> Self {
        Self::with_chunk_size(
            db,
            organization_id,
            internal_version,
            external_version,
            Self::DEFAULT_CHUNK_SIZE,
        )
    }

    /// Create a new storage handler with custom chunk size
    pub fn with_chunk_size(
        db: PgPool,
        organization_id: Uuid,
        internal_version: String,
        external_version: String,
        chunk_size: usize,
    ) -> Self {
        Self {
            db,
            s3: None,
            organization_id,
            internal_version,
            external_version,
            chunk_size,
            transaction_batch_size: None,
        }
    }

    /// Create a new storage handler with custom chunk size and transaction batch size
    ///
    /// # Arguments
    /// * `transaction_batch_size` - Number of chunks to process before committing transaction
    ///   - `None` (default): Single transaction for entire ingestion
    ///   - `Some(10)`: Commit every 10 chunks (recommended for extreme scale)
    ///   - `Some(20)`: Commit every 20 chunks (good balance)
    ///
    /// Use transaction batching for very large ingestions (>10M entries) to prevent:
    /// - Lock contention on long-running transactions
    /// - Transaction timeout issues
    /// - Excessive memory usage
    pub fn with_transaction_batching(
        db: PgPool,
        organization_id: Uuid,
        internal_version: String,
        external_version: String,
        chunk_size: usize,
        transaction_batch_size: usize,
    ) -> Self {
        Self {
            db,
            s3: None,
            organization_id,
            internal_version,
            external_version,
            chunk_size,
            transaction_batch_size: Some(transaction_batch_size),
        }
    }

    /// Create storage handler with S3 support and default chunk size
    pub fn with_s3(
        db: PgPool,
        s3: Storage,
        organization_id: Uuid,
        internal_version: String,
        external_version: String,
    ) -> Self {
        Self::with_s3_and_chunk_size(
            db,
            s3,
            organization_id,
            internal_version,
            external_version,
            Self::DEFAULT_CHUNK_SIZE,
        )
    }

    /// Create storage handler with S3 support and custom chunk size
    pub fn with_s3_and_chunk_size(
        db: PgPool,
        s3: Storage,
        organization_id: Uuid,
        internal_version: String,
        external_version: String,
        chunk_size: usize,
    ) -> Self {
        Self {
            db,
            s3: Some(s3),
            organization_id,
            internal_version,
            external_version,
            chunk_size,
            transaction_batch_size: None,
        }
    }

    /// Create storage handler with S3, custom chunk size, and transaction batching
    pub fn with_s3_and_transaction_batching(
        db: PgPool,
        s3: Storage,
        organization_id: Uuid,
        internal_version: String,
        external_version: String,
        chunk_size: usize,
        transaction_batch_size: usize,
    ) -> Self {
        Self {
            db,
            s3: Some(s3),
            organization_id,
            internal_version,
            external_version,
            chunk_size,
            transaction_batch_size: Some(transaction_batch_size),
        }
    }

    /// Set up citation policy for NCBI Taxonomy organization (idempotent)
    ///
    /// This should be called once during pipeline initialization to ensure
    /// citation policy is properly configured for the organization.
    pub async fn setup_citations(&self) -> Result<()> {
        let policy_config = ncbi_taxonomy_policy(self.organization_id, None);
        setup_citation_policy(&self.db, &policy_config).await?;
        info!("NCBI Taxonomy citation policy configured");
        Ok(())
    }

    /// Store taxdump data to database and S3 using batch operations
    ///
    /// Creates individual data sources for each taxonomy entry with versions
    ///
    /// Uses batch inserts to avoid N+1 query patterns for massive performance improvement:
    /// - Old: ~6 queries per entry × 2.5M entries = 15M queries
    /// - New: ~10 queries per 500-entry chunk = ~50K queries
    /// - Improvement: ~300-500x faster
    pub async fn store(&self, taxdump: &TaxdumpData) -> Result<StorageStats> {
        info!(
            "Storing {} taxonomy entries using batch operations (external: {}, internal: {})",
            taxdump.entries.len(),
            self.external_version,
            self.internal_version
        );

        let mut tx = self
            .db
            .begin()
            .await
            .context("Failed to begin transaction")?;

        // Check which entries already exist
        let existing_taxonomy_ids = self
            .get_existing_taxonomy_ids(&mut tx, &taxdump.entries)
            .await?;

        let new_count = taxdump.entries.len() - existing_taxonomy_ids.len();
        let update_count = existing_taxonomy_ids.len();

        info!(
            "Found {} existing entries to update, {} new entries to create",
            update_count, new_count
        );

        // Process entries in configurable chunks (PostgreSQL parameter limit consideration)
        let total_chunks = (taxdump.entries.len() + self.chunk_size - 1) / self.chunk_size;

        if let Some(tx_batch_size) = self.transaction_batch_size {
            info!("Using transaction batching: committing every {} chunks", tx_batch_size);
        } else {
            info!("Using single transaction for all {} chunks", total_chunks);
        }

        for (chunk_idx, chunk) in taxdump.entries.chunks(self.chunk_size).enumerate() {
            info!(
                "Processing chunk {} / {} ({} entries)",
                chunk_idx + 1,
                total_chunks,
                chunk.len()
            );

            // Batch insert/update for this chunk
            self.store_chunk_batch(&mut tx, chunk, &existing_taxonomy_ids)
                .await?;

            // Commit transaction periodically if batching is enabled
            if let Some(tx_batch_size) = self.transaction_batch_size {
                if (chunk_idx + 1) % tx_batch_size == 0 && chunk_idx + 1 < total_chunks {
                    debug!(
                        "Committing transaction batch at chunk {} / {}",
                        chunk_idx + 1,
                        total_chunks
                    );
                    tx.commit()
                        .await
                        .context("Failed to commit transaction batch")?;
                    tx = self
                        .db
                        .begin()
                        .await
                        .context("Failed to begin new transaction")?;
                }
            }
        }

        // Handle merged and deleted taxa in batches
        if !taxdump.merged.is_empty() {
            info!("Handling {} merged taxa", taxdump.merged.len());
            self.handle_merged_taxa_batch(&mut tx, &taxdump.merged)
                .await?;
        }

        if !taxdump.deleted.is_empty() {
            info!("Handling {} deleted taxa", taxdump.deleted.len());
            self.handle_deleted_taxa_batch(&mut tx, &taxdump.deleted)
                .await?;
        }

        // Commit final transaction
        tx.commit().await.context("Failed to commit transaction")?;

        info!(
            "Successfully stored/updated {} entries ({} new, {} updated)",
            taxdump.entries.len(),
            new_count,
            update_count
        );

        Ok(StorageStats {
            total: taxdump.entries.len(),
            stored: new_count,
            updated: update_count,
            failed: 0,
        })
    }

    /// Get set of taxonomy IDs that already exist in database
    async fn get_existing_taxonomy_ids(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entries: &[TaxonomyEntry],
    ) -> Result<HashMap<i32, Uuid>> {
        let taxonomy_ids: Vec<i32> = entries.iter().map(|e| e.taxonomy_id).collect();

        if taxonomy_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // Query in chunks to avoid parameter limits
        let mut existing = HashMap::new();
        for chunk in taxonomy_ids.chunks(self.chunk_size) {
            let mut query_builder = QueryBuilder::new(
                "SELECT taxonomy_id, data_source_id FROM taxonomy_metadata WHERE taxonomy_id IN (",
            );

            let mut separated = query_builder.separated(", ");
            for &tax_id in chunk {
                separated.push_bind(tax_id);
            }
            separated.push_unseparated(")");

            let rows = query_builder
                .build_query_as::<(i32, Uuid)>()
                .fetch_all(&mut **tx)
                .await?;

            for (tax_id, data_source_id) in rows {
                existing.insert(tax_id, data_source_id);
            }
        }

        Ok(existing)
    }

    /// Store a chunk of entries using batch operations
    async fn store_chunk_batch(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        chunk: &[TaxonomyEntry],
        _existing_taxonomy_ids: &HashMap<i32, Uuid>,
    ) -> Result<()> {
        // 1. Batch insert/update registry_entries
        let entry_id_map = self.batch_upsert_registry_entries(tx, chunk).await?;

        // 2. Batch insert data_sources
        self.batch_insert_data_sources(tx, &entry_id_map).await?;

        // 3. Batch insert/update taxonomy_metadata
        self.batch_upsert_taxonomy_metadata(tx, chunk, &entry_id_map)
            .await?;

        // 4. Batch insert versions
        let version_id_map = self.batch_insert_versions(tx, &entry_id_map).await?;

        // 5. Batch insert version_files (with S3 uploads)
        self.batch_insert_version_files(tx, chunk, &entry_id_map, &version_id_map)
            .await?;

        Ok(())
    }

    /// Batch upsert registry entries
    async fn batch_upsert_registry_entries(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        chunk: &[TaxonomyEntry],
    ) -> Result<HashMap<i32, Uuid>> {
        let mut entry_id_map = HashMap::new();

        // Build batch upsert query
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO registry_entries (id, organization_id, slug, name, description, entry_type) "
        );

        query_builder.push_values(chunk, |mut b, entry| {
            let id = Uuid::new_v4();
            entry_id_map.insert(entry.taxonomy_id, id);

            let slug = format!("{}", entry.taxonomy_id);
            let description = format!("NCBI Taxonomy: {} ({})", entry.scientific_name, entry.rank);

            b.push_bind(id)
                .push_bind(self.organization_id)
                .push_bind(slug)
                .push_bind(&entry.scientific_name)
                .push_bind(description)
                .push_bind("data_source");
        });

        query_builder.push(
            " ON CONFLICT (slug) DO UPDATE SET \
             name = EXCLUDED.name, \
             description = EXCLUDED.description, \
             updated_at = NOW() \
             RETURNING id, slug",
        );

        let rows = query_builder
            .build_query_as::<(Uuid, String)>()
            .fetch_all(&mut **tx)
            .await?;

        // Update map with returned IDs (handles conflicts)
        let mut result_map = HashMap::new();
        for (id, slug) in rows {
            if let Ok(tax_id) = slug.parse::<i32>() {
                result_map.insert(tax_id, id);
            }
        }

        Ok(result_map)
    }

    /// Batch insert data sources
    async fn batch_insert_data_sources(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry_id_map: &HashMap<i32, Uuid>,
    ) -> Result<()> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("INSERT INTO data_sources (id, source_type) ");

        query_builder.push_values(entry_id_map.values(), |mut b, entry_id| {
            b.push_bind(entry_id).push_bind("taxonomy");
        });

        query_builder.push(" ON CONFLICT (id) DO NOTHING");

        query_builder.build().execute(&mut **tx).await?;

        Ok(())
    }

    /// Batch upsert taxonomy metadata
    async fn batch_upsert_taxonomy_metadata(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        chunk: &[TaxonomyEntry],
        entry_id_map: &HashMap<i32, Uuid>,
    ) -> Result<()> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO taxonomy_metadata \
             (data_source_id, taxonomy_id, scientific_name, common_name, rank, lineage, ncbi_tax_version) "
        );

        query_builder.push_values(chunk, |mut b, entry| {
            // Entry ID is guaranteed to exist since we just inserted it in batch_upsert_registry_entries
            let entry_id = entry_id_map.get(&entry.taxonomy_id)
                .unwrap_or_else(|| panic!(
                    "Entry ID must exist in map for taxonomy_id {} - was just inserted in batch_upsert_registry_entries",
                    entry.taxonomy_id
                ));
            b.push_bind(entry_id)
                .push_bind(entry.taxonomy_id)
                .push_bind(&entry.scientific_name)
                .push_bind(&entry.common_name)
                .push_bind(&entry.rank)
                .push_bind(&entry.lineage)
                .push_bind(&self.external_version);
        });

        query_builder.push(
            " ON CONFLICT (data_source_id) DO UPDATE SET \
             scientific_name = EXCLUDED.scientific_name, \
             common_name = EXCLUDED.common_name, \
             rank = EXCLUDED.rank, \
             lineage = EXCLUDED.lineage, \
             ncbi_tax_version = EXCLUDED.ncbi_tax_version, \
             parsed_at = NOW()",
        );

        query_builder.build().execute(&mut **tx).await?;

        Ok(())
    }

    /// Batch insert versions
    async fn batch_insert_versions(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entry_id_map: &HashMap<i32, Uuid>,
    ) -> Result<HashMap<i32, Uuid>> {
        let mut version_id_map = HashMap::new();

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO versions (id, registry_entry_id, version_string, status) ",
        );

        query_builder.push_values(entry_id_map, |mut b, (tax_id, entry_id)| {
            let version_id = Uuid::new_v4();
            version_id_map.insert(*tax_id, version_id);

            b.push_bind(version_id)
                .push_bind(entry_id)
                .push_bind(&self.internal_version)
                .push_bind("published");
        });

        query_builder.push(" ON CONFLICT (registry_entry_id, version_string) DO NOTHING");

        query_builder.build().execute(&mut **tx).await?;

        // Fetch actual version IDs (handles conflicts)
        let mut result_map = HashMap::new();
        for (tax_id, entry_id) in entry_id_map {
            let version_id = sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM versions WHERE registry_entry_id = $1 AND version_string = $2",
            )
            .bind(entry_id)
            .bind(&self.internal_version)
            .fetch_one(&mut **tx)
            .await?;

            result_map.insert(*tax_id, version_id);
        }

        Ok(result_map)
    }

    /// Batch insert version files (with S3 uploads)
    async fn batch_insert_version_files(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        chunk: &[TaxonomyEntry],
        _entry_id_map: &HashMap<i32, Uuid>,
        version_id_map: &HashMap<i32, Uuid>,
    ) -> Result<()> {
        // Generate all file contents and upload to S3 if configured
        let mut file_data = Vec::new();

        for entry in chunk {
            // Version ID is guaranteed to exist since we just inserted it in batch_insert_versions
            let version_id = version_id_map.get(&entry.taxonomy_id)
                .unwrap_or_else(|| panic!(
                    "Version ID must exist in map for taxonomy_id {} - was just inserted in batch_insert_versions",
                    entry.taxonomy_id
                ));

            // Generate JSON content
            let json_content = entry.to_json()?;
            let json_size = json_content.len() as i64;
            let json_checksum = format!("{:x}", md5::compute(&json_content));

            // Generate TSV content
            let tsv_content = format!("{}\n{}", TaxonomyEntry::tsv_header(), entry.to_tsv());
            let tsv_size = tsv_content.len() as i64;
            let tsv_checksum = format!("{:x}", md5::compute(&tsv_content));

            // S3 keys
            let s3_key_json =
                format!("ncbi/{}/{}/taxonomy.json", entry.taxonomy_id, self.internal_version);
            let s3_key_tsv =
                format!("ncbi/{}/{}/taxonomy.tsv", entry.taxonomy_id, self.internal_version);

            // Upload to S3 if configured
            if let Some(s3) = &self.s3 {
                s3.upload(
                    &s3_key_json,
                    json_content.as_bytes().to_vec(),
                    Some("application/json".to_string()),
                )
                .await
                .context("Failed to upload JSON to S3")?;

                s3.upload(
                    &s3_key_tsv,
                    tsv_content.as_bytes().to_vec(),
                    Some("text/tab-separated-values".to_string()),
                )
                .await
                .context("Failed to upload TSV to S3")?;
            }

            file_data.push((*version_id, "json", s3_key_json, json_checksum, json_size));
            file_data.push((*version_id, "tsv", s3_key_tsv, tsv_checksum, tsv_size));
        }

        // Batch insert version_files
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes) ",
        );

        query_builder.push_values(
            &file_data,
            |mut b, (version_id, format, s3_key, checksum, size)| {
                b.push_bind(version_id)
                    .push_bind(format)
                    .push_bind(s3_key)
                    .push_bind(checksum)
                    .push_bind(size);
            },
        );

        query_builder.push(
            " ON CONFLICT (version_id, format) DO UPDATE SET \
             s3_key = EXCLUDED.s3_key, \
             checksum = EXCLUDED.checksum, \
             size_bytes = EXCLUDED.size_bytes",
        );

        query_builder.build().execute(&mut **tx).await?;

        Ok(())
    }

    /// Handle merged taxa in batch
    async fn handle_merged_taxa_batch(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        merged: &[MergedTaxon],
    ) -> Result<()> {
        // Get all old taxonomy IDs that exist
        let old_tax_ids: Vec<i32> = merged.iter().map(|m| m.old_taxonomy_id).collect();

        let mut existing_map = HashMap::new();
        for chunk in old_tax_ids.chunks(self.chunk_size) {
            let mut query_builder = QueryBuilder::new(
                "SELECT taxonomy_id, data_source_id FROM taxonomy_metadata WHERE taxonomy_id IN (",
            );

            let mut separated = query_builder.separated(", ");
            for &tax_id in chunk {
                separated.push_bind(tax_id);
            }
            separated.push_unseparated(")");

            let rows = query_builder
                .build_query_as::<(i32, Uuid)>()
                .fetch_all(&mut **tx)
                .await?;

            for (tax_id, data_source_id) in rows {
                existing_map.insert(tax_id, data_source_id);
            }
        }

        // Batch update merged taxa
        for merged_taxon in merged {
            if let Some(old_entry_id) = existing_map.get(&merged_taxon.old_taxonomy_id) {
                let lineage_note = format!(
                    "[MERGED INTO {}] Previous lineage recorded here",
                    merged_taxon.new_taxonomy_id
                );

                sqlx::query(
                    "UPDATE taxonomy_metadata SET lineage = $1, parsed_at = NOW() WHERE data_source_id = $2"
                )
                .bind(&lineage_note)
                .bind(old_entry_id)
                .execute(&mut **tx)
                .await?;

                debug!(
                    old_id = merged_taxon.old_taxonomy_id,
                    new_id = merged_taxon.new_taxonomy_id,
                    "Marked taxonomy as merged"
                );
            }
        }

        Ok(())
    }

    /// Handle deleted taxa in batch
    async fn handle_deleted_taxa_batch(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        deleted: &[DeletedTaxon],
    ) -> Result<()> {
        // Get all deleted taxonomy IDs that exist
        let del_tax_ids: Vec<i32> = deleted.iter().map(|d| d.taxonomy_id).collect();

        let mut existing_map = HashMap::new();
        for chunk in del_tax_ids.chunks(self.chunk_size) {
            let mut query_builder = QueryBuilder::new(
                "SELECT taxonomy_id, data_source_id FROM taxonomy_metadata WHERE taxonomy_id IN (",
            );

            let mut separated = query_builder.separated(", ");
            for &tax_id in chunk {
                separated.push_bind(tax_id);
            }
            separated.push_unseparated(")");

            let rows = query_builder
                .build_query_as::<(i32, Uuid)>()
                .fetch_all(&mut **tx)
                .await?;

            for (tax_id, data_source_id) in rows {
                existing_map.insert(tax_id, data_source_id);
            }
        }

        // Batch update deleted taxa
        for deleted_taxon in deleted {
            if let Some(entry_id) = existing_map.get(&deleted_taxon.taxonomy_id) {
                let lineage_note = "[DELETED FROM NCBI] This taxonomy ID is no longer valid";

                sqlx::query(
                    "UPDATE taxonomy_metadata SET lineage = $1, parsed_at = NOW() WHERE data_source_id = $2"
                )
                .bind(lineage_note)
                .bind(entry_id)
                .execute(&mut **tx)
                .await?;

                debug!(taxonomy_id = deleted_taxon.taxonomy_id, "Marked taxonomy as deleted");
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
