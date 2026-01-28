// Gene Ontology Storage Layer

use crate::ingest::citations::{gene_ontology_policy, setup_citation_policy_tx};
use crate::ingest::gene_ontology::{
    GoAnnotation, GoRelationship, GoTerm, Result, DEFAULT_ANNOTATION_CHUNK_SIZE,
    DEFAULT_RELATIONSHIP_CHUNK_SIZE, DEFAULT_TERM_CHUNK_SIZE,
};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub terms_stored: usize,
    pub relationships_stored: usize,
    pub annotations_stored: usize,
}

/// Storage handler for Gene Ontology data
pub struct GoStorage {
    db: PgPool,
    organization_id: Uuid,
    term_chunk_size: usize,
    relationship_chunk_size: usize,
    annotation_chunk_size: usize,
}

impl GoStorage {
    /// Create new storage handler with default chunk sizes
    pub fn new(db: PgPool, organization_id: Uuid) -> Self {
        Self {
            db,
            organization_id,
            term_chunk_size: DEFAULT_TERM_CHUNK_SIZE,
            relationship_chunk_size: DEFAULT_RELATIONSHIP_CHUNK_SIZE,
            annotation_chunk_size: DEFAULT_ANNOTATION_CHUNK_SIZE,
        }
    }

    /// Create storage handler with custom chunk sizes
    pub fn with_chunk_sizes(
        db: PgPool,
        organization_id: Uuid,
        term_chunk_size: usize,
        relationship_chunk_size: usize,
        annotation_chunk_size: usize,
    ) -> Self {
        Self {
            db,
            organization_id,
            term_chunk_size,
            relationship_chunk_size,
            annotation_chunk_size,
        }
    }

    // ========================================================================
    // GO Ontology Storage (Terms + Relationships)
    // ========================================================================

    /// Store GO ontology (terms and relationships)
    pub async fn store_ontology(
        &self,
        terms: &[GoTerm],
        relationships: &[GoRelationship],
        go_release_version: &str,
        internal_version: &str,
    ) -> Result<StorageStats> {
        info!(
            "Storing GO ontology: {} terms, {} relationships (version: {})",
            terms.len(),
            relationships.len(),
            go_release_version
        );

        let mut tx = self.db.begin().await?;

        // 1. Create data source for GO ontology
        let data_source_id = self
            .create_go_data_source(&mut tx, go_release_version, internal_version)
            .await?;

        // 2. Store terms in batches
        let terms_stored = self.store_terms(&mut tx, terms, data_source_id).await?;

        // 3. Store relationships in batches
        let relationships_stored = self.store_relationships(&mut tx, relationships).await?;

        tx.commit().await?;

        info!(
            "Successfully stored {} terms and {} relationships",
            terms_stored, relationships_stored
        );

        Ok(StorageStats {
            terms_stored,
            relationships_stored,
            annotations_stored: 0,
        })
    }

    /// Create data source for GO ontology
    async fn create_go_data_source(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        go_release_version: &str,
        internal_version: &str,
    ) -> Result<Uuid> {
        // Set up citation policy for Gene Ontology (idempotent)
        let policy_config = gene_ontology_policy(self.organization_id, None);
        setup_citation_policy_tx(tx, &policy_config)
            .await
            .map_err(|e| crate::ingest::gene_ontology::GoError::Validation(e.to_string()))?;

        // Create registry entry
        let entry_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO registry_entries (
                organization_id,
                source_type,
                name,
                description
            )
            VALUES ($1, 'go_term', 'Gene Ontology', 'Gene Ontology Consortium')
            ON CONFLICT (organization_id, source_type, name)
            DO UPDATE SET description = EXCLUDED.description
            RETURNING id
            "#,
        )
        .bind(self.organization_id)
        .fetch_one(&mut **tx)
        .await?;

