//! Helper module for integrating UniProt with NCBI Taxonomy database
//!
//! Provides functions to:
//! - Look up existing taxonomy entries
//! - Create taxonomy stubs for missing entries
//! - Resolve taxonomy IDs to data_source_ids for foreign key relationships

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

/// Helper for managing taxonomy references in UniProt ingestion
pub struct TaxonomyHelper {
    db: PgPool,
    organization_id: Uuid,
    /// Cache of taxonomy_id -> data_source_id mappings
    cache: HashMap<i32, Uuid>,
}

impl TaxonomyHelper {
    /// Create a new taxonomy helper
    pub fn new(db: PgPool, organization_id: Uuid) -> Self {
        Self {
            db,
            organization_id,
            cache: HashMap::new(),
        }
    }

    /// Get or create data_source_id for a taxonomy ID
    ///
    /// This method:
    /// 1. Checks the cache first
    /// 2. Queries the database for existing taxonomy entries
    /// 3. Creates a stub entry if not found
    ///
    /// Returns the data_source_id that can be used as a foreign key
    pub async fn get_or_create_taxonomy(
        &mut self,
        taxonomy_id: i32,
        scientific_name: &str,
        lineage: &[String],
    ) -> Result<Uuid> {
        // Check cache first
        if let Some(&data_source_id) = self.cache.get(&taxonomy_id) {
            return Ok(data_source_id);
        }

        // Check database
        if let Some(data_source_id) = self.lookup_taxonomy(taxonomy_id).await? {
            self.cache.insert(taxonomy_id, data_source_id);
            return Ok(data_source_id);
        }

        // Create stub entry
        let data_source_id = self
            .create_taxonomy_stub(taxonomy_id, scientific_name, lineage)
            .await?;

        self.cache.insert(taxonomy_id, data_source_id);
        Ok(data_source_id)
    }

    /// Look up existing taxonomy entry by taxonomy ID
    async fn lookup_taxonomy(&self, taxonomy_id: i32) -> Result<Option<Uuid>> {
        let result = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT data_source_id
            FROM taxonomy_metadata
            WHERE taxonomy_id = $1
            LIMIT 1
            "#,
        )
        .bind(taxonomy_id)
        .fetch_optional(&self.db)
        .await
        .context("Failed to lookup taxonomy")?;

