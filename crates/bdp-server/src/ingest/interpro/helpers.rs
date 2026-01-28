// InterPro Cross-Reference Helpers
//
// Utilities for looking up protein, GO term, and other data sources
// when creating InterPro entries and protein matches.
//
// These helpers enable efficient batch lookups to minimize database queries.

use crate::error::Error;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::debug;
use uuid::Uuid;

// ============================================================================
// Protein Lookup Helper
// ============================================================================

/// Helper for looking up protein data sources and versions by UniProt accession
pub struct ProteinLookupHelper {
    /// Cache of UniProt accession -> (data_source_id, latest_version_id)
    cache: HashMap<String, (Uuid, Uuid)>,
}

impl ProteinLookupHelper {
    /// Create a new helper
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Batch load protein data sources and their latest versions
    ///
    /// This reduces N queries to 1 query for a batch of accessions
    pub async fn load_batch(
        &mut self,
        pool: &PgPool,
        accessions: &[String],
    ) -> Result<(), Error> {
        if accessions.is_empty() {
            return Ok(());
        }

        debug!("Loading {} protein data sources", accessions.len());

        // Query to get data_source_id and latest version_id for each accession
        let records = sqlx::query!(
            r#"
            SELECT
                pm.data_source_id,
                pm.accession,
                v.id as version_id,
                v.version_major,
                v.version_minor
            FROM protein_metadata pm
            JOIN data_sources ds ON ds.id = pm.data_source_id
            JOIN versions v ON v.entry_id = ds.id
            WHERE pm.accession = ANY($1)
            ORDER BY pm.accession, v.version_major DESC, v.version_minor DESC
            "#,
            accessions
        )
        .fetch_all(pool)
        .await?;

        // Group by accession and take the latest version
        let mut latest: HashMap<String, (Uuid, Uuid)> = HashMap::new();

        for record in records {
            latest
                .entry(record.accession.clone())
                .or_insert((record.data_source_id, record.version_id));
        }

        // Update cache
        self.cache.extend(latest);

        debug!("Loaded {} protein data sources into cache", self.cache.len());

        Ok(())
    }

    /// Get data_source_id and latest version_id for a UniProt accession
    ///
    /// Returns None if not found in cache
    pub fn get(&self, accession: &str) -> Option<(Uuid, Uuid)> {
        self.cache.get(accession).copied()
    }

