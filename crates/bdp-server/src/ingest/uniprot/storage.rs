//! Storage layer for UniProt parsed data
//!
//! Creates individual data sources for each protein with proper schema structure.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tracing::{debug, info};
use uuid::Uuid;

use super::models::UniProtEntry;
use crate::storage::Storage;

/// Storage handler for UniProt data
pub struct UniProtStorage {
    db: PgPool,
    s3: Option<Storage>,
    organization_id: Uuid,
    internal_version: String,
    external_version: String,
}

impl UniProtStorage {
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

    /// Store a batch of parsed entries
    pub async fn store_entries(&self, entries: &[UniProtEntry]) -> Result<usize> {
        info!("Storing {} UniProt entries", entries.len());

        let mut stored_count = 0;

        for entry in entries {
            if let Err(e) = self.store_entry(entry).await {
                debug!("Failed to store entry {}: {}", entry.accession, e);
                // Continue with other entries
                continue;
            }
            stored_count += 1;
        }

        info!("Successfully stored {}/{} entries", stored_count, entries.len());
        Ok(stored_count)
    }

    /// Store a single entry
    ///
    /// Creates: registry_entry -> data_source -> protein_metadata -> version -> version_file
    async fn store_entry(&self, entry: &UniProtEntry) -> Result<()> {
        debug!("Storing protein: {}", entry.accession);

        // 1. Get or create organism
        let organism_id = self.get_or_create_organism(entry).await?;

        // 2. Create registry entry (each protein is its own entry)
        let entry_id = self.create_registry_entry(entry).await?;

        // 3. Create data source
        self.create_data_source(entry_id, entry, organism_id).await?;

        // 4. Create protein metadata (with sequence deduplication and organism reference)
        self.create_protein_metadata(entry_id, entry, organism_id).await?;

        // 5. Create version with semantic versioning
        let version_id = self.create_version(entry_id).await?;

        // 6. Create version files for multiple formats (DAT, FASTA, JSON)
        self.create_version_files(entry, version_id).await?;

        debug!("Successfully stored protein: {}", entry.accession);
        Ok(())
    }

    /// Get or create organism as a data source
    ///
    /// Organisms are now data sources with organism_metadata, not a separate organisms table
    async fn get_or_create_organism(&self, entry: &UniProtEntry) -> Result<Uuid> {
        let ncbi_taxonomy_id = entry.taxonomy_id;

        // Check if organism exists
        let existing = sqlx::query_scalar::<_, Uuid>(
            "SELECT data_source_id FROM organism_metadata WHERE taxonomy_id = $1"
        )
        .bind(ncbi_taxonomy_id)
        .fetch_optional(&self.db)
        .await?;

        if let Some(id) = existing {
            return Ok(id);
        }

        // Create new organism as a data source
        let scientific_name = &entry.organism_name;
        let slug = format!("organism-{}", ncbi_taxonomy_id);

        // 1. Create registry entry
        let entry_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            ON CONFLICT (slug) DO UPDATE SET updated_at = NOW()
            RETURNING id
            "#
        )
        .bind(self.organization_id)
        .bind(&slug)
        .bind(scientific_name)
        .bind(format!("Organism: {}", scientific_name))
        .fetch_one(&self.db)
        .await?;

        // 2. Create data source
        sqlx::query(
            r#"
            INSERT INTO data_sources (id, source_type)
            VALUES ($1, 'organism')
            ON CONFLICT (id) DO NOTHING
            "#
        )
        .bind(entry_id)
        .execute(&self.db)
        .await?;

        // 3. Create organism metadata
        sqlx::query(
            r#"
            INSERT INTO organism_metadata (data_source_id, taxonomy_id, scientific_name)
            VALUES ($1, $2, $3)
            ON CONFLICT (taxonomy_id) DO NOTHING
            "#
        )
        .bind(entry_id)
        .bind(ncbi_taxonomy_id)
        .bind(scientific_name)
        .execute(&self.db)
        .await?;

