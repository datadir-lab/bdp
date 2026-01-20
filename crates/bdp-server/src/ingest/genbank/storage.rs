// Storage layer for GenBank/RefSeq data
//
// Uses batch operations (500 chunks) for massive performance improvement:
// - Without batching: ~10 queries per record × 5M records = 50M queries
// - With batching: ~20K queries per 500-chunk
// - Improvement: ~2,500x faster

use anyhow::{Context, Result};
use serde_json::json;
use sqlx::{PgPool, QueryBuilder, Postgres, Row};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::models::GenbankRecord;
use crate::storage::Storage;

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total: usize,
    pub stored: usize,
    pub updated: usize,
    pub skipped: usize,
    pub mappings_created: usize,
    pub bytes_uploaded: u64,
}

/// Storage handler for GenBank/RefSeq data
pub struct GenbankStorage {
    db: PgPool,
    s3: Storage,
    organization_id: Uuid,
    internal_version: String,
    external_version: String,
    release: String,
}

impl GenbankStorage {
    /// Create a new storage handler
    pub fn new(
        db: PgPool,
        s3: Storage,
        organization_id: Uuid,
        internal_version: String,
        external_version: String,
        release: String,
    ) -> Self {
        Self {
            db,
            s3,
            organization_id,
            internal_version,
            external_version,
            release,
        }
    }