        Ok(result)
    }

    /// Create a stub taxonomy entry for a taxonomy ID
    ///
    /// This creates minimal entries in:
    /// - registry_entries
    /// - data_sources
    /// - taxonomy_metadata
    ///
    /// These stubs can be filled in later by running the NCBI Taxonomy ingestion
    async fn create_taxonomy_stub(
        &self,
        taxonomy_id: i32,
        scientific_name: &str,
        lineage: &[String],
    ) -> Result<Uuid> {
        let mut tx = self
            .db
            .begin()
            .await
            .context("Failed to begin transaction")?;

        // 1. Create registry entry
        let slug = format!("{}", taxonomy_id);
        let description = format!("NCBI Taxonomy: {} (stub)", scientific_name);

        let registry_entry_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            ON CONFLICT (slug) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(self.organization_id)
        .bind(&slug)
        .bind(scientific_name)
        .bind(&description)
        .fetch_one(&mut *tx)
        .await
        .context("Failed to create registry entry")?;

        // 2. Create data source entry
        sqlx::query(
            r#"
            INSERT INTO data_sources (id, source_type)
            VALUES ($1, 'taxonomy')
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(registry_entry_id)
        .execute(&mut *tx)
        .await
        .context("Failed to create data source")?;

        // 3. Create taxonomy metadata entry (stub)
        let lineage_str = lineage.join("; ");

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
                lineage = EXCLUDED.lineage,
                parsed_at = NOW()
            "#,
        )
        .bind(registry_entry_id)
        .bind(taxonomy_id)
        .bind(scientific_name)
        .bind(Option::<String>::None) // common_name
        .bind("stub") // rank
        .bind(&lineage_str) // lineage
        .bind("stub-from-uniprot") // ncbi_tax_version
        .execute(&mut *tx)
        .await
        .context("Failed to create taxonomy metadata")?;

        tx.commit().await.context("Failed to commit transaction")?;

        tracing::debug!(
            taxonomy_id = taxonomy_id,
            scientific_name = scientific_name,
            data_source_id = %registry_entry_id,
            "Created taxonomy stub"
        );

        Ok(registry_entry_id)
    }

    /// Batch lookup taxonomy IDs
    ///
    /// Returns a map of taxonomy_id -> data_source_id for all found entries
    pub async fn batch_lookup_taxonomies(
        &mut self,
        taxonomy_ids: &[i32],
    ) -> Result<HashMap<i32, Uuid>> {
        if taxonomy_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // Filter out already cached IDs
        let uncached_ids: Vec<i32> = taxonomy_ids
            .iter()
            .filter(|&&id| !self.cache.contains_key(&id))
            .copied()
            .collect();

        if uncached_ids.is_empty() {
            // All IDs are in cache
            return Ok(taxonomy_ids
                .iter()
                .filter_map(|&id| self.cache.get(&id).map(|&ds_id| (id, ds_id)))
                .collect());
        }

        // Query database for uncached IDs
        let rows = sqlx::query_as::<_, (i32, Uuid)>(
            r#"
            SELECT taxonomy_id, data_source_id
            FROM taxonomy_metadata
            WHERE taxonomy_id = ANY($1)
            "#,
        )
        .bind(&uncached_ids)
        .fetch_all(&self.db)
        .await
        .context("Failed to batch lookup taxonomies")?;

        // Update cache
        for (taxonomy_id, data_source_id) in &rows {
            self.cache.insert(*taxonomy_id, *data_source_id);
        }

        // Build result map from cache
        Ok(taxonomy_ids
            .iter()
            .filter_map(|&id| self.cache.get(&id).map(|&ds_id| (id, ds_id)))
            .collect())
    }

    /// Batch create taxonomy stubs for missing entries
    ///
    /// Creates stubs for all taxonomy IDs that don't exist in the database
    pub async fn batch_create_stubs(
        &mut self,
        entries: &[(i32, String, Vec<String>)], // (taxonomy_id, scientific_name, lineage)
    ) -> Result<HashMap<i32, Uuid>> {
        if entries.is_empty() {
            return Ok(HashMap::new());
        }

        // First, do a batch lookup to see which ones already exist
        let taxonomy_ids: Vec<i32> = entries.iter().map(|(id, _, _)| *id).collect();
        let existing = self.batch_lookup_taxonomies(&taxonomy_ids).await?;

        // Filter out existing entries
        let missing: Vec<_> = entries
            .iter()
            .filter(|(id, _, _)| !existing.contains_key(id))
            .collect();

        if missing.is_empty() {
            return Ok(existing);
        }

        // Create stubs for missing entries
        let mut result = existing;

        for (taxonomy_id, scientific_name, lineage) in missing {
            match self
                .create_taxonomy_stub(*taxonomy_id, scientific_name, lineage)
                .await
            {
                Ok(data_source_id) => {
                    result.insert(*taxonomy_id, data_source_id);
                },
                Err(e) => {
                    tracing::warn!(
                        taxonomy_id = taxonomy_id,
                        error = %e,
                        "Failed to create taxonomy stub"
                    );
                },
            }
        }

        Ok(result)
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a database connection and are integration tests
    // They should be run with `cargo test --test '*'` or in a CI environment with a test database

    #[test]
    fn test_taxonomy_helper_creation() {
        // This is a basic compile-time test
        // Real tests would require a database connection
        let _organization_id = Uuid::new_v4();
        // TaxonomyHelper::new(pool, organization_id);
    }
}