        Ok(entry_id)
    }

    /// Create registry entry for the protein
    async fn create_registry_entry(&self, entry: &UniProtEntry) -> Result<Uuid> {
        let slug = &entry.accession;
        let name = format!("{} [{}]", entry.protein_name, entry.organism_name);
        let description = format!("UniProt protein: {}", entry.protein_name);

        let entry_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            ON CONFLICT (slug) DO UPDATE SET updated_at = NOW()
            RETURNING id
            "#
        )
        .bind(self.organization_id)
        .bind(slug)
        .bind(&name)
        .bind(&description)
        .fetch_one(&self.db)
        .await
        .context("Failed to create registry_entry")?;

        Ok(entry_id)
    }

    /// Create data source record
    async fn create_data_source(
        &self,
        entry_id: Uuid,
        entry: &UniProtEntry,
        organism_id: Uuid,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO data_sources (id, source_type, external_id, organism_id)
            VALUES ($1, 'protein', $2, $3)
            ON CONFLICT (id) DO NOTHING
            "#
        )
        .bind(entry_id)
        .bind(&entry.accession)
        .bind(organism_id)
        .execute(&self.db)
        .await
        .context("Failed to create data_source")?;

        Ok(())
    }

    /// Create protein_metadata record with deduplicated sequence
    async fn create_protein_metadata(&self, data_source_id: Uuid, entry: &UniProtEntry, organism_id: Uuid) -> Result<()> {
        // 1. Get or create deduplicated sequence
        let sequence_id = self.get_or_create_sequence(entry).await?;

        // 2. Compute sequence checksum for metadata
        let mut hasher = Sha256::new();
        hasher.update(entry.sequence.as_bytes());
        let sequence_checksum = format!("{:x}", hasher.finalize());

        // 3. Insert protein metadata
        sqlx::query(
            r#"
            INSERT INTO protein_metadata (
                data_source_id, accession, entry_name, protein_name, gene_name,
                sequence_length, mass_da, sequence_checksum,
                sequence_id, organism_id, uniprot_version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (accession) DO UPDATE SET
                entry_name = EXCLUDED.entry_name,
                protein_name = EXCLUDED.protein_name,
                gene_name = EXCLUDED.gene_name,
                sequence_id = EXCLUDED.sequence_id,
                organism_id = EXCLUDED.organism_id,
                uniprot_version = EXCLUDED.uniprot_version
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
        .bind(organism_id)
        .bind(&self.external_version)
        .execute(&self.db)
        .await
        .context("Failed to create protein_metadata")?;

        Ok(())
    }

    /// Get or create deduplicated protein sequence
    async fn get_or_create_sequence(&self, entry: &UniProtEntry) -> Result<Uuid> {
        // Compute sequence hash (SHA256)
        let mut hasher = Sha256::new();
        hasher.update(entry.sequence.as_bytes());
        let sequence_hash = format!("{:x}", hasher.finalize());

        // Check if sequence already exists
        let existing = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM protein_sequences WHERE sequence_hash = $1"
        )
        .bind(&sequence_hash)
        .fetch_optional(&self.db)
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
            "#
        )
        .bind(&entry.sequence)
        .bind(&sequence_hash)
        .bind(entry.sequence_length)
        .bind(&sequence_md5)
        .fetch_one(&self.db)
        .await
        .context("Failed to create protein_sequence")?;

        debug!("Created new deduplicated sequence (hash: {})", &sequence_hash[..16]);
        Ok(sequence_id)
    }

    /// Create version record with semantic versioning
    async fn create_version(&self, entry_id: Uuid) -> Result<Uuid> {
        // Parse internal version (e.g., "1.0" → major=1, minor=0, patch=0)
        let version_parts: Vec<&str> = self.internal_version.split('.').collect();
        let version_major = version_parts.first().and_then(|v| v.parse::<i32>().ok()).unwrap_or(1);
        let version_minor = version_parts.get(1).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
        let version_patch = version_parts.get(2).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);

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
            "#
        )
        .bind(entry_id)
        .bind(&self.internal_version)
        .bind(&self.external_version)
        .bind(version_major)
        .bind(version_minor)
        .bind(version_patch)
        .fetch_one(&self.db)
        .await
        .context("Failed to create version")?;

        Ok(version_id)
    }

    /// Create version_file records for multiple formats (DAT, FASTA, JSON)
    ///
    /// If S3 is configured, uploads files to S3 before creating database records.
    async fn create_version_files(&self, entry: &UniProtEntry, version_id: Uuid) -> Result<()> {
        // Base S3 path for this protein version
        let base_path = format!(
            "proteins/uniprot/{}/{}",
            entry.accession, self.internal_version
        );

        // 1. DAT format (original sequence data)
        let dat_content = entry.sequence.as_bytes().to_vec();
        let dat_checksum = self.compute_checksum(&dat_content);
        let dat_key = format!("{}/{}.dat", base_path, entry.accession);

        // Upload to S3 if configured
        if let Some(ref s3) = self.s3 {
            s3.upload(&dat_key, dat_content.clone(), Some("text/plain".to_string()))
                .await
                .context("Failed to upload DAT file to S3")?;
        }

        self.insert_version_file(
            version_id,
            "dat",
            &dat_key,
            &dat_checksum,
            dat_content.len() as i64,
        )
        .await?;

        // 2. FASTA format
        let fasta_content = entry.to_fasta();
        let fasta_checksum = self.compute_checksum(fasta_content.as_bytes());
        let fasta_key = format!("{}/{}.fasta", base_path, entry.accession);

        if let Some(ref s3) = self.s3 {
            s3.upload(&fasta_key, fasta_content.as_bytes().to_vec(), Some("text/plain".to_string()))
                .await
                .context("Failed to upload FASTA file to S3")?;
        }

        self.insert_version_file(
            version_id,
            "fasta",
            &fasta_key,
            &fasta_checksum,
            fasta_content.len() as i64,
        )
        .await?;

        // 3. JSON format
        let json_content = entry.to_json()?;
        let json_checksum = self.compute_checksum(json_content.as_bytes());
        let json_key = format!("{}/{}.json", base_path, entry.accession);

        if let Some(ref s3) = self.s3 {
            s3.upload(&json_key, json_content.as_bytes().to_vec(), Some("application/json".to_string()))
                .await
                .context("Failed to upload JSON file to S3")?;
        }

        self.insert_version_file(
            version_id,
            "json",
            &json_key,
            &json_checksum,
            json_content.len() as i64,
        )
        .await?;

        Ok(())
    }

    /// Insert a single version_file record
    async fn insert_version_file(
        &self,
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
            "#
        )
        .bind(version_id)
        .bind(format)
        .bind(s3_key)
        .bind(checksum)
        .bind(size_bytes)
        .execute(&self.db)
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
            "#
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
            "#
        )
        .bind(entry_id)
        .execute(&self.db)
        .await
        .context("Failed to create aggregate data_source")?;

        // 3. Create version with semantic versioning
        let version_parts: Vec<&str> = self.internal_version.split('.').collect();
        let version_major = version_parts.first().and_then(|v| v.parse::<i32>().ok()).unwrap_or(1);
        let version_minor = version_parts.get(1).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
        let version_patch = version_parts.get(2).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);

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
            "#
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
            "#
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
        for chunk in protein_entries.chunks(1000) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_creation() {
        let pool = PgPool::connect_lazy("postgresql://test").unwrap();
        let data_source_id = Uuid::new_v4();
        let version_id = Uuid::new_v4();

        let storage = UniProtStorage::new(pool, data_source_id, version_id);
        assert_eq!(storage.data_source_id, data_source_id);
        assert_eq!(storage.version_id, version_id);
    }
}