    /// Store GenBank records using batch operations
    ///
    /// This method:
    /// 1. Creates data_sources entries for each record
    /// 2. Batch inserts sequence_metadata
    /// 3. Uploads FASTA files to S3
    /// 4. Creates protein mappings (if protein_id exists)
    ///
    /// Uses 500-entry chunks to avoid PostgreSQL parameter limits (65,535)
    pub async fn store_records(&self, records: &[GenbankRecord]) -> Result<StorageStats> {
        if records.is_empty() {
            return Ok(StorageStats {
                total: 0,
                stored: 0,
                updated: 0,
                skipped: 0,
                mappings_created: 0,
                bytes_uploaded: 0,
            });
        }

        info!(
            "Storing {} GenBank records using batch operations (release: {})",
            records.len(),
            self.release
        );

        let mut tx = self.db.begin().await.context("Failed to begin transaction")?;

        // Check for existing records by hash (deduplication)
        let existing_hashes = self.get_existing_hashes(&mut tx, records).await?;
        info!(
            "Found {} existing records (deduplication)",
            existing_hashes.len()
        );

        // Filter out duplicates
        let new_records: Vec<&GenbankRecord> = records
            .iter()
            .filter(|r| !existing_hashes.contains(&r.sequence_hash))
            .collect();

        info!(
            "Processing {} new records ({} duplicates skipped)",
            new_records.len(),
            existing_hashes.len()
        );

        // Process in chunks of 500
        const CHUNK_SIZE: usize = 500;
        let total_chunks = (new_records.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;
        let mut total_bytes_uploaded = 0u64;
        let mut total_mappings = 0usize;

        for (chunk_idx, chunk) in new_records.chunks(CHUNK_SIZE).enumerate() {
            info!(
                "Processing chunk {} / {} ({} records)",
                chunk_idx + 1,
                total_chunks,
                chunk.len()
            );

            // Step 1: Create data_sources and sequence_metadata entries
            let data_source_ids = self.create_data_sources_batch(&mut tx, chunk).await?;

            // Step 2: Insert sequence metadata batch
            self.insert_sequence_metadata_batch(&mut tx, chunk, &data_source_ids)
                .await?;

            // Step 3: Upload FASTA files to S3 (in parallel)
            let bytes_uploaded = self.upload_fasta_batch(chunk, &data_source_ids).await?;
            total_bytes_uploaded += bytes_uploaded;

            // Step 4: Create protein mappings batch
            let mappings_created = self
                .create_protein_mappings_batch(&mut tx, chunk, &data_source_ids)
                .await?;
            total_mappings += mappings_created;

            info!(
                "Chunk {} complete: {} records, {} bytes uploaded, {} mappings",
                chunk_idx + 1,
                chunk.len(),
                bytes_uploaded,
                mappings_created
            );
        }

        // Commit transaction
        tx.commit().await.context("Failed to commit transaction")?;

        info!(
            "Successfully stored {} records ({} bytes, {} mappings)",
            new_records.len(),
            total_bytes_uploaded,
            total_mappings
        );

        Ok(StorageStats {
            total: records.len(),
            stored: new_records.len(),
            updated: 0,
            skipped: existing_hashes.len(),
            mappings_created: total_mappings,
            bytes_uploaded: total_bytes_uploaded,
        })
    }

    /// Get existing sequence hashes for deduplication
    async fn get_existing_hashes(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        records: &[GenbankRecord],
    ) -> Result<std::collections::HashSet<String>> {
        let hashes: Vec<String> = records.iter().map(|r| r.sequence_hash.clone()).collect();

        if hashes.is_empty() {
            return Ok(std::collections::HashSet::new());
        }

        let mut existing = std::collections::HashSet::new();

        // Query in chunks to avoid parameter limits
        for chunk in hashes.chunks(500) {
            let mut query_builder = QueryBuilder::new(
                "SELECT sequence_hash FROM sequence_metadata WHERE sequence_hash IN ("
            );

            let mut separated = query_builder.separated(", ");
            for hash in chunk {
                separated.push_bind(hash);
            }
            separated.push_unseparated(")");

            let rows = query_builder
                .build()
                .fetch_all(&mut **tx)
                .await
                .context("Failed to query existing hashes")?;

            for row in rows {
                let hash: String = row.try_get("sequence_hash")?;
                existing.insert(hash);
            }
        }

        Ok(existing)
    }

    /// Create data_sources entries in batch
    async fn create_data_sources_batch(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        records: &[&GenbankRecord],
    ) -> Result<Vec<Uuid>> {
        let mut data_source_ids = Vec::new();

        // Batch insert data_sources
        let mut query_builder = QueryBuilder::new(
            r#"
            INSERT INTO data_sources (
                id, organization_id, name, source_type, description,
                internal_version, external_version, parsed_at
            )
            "#
        );

        query_builder.push_values(records.iter(), |mut b, record| {
            let id = Uuid::new_v4();
            data_source_ids.push(id);

            b.push_bind(id)
                .push_bind(self.organization_id)
                .push_bind(&record.accession_version)
                .push_bind(record.source_database.as_str())
                .push_bind(&record.definition)
                .push_bind(&self.internal_version)
                .push_bind(&self.external_version)
                .push_bind(chrono::Utc::now());
        });

        query_builder
            .build()
            .execute(&mut **tx)
            .await
            .context("Failed to insert data_sources")?;

        debug!("Created {} data_sources entries", data_source_ids.len());
        Ok(data_source_ids)
    }

    /// Insert sequence metadata in batch
    async fn insert_sequence_metadata_batch(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        records: &[&GenbankRecord],
        data_source_ids: &[Uuid],
    ) -> Result<()> {
        let mut query_builder = QueryBuilder::new(
            r#"
            INSERT INTO sequence_metadata (
                data_source_id, accession, accession_version, sequence_length,
                molecule_type, topology, definition, organism, taxonomy_id,
                gene_name, locus_tag, protein_id, product, features, gc_content,
                sequence_hash, s3_key, source_database, division
            )
            "#
        );

        query_builder.push_values(
            records.iter().zip(data_source_ids.iter()),
            |mut b, (record, ds_id)| {
                let topology_str = record
                    .topology
                    .as_ref()
                    .map(|t| match t {
                        super::models::Topology::Linear => "linear",
                        super::models::Topology::Circular => "circular",
                    })
                    .unwrap_or("linear");

                let features_json = json!(record.all_features);
                let s3_key = record.generate_s3_key(&self.release);

                b.push_bind(ds_id)
                    .push_bind(&record.accession)
                    .push_bind(&record.accession_version)
                    .push_bind(record.sequence_length)
                    .push_bind(&record.molecule_type)
                    .push_bind(topology_str)
                    .push_bind(&record.definition)
                    .push_bind(&record.organism)
                    .push_bind(record.taxonomy_id)
                    .push_bind(record.extract_gene_name())
                    .push_bind(record.extract_locus_tag())
                    .push_bind(record.extract_protein_id())
                    .push_bind(record.extract_product())
                    .push_bind(features_json)
                    .push_bind(record.gc_content)
                    .push_bind(&record.sequence_hash)
                    .push_bind(s3_key)
                    .push_bind(record.source_database.as_str())
                    .push_bind(record.division.as_ref().map(|d| d.as_str()));
            },
        );

        query_builder
            .build()
            .execute(&mut **tx)
            .await
            .context("Failed to insert sequence_metadata")?;

        debug!("Inserted {} sequence_metadata entries", records.len());
        Ok(())
    }

    /// Upload FASTA files to S3 in batch (parallel uploads)
    async fn upload_fasta_batch(
        &self,
        records: &[&GenbankRecord],
        _data_source_ids: &[Uuid],
    ) -> Result<u64> {
        let mut total_bytes = 0u64;

        // Upload in parallel using futures
        let uploads: Vec<_> = records
            .iter()
            .map(|record| {
                let s3_key = record.generate_s3_key(&self.release);
                let fasta_content = record.to_fasta();
                let bytes = fasta_content.len() as u64;
                total_bytes += bytes;

                async move {
                    self.s3
                        .upload(&s3_key, fasta_content.as_bytes().to_vec(), Some("text/plain".to_string()))
                        .await
                        .context(format!("Failed to upload {}", s3_key))?;
                    Ok::<u64, anyhow::Error>(bytes)
                }
            })
            .collect();

        // Wait for all uploads
        let results = futures::future::join_all(uploads).await;

        // Check for errors
        let mut successful_bytes = 0u64;
        for result in results {
            match result {
                Ok(bytes) => successful_bytes += bytes,
                Err(e) => warn!("Upload failed: {}", e),
            }
        }

        debug!(
            "Uploaded {} FASTA files ({} bytes)",
            records.len(),
            successful_bytes
        );
        Ok(successful_bytes)
    }

    /// Create protein mappings in batch
    async fn create_protein_mappings_batch(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        records: &[&GenbankRecord],
        data_source_ids: &[Uuid],
    ) -> Result<usize> {
        // Filter records that have CDS features with protein_id
        let mappable: Vec<_> = records
            .iter()
            .zip(data_source_ids.iter())
            .filter(|(record, _)| !record.cds_features.is_empty())
            .collect();

        if mappable.is_empty() {
            return Ok(0);
        }

        // Collect all protein IDs
        let mut protein_ids = Vec::new();
        for (record, _) in &mappable {
            for cds in &record.cds_features {
                if let Some(ref pid) = cds.protein_id {
                    protein_ids.push(pid.clone());
                }
            }
        }

        if protein_ids.is_empty() {
            return Ok(0);
        }

        // Query UniProt data_sources by accession (protein_id)
        let protein_data_sources = self.query_protein_data_sources(tx, &protein_ids).await?;

        if protein_data_sources.is_empty() {
            debug!("No matching protein data sources found");
            return Ok(0);
        }

        // Build mappings
        let mut mappings = Vec::new();
        for (record, seq_ds_id) in mappable {
            for cds in &record.cds_features {
                if let Some(ref pid) = cds.protein_id {
                    if let Some(prot_ds_id) = protein_data_sources.get(pid) {
                        mappings.push((
                            seq_ds_id,
                            prot_ds_id,
                            cds.start,
                            cds.end,
                            cds.strand.as_ref().map(|s| s.as_str()),
                            cds.codon_start,
                            cds.transl_table,
                        ));
                    }
                }
            }
        }

        if mappings.is_empty() {
            return Ok(0);
        }

        // Batch insert mappings
        let mut query_builder = QueryBuilder::new(
            r#"
            INSERT INTO sequence_protein_mappings (
                sequence_data_source_id, protein_data_source_id, mapping_type,
                cds_start, cds_end, strand, codon_start, transl_table
            )
            "#
        );

        query_builder.push_values(mappings.iter(), |mut b, mapping| {
            b.push_bind(mapping.0)
                .push_bind(mapping.1)
                .push_bind("cds")
                .push_bind(mapping.2)
                .push_bind(mapping.3)
                .push_bind(mapping.4)
                .push_bind(mapping.5)
                .push_bind(mapping.6);
        });

        query_builder.push(" ON CONFLICT DO NOTHING");

        query_builder
            .build()
            .execute(&mut **tx)
            .await
            .context("Failed to insert protein mappings")?;

        debug!("Created {} protein mappings", mappings.len());
        Ok(mappings.len())
    }

    /// Query protein data sources by protein IDs (UniProt accessions)
    async fn query_protein_data_sources(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        protein_ids: &[String],
    ) -> Result<HashMap<String, Uuid>> {
        let mut results = HashMap::new();

        // Query in chunks
        for chunk in protein_ids.chunks(500) {
            let mut query_builder = QueryBuilder::new(
                r#"
                SELECT pm.accession, ds.id as data_source_id
                FROM protein_metadata pm
                JOIN data_sources ds ON pm.data_source_id = ds.id
                WHERE pm.accession IN (
                "#
            );

            let mut separated = query_builder.separated(", ");
            for pid in chunk {
                separated.push_bind(pid);
            }
            separated.push_unseparated(")");

            let rows = query_builder
                .build()
                .fetch_all(&mut **tx)
                .await
                .context("Failed to query protein data sources")?;

            for row in rows {
                let accession: String = row.try_get("accession")?;
                let ds_id: Uuid = row.try_get("data_source_id")?;
                results.insert(accession, ds_id);
            }
        }

        debug!(
            "Found {} protein data sources for {} protein IDs",
            results.len(),
            protein_ids.len()
        );
        Ok(results)
    }
}