    /// Check if accession is in cache
    pub fn contains(&self, accession: &str) -> bool {
        self.cache.contains_key(accession)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for ProteinLookupHelper {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GO Term Lookup Helper
// ============================================================================

/// Helper for looking up GO term data sources and versions by GO ID
pub struct GoTermLookupHelper {
    /// Cache of GO ID -> (data_source_id, latest_version_id)
    cache: HashMap<String, (Uuid, Uuid)>,
}

impl GoTermLookupHelper {
    /// Create a new helper
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Batch load GO term data sources and their latest versions
    pub async fn load_batch(&mut self, pool: &PgPool, go_ids: &[String]) -> Result<(), Error> {
        if go_ids.is_empty() {
            return Ok(());
        }

        debug!("Loading {} GO term data sources", go_ids.len());

        // Query to get data_source_id and latest version_id for each GO ID
        let records = sqlx::query!(
            r#"
            SELECT
                gtm.data_source_id,
                gtm.go_id,
                v.id as version_id,
                v.version_major,
                v.version_minor
            FROM go_term_metadata gtm
            JOIN data_sources ds ON ds.id = gtm.data_source_id
            JOIN versions v ON v.entry_id = ds.id
            WHERE gtm.go_id = ANY($1)
            ORDER BY gtm.go_id, v.version_major DESC, v.version_minor DESC
            "#,
            go_ids
        )
        .fetch_all(pool)
        .await?;

        // Group by GO ID and take the latest version
        let mut latest: HashMap<String, (Uuid, Uuid)> = HashMap::new();

        for record in records {
            latest
                .entry(record.go_id.clone())
                .or_insert((record.data_source_id, record.version_id));
        }

        // Update cache
        self.cache.extend(latest);

        debug!("Loaded {} GO term data sources into cache", self.cache.len());

        Ok(())
    }

    /// Get data_source_id and latest version_id for a GO ID
    pub fn get(&self, go_id: &str) -> Option<(Uuid, Uuid)> {
        self.cache.get(go_id).copied()
    }

    /// Check if GO ID is in cache
    pub fn contains(&self, go_id: &str) -> bool {
        self.cache.contains_key(go_id)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for GoTermLookupHelper {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// InterPro Entry Lookup Helper
// ============================================================================

/// Helper for looking up existing InterPro entry data sources
pub struct InterProEntryLookupHelper {
    /// Cache of InterPro ID -> data_source_id
    cache: HashMap<String, Uuid>,
}

impl InterProEntryLookupHelper {
    /// Create a new helper
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Batch load InterPro entry data sources
    pub async fn load_batch(
        &mut self,
        pool: &PgPool,
        interpro_ids: &[String],
    ) -> Result<(), Error> {
        if interpro_ids.is_empty() {
            return Ok(());
        }

        debug!("Loading {} InterPro entry data sources", interpro_ids.len());

        let records = sqlx::query!(
            r#"
            SELECT data_source_id, interpro_id
            FROM interpro_entry_metadata
            WHERE interpro_id = ANY($1)
            "#,
            interpro_ids
        )
        .fetch_all(pool)
        .await?;

        for record in records {
            self.cache
                .insert(record.interpro_id, record.data_source_id);
        }

        debug!(
            "Loaded {} InterPro entry data sources into cache",
            self.cache.len()
        );

        Ok(())
    }

    /// Get data_source_id for an InterPro ID
    pub fn get(&self, interpro_id: &str) -> Option<Uuid> {
        self.cache.get(interpro_id).copied()
    }

    /// Check if InterPro ID exists
    pub fn contains(&self, interpro_id: &str) -> bool {
        self.cache.contains_key(interpro_id)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for InterProEntryLookupHelper {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Signature Lookup Helper
// ============================================================================

/// Helper for looking up protein signature IDs
pub struct SignatureLookupHelper {
    /// Cache of (database, accession) -> signature_id
    cache: HashMap<(String, String), Uuid>,
}

impl SignatureLookupHelper {
    /// Create a new helper
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Batch load protein signatures
    pub async fn load_batch(
        &mut self,
        pool: &PgPool,
        signatures: &[(String, String)], // (database, accession)
    ) -> Result<(), Error> {
        if signatures.is_empty() {
            return Ok(());
        }

        debug!("Loading {} protein signatures", signatures.len());

        // Extract databases and accessions
        let databases: Vec<String> = signatures.iter().map(|(db, _)| db.clone()).collect();
        let accessions: Vec<String> = signatures.iter().map(|(_, acc)| acc.clone()).collect();

        let records = sqlx::query!(
            r#"
            SELECT id, database, accession
            FROM protein_signatures
            WHERE database = ANY($1) AND accession = ANY($2)
            "#,
            &databases,
            &accessions
        )
        .fetch_all(pool)
        .await?;

        for record in records {
            self.cache
                .insert((record.database, record.accession), record.id);
        }

        debug!("Loaded {} protein signatures into cache", self.cache.len());

        Ok(())
    }

    /// Get signature_id for a (database, accession) pair
    pub fn get(&self, database: &str, accession: &str) -> Option<Uuid> {
        self.cache
            .get(&(database.to_string(), accession.to_string()))
            .copied()
    }

    /// Check if signature exists
    pub fn contains(&self, database: &str, accession: &str) -> bool {
        self.cache
            .contains_key(&(database.to_string(), accession.to_string()))
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for SignatureLookupHelper {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protein_lookup_helper_new() {
        let helper = ProteinLookupHelper::new();
        assert_eq!(helper.cache_size(), 0);
    }

    #[test]
    fn test_go_term_lookup_helper_new() {
        let helper = GoTermLookupHelper::new();
        assert_eq!(helper.cache_size(), 0);
        assert!(!helper.contains("GO:0005515"));
    }

    #[test]
    fn test_interpro_entry_lookup_helper_new() {
        let helper = InterProEntryLookupHelper::new();
        assert_eq!(helper.cache_size(), 0);
    }

    #[test]
    fn test_signature_lookup_helper_new() {
        let helper = SignatureLookupHelper::new();
        assert_eq!(helper.cache_size(), 0);
        assert!(!helper.contains("Pfam", "PF00051"));
    }

    #[test]
    fn test_protein_lookup_helper_cache() {
        let mut helper = ProteinLookupHelper::new();
        let ds_id = Uuid::new_v4();
        let ver_id = Uuid::new_v4();

        helper.cache.insert("P12345".to_string(), (ds_id, ver_id));

        assert!(helper.contains("P12345"));
        assert_eq!(helper.get("P12345"), Some((ds_id, ver_id)));
        assert_eq!(helper.cache_size(), 1);

        helper.clear();
        assert_eq!(helper.cache_size(), 0);
    }
}