        // Create data source
        let data_source_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO data_sources (
                registry_entry_id,
                source_type,
                external_id,
                metadata
            )
            VALUES ($1, 'go_term', $2, $3)
            ON CONFLICT (registry_entry_id, external_id)
            DO UPDATE SET metadata = EXCLUDED.metadata
            RETURNING id
            "#,
        )
        .bind(entry_id)
        .bind(go_release_version)
        .bind(serde_json::json!({
            "go_release_version": go_release_version,
            "source": "Gene Ontology Consortium",
            "url": format!("http://release.geneontology.org/{}/ontology/go-basic.obo", go_release_version)
        }))
        .fetch_one(&mut **tx)
        .await?;

        // Create version
        sqlx::query(
            r#"
            INSERT INTO versions (
                data_source_id,
                version_number,
                external_version,
                metadata
            )
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (data_source_id, version_number)
            DO NOTHING
            "#,
        )
        .bind(data_source_id)
        .bind(internal_version)
        .bind(go_release_version)
        .bind(serde_json::json!({
            "release_date": go_release_version,
            "ontology_type": "GO Basic OBO"
        }))
        .execute(&mut **tx)
        .await?;

        info!(
            "Created GO data source: {} (version: {}, internal: {})",
            data_source_id, go_release_version, internal_version
        );

        Ok(data_source_id)
    }

    /// Store GO terms in batches
    async fn store_terms(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        terms: &[GoTerm],
        data_source_id: Uuid,
    ) -> Result<usize> {
        let total_chunks = (terms.len() + self.term_chunk_size - 1) / self.term_chunk_size;
        let mut stored = 0;

        for (chunk_idx, chunk) in terms.chunks(self.term_chunk_size).enumerate() {
            info!(
                "Storing terms chunk {} / {} ({} terms)",
                chunk_idx + 1,
                total_chunks,
                chunk.len()
            );

            self.batch_insert_terms(tx, chunk, data_source_id).await?;
            stored += chunk.len();
        }

        Ok(stored)
    }

    /// Batch insert GO terms
    async fn batch_insert_terms(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        terms: &[GoTerm],
        data_source_id: Uuid,
    ) -> Result<()> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            r#"
            INSERT INTO go_term_metadata (
                data_source_id,
                go_id,
                go_accession,
                name,
                definition,
                namespace,
                is_obsolete,
                synonyms,
                xrefs,
                alt_ids,
                comments,
                go_release_version
            )
            "#,
        );

        query_builder.push_values(terms, |mut b, term| {
            b.push_bind(data_source_id)
                .push_bind(&term.go_id)
                .push_bind(term.go_accession)
                .push_bind(&term.name)
                .push_bind(&term.definition)
                .push_bind(term.namespace.as_str())
                .push_bind(term.is_obsolete)
                .push_bind(serde_json::to_value(&term.synonyms).unwrap_or(serde_json::json!([])))
                .push_bind(serde_json::to_value(&term.xrefs).unwrap_or(serde_json::json!([])))
                .push_bind(serde_json::to_value(&term.alt_ids).unwrap_or(serde_json::json!([])))
                .push_bind(&term.comments)
                .push_bind(&term.go_release_version);
        });

        query_builder.push(
            r#"
            ON CONFLICT (go_id, go_release_version)
            DO UPDATE SET
                name = EXCLUDED.name,
                definition = EXCLUDED.definition,
                namespace = EXCLUDED.namespace,
                is_obsolete = EXCLUDED.is_obsolete,
                synonyms = EXCLUDED.synonyms,
                xrefs = EXCLUDED.xrefs,
                alt_ids = EXCLUDED.alt_ids,
                comments = EXCLUDED.comments,
                updated_at = NOW()
            "#,
        );

        query_builder.build().execute(&mut **tx).await?;

        Ok(())
    }

    /// Store GO relationships in batches
    async fn store_relationships(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        relationships: &[GoRelationship],
    ) -> Result<usize> {
        let total_chunks =
            (relationships.len() + self.relationship_chunk_size - 1) / self.relationship_chunk_size;
        let mut stored = 0;

        for (chunk_idx, chunk) in relationships.chunks(self.relationship_chunk_size).enumerate() {
            info!(
                "Storing relationships chunk {} / {} ({} relationships)",
                chunk_idx + 1,
                total_chunks,
                chunk.len()
            );

            self.batch_insert_relationships(tx, chunk).await?;
            stored += chunk.len();
        }

        Ok(stored)
    }

    /// Batch insert GO relationships
    async fn batch_insert_relationships(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        relationships: &[GoRelationship],
    ) -> Result<()> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            r#"
            INSERT INTO go_relationships (
                subject_go_id,
                object_go_id,
                relationship_type,
                go_release_version
            )
            "#,
        );

        query_builder.push_values(relationships, |mut b, rel| {
            b.push_bind(&rel.subject_go_id)
                .push_bind(&rel.object_go_id)
                .push_bind(rel.relationship_type.as_str())
                .push_bind(&rel.go_release_version);
        });

        query_builder.push(
            r#"
            ON CONFLICT (subject_go_id, object_go_id, relationship_type, go_release_version)
            DO NOTHING
            "#,
        );

        query_builder.build().execute(&mut **tx).await?;

        Ok(())
    }

    // ========================================================================
    // GO Annotations Storage
    // ========================================================================

    /// Store GO annotations
    pub async fn store_annotations(
        &self,
        annotations: &[GoAnnotation],
        goa_release_version: &str,
    ) -> Result<usize> {
        info!(
            "Storing {} GO annotations (version: {})",
            annotations.len(),
            goa_release_version
        );

        let mut tx = self.db.begin().await?;

        let total_chunks =
            (annotations.len() + self.annotation_chunk_size - 1) / self.annotation_chunk_size;
        let mut stored = 0;

        for (chunk_idx, chunk) in annotations.chunks(self.annotation_chunk_size).enumerate() {
            info!(
                "Storing annotations chunk {} / {} ({} annotations)",
                chunk_idx + 1,
                total_chunks,
                chunk.len()
            );

            self.batch_insert_annotations(&mut tx, chunk).await?;
            stored += chunk.len();
        }

        tx.commit().await?;

        info!("Successfully stored {} annotations", stored);

        Ok(stored)
    }

    /// Batch insert GO annotations
    async fn batch_insert_annotations(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        annotations: &[GoAnnotation],
    ) -> Result<()> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            r#"
            INSERT INTO go_annotations (
                entity_type,
                entity_id,
                go_id,
                evidence_code,
                qualifier,
                reference,
                with_from,
                annotation_source,
                assigned_by,
                annotation_date,
                taxonomy_id,
                annotation_extension,
                gene_product_form_id,
                goa_release_version
            )
            "#,
        );

        query_builder.push_values(annotations, |mut b, ann| {
            b.push_bind(ann.entity_type.as_str())
                .push_bind(ann.entity_id)
                .push_bind(&ann.go_id)
                .push_bind(&ann.evidence_code.0)
                .push_bind(&ann.qualifier)
                .push_bind(&ann.reference)
                .push_bind(&ann.with_from)
                .push_bind(&ann.annotation_source)
                .push_bind(&ann.assigned_by)
                .push_bind(ann.annotation_date)
                .push_bind(ann.taxonomy_id)
                .push_bind(&ann.annotation_extension)
                .push_bind(&ann.gene_product_form_id)
                .push_bind(&ann.goa_release_version);
        });

        query_builder.push(
            r#"
            ON CONFLICT (entity_type, entity_id, go_id, evidence_code, COALESCE(qualifier, ''), COALESCE(reference, ''), goa_release_version)
            DO NOTHING
            "#,
        );

        query_builder.build().execute(&mut **tx).await?;

        Ok(())
    }

    // ========================================================================
    // Utilities
    // ========================================================================

    /// Build protein accession -> entity_id lookup map
    pub async fn build_protein_lookup(&self) -> Result<HashMap<String, Uuid>> {
        info!("Building protein lookup map...");

        let rows: Vec<(String, Uuid)> = sqlx::query_as(
            r#"
            SELECT accession, data_source_id
            FROM protein_metadata
            WHERE accession IS NOT NULL
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        let lookup: HashMap<String, Uuid> = rows.into_iter().collect();

        info!("Built protein lookup map with {} entries", lookup.len());

        Ok(lookup)
    }

    /// Get database connection pool
    pub fn db(&self) -> &PgPool {
        &self.db
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::gene_ontology::{Namespace, RelationshipType};

    // Note: These tests require a database connection and are integration tests
    // Run with: cargo test --test go_integration_test

    #[test]
    fn test_storage_creation() {
        let db = PgPool::connect_lazy("postgresql://localhost/test").unwrap();
        let org_id = Uuid::new_v4();
        let storage = GoStorage::new(db, org_id);

        assert_eq!(storage.term_chunk_size, DEFAULT_TERM_CHUNK_SIZE);
        assert_eq!(
            storage.relationship_chunk_size,
            DEFAULT_RELATIONSHIP_CHUNK_SIZE
        );
        assert_eq!(
            storage.annotation_chunk_size,
            DEFAULT_ANNOTATION_CHUNK_SIZE
        );
    }

    #[test]
    fn test_storage_with_custom_chunk_sizes() {
        let db = PgPool::connect_lazy("postgresql://localhost/test").unwrap();
        let org_id = Uuid::new_v4();
        let storage = GoStorage::with_chunk_sizes(db, org_id, 100, 200, 300);

        assert_eq!(storage.term_chunk_size, 100);
        assert_eq!(storage.relationship_chunk_size, 200);
        assert_eq!(storage.annotation_chunk_size, 300);
    }
}
