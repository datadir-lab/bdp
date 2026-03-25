// HPO Storage Layer — writes to hpo_term_metadata, hpo_relationships,
// and disease_phenotype_annotations tables.

use crate::pipelines::hpo::{
    models::{DiseaseAnnotation, HpoRelationship, HpoTerm},
    DEFAULT_ANNOTATION_CHUNK_SIZE, DEFAULT_RELATIONSHIP_CHUNK_SIZE, DEFAULT_TERM_CHUNK_SIZE,
};
use anyhow::Result;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use tracing::info;
use uuid::Uuid;

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct HpoStorageStats {
    pub terms_stored: usize,
    pub relationships_stored: usize,
    pub annotations_stored: usize,
}

/// Storage handler for HPO data
pub struct HpoStorage {
    db: PgPool,
    organization_id: Uuid,
    term_chunk_size: usize,
    relationship_chunk_size: usize,
    annotation_chunk_size: usize,
}

impl HpoStorage {
    /// Create with default chunk sizes
    pub fn new(db: PgPool, organization_id: Uuid) -> Self {
        Self {
            db,
            organization_id,
            term_chunk_size: DEFAULT_TERM_CHUNK_SIZE,
            relationship_chunk_size: DEFAULT_RELATIONSHIP_CHUNK_SIZE,
            annotation_chunk_size: DEFAULT_ANNOTATION_CHUNK_SIZE,
        }
    }

    // ========================================================================
    // Top-level store methods
    // ========================================================================

    /// Store HPO terms and relationships
    pub async fn store_ontology(
        &self,
        terms: &[HpoTerm],
        relationships: &[HpoRelationship],
        release_version: &str,
        internal_version: &str,
    ) -> Result<HpoStorageStats> {
        info!(
            "Storing HPO ontology: {} terms, {} relationships (version: {})",
            terms.len(),
            relationships.len(),
            release_version
        );

        let mut tx = self.db.begin().await?;

        // 1. Ensure registry entry + data source + version exist
        self.upsert_hpo_data_source(&mut tx, release_version, internal_version)
            .await?;

        // 2. Store terms
        let terms_stored = self.store_terms(&mut tx, terms, release_version).await?;

        // 3. Store relationships
        let relationships_stored = self.store_relationships(&mut tx, relationships).await?;

        tx.commit().await?;

        info!(
            "HPO ontology stored: {} terms, {} relationships",
            terms_stored, relationships_stored
        );

        Ok(HpoStorageStats {
            terms_stored,
            relationships_stored,
            annotations_stored: 0,
        })
    }

    /// Store disease-phenotype annotations
    pub async fn store_annotations(
        &self,
        annotations: &[DiseaseAnnotation],
        release_version: &str,
    ) -> Result<usize> {
        info!("Storing {} disease-phenotype annotations", annotations.len());

        let mut total = 0;
        let chunk_size = self.annotation_chunk_size;

        for (idx, chunk) in annotations.chunks(chunk_size).enumerate() {
            let mut tx = self.db.begin().await?;
            self.batch_insert_annotations(&mut tx, chunk).await?;
            tx.commit().await?;
            total += chunk.len();

            if (idx + 1) % 10 == 0 {
                info!("Stored {} / {} annotations", total, annotations.len());
            }
        }

        info!("Stored {} annotations for version {}", total, release_version);
        Ok(total)
    }

    // ========================================================================
    // Internal helpers
    // ========================================================================

    /// Upsert registry_entry + data_source + version for HPO
    async fn upsert_hpo_data_source(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        release_version: &str,
        internal_version: &str,
    ) -> Result<Uuid> {
        // 1. registry_entry
        let entry_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO registry_entries (
                organization_id,
                slug,
                name,
                description,
                entry_type
            )
            VALUES ($1, 'human-phenotype-ontology', 'Human Phenotype Ontology',
                    'Standardized vocabulary of phenotypic abnormalities in human disease.', 'data_source')
            ON CONFLICT (slug)
            DO UPDATE SET description = EXCLUDED.description
            RETURNING id
            "#,
        )
        .bind(self.organization_id)
        .fetch_one(&mut **tx)
        .await?;

        // 2. data_source (shared PK)
        sqlx::query(
            r#"
            INSERT INTO data_sources (id, source_type, external_id)
            VALUES ($1, 'phenotype', $2)
            ON CONFLICT (id)
            DO UPDATE SET external_id = EXCLUDED.external_id
            "#,
        )
        .bind(entry_id)
        .bind(release_version)
        .execute(&mut **tx)
        .await?;

        // 3. version record
        sqlx::query(
            r#"
            INSERT INTO versions (entry_id, version, external_version, additional_metadata)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (entry_id, version)
            DO NOTHING
            "#,
        )
        .bind(entry_id)
        .bind(internal_version)
        .bind(release_version)
        .bind(serde_json::json!({
            "release_date": release_version,
            "ontology_type": "Human Phenotype Ontology"
        }))
        .execute(&mut **tx)
        .await?;

        info!(
            "Upserted HPO data source {} (version: {}, internal: {})",
            entry_id, release_version, internal_version
        );

        Ok(entry_id)
    }

    /// Store HPO terms in batches
    async fn store_terms(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        terms: &[HpoTerm],
        release_version: &str,
    ) -> Result<usize> {
        let chunk_size = self.term_chunk_size;
        let total_chunks = terms.len().div_ceil(chunk_size);
        let mut stored = 0;

        for (idx, chunk) in terms.chunks(chunk_size).enumerate() {
            info!("Storing HPO terms chunk {} / {} ({} terms)", idx + 1, total_chunks, chunk.len());
            self.batch_insert_terms(tx, chunk, release_version).await?;
            stored += chunk.len();
        }

        Ok(stored)
    }

    /// Batch insert HPO terms
    async fn batch_insert_terms(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        terms: &[HpoTerm],
        _release_version: &str,
    ) -> Result<()> {
        if terms.is_empty() {
            return Ok(());
        }

        // We need a data_source_id — fetch it once per batch by slug
        let data_source_id: Uuid = sqlx::query_scalar(
            r#"SELECT ds.id FROM data_sources ds
               JOIN registry_entries re ON re.id = ds.id
               WHERE re.slug = 'human-phenotype-ontology'
               LIMIT 1"#,
        )
        .fetch_one(&mut **tx)
        .await?;

        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            r#"INSERT INTO hpo_term_metadata (
                data_source_id, hpo_id, hpo_accession, name, definition, comment,
                is_obsolete, replaced_by, synonyms, xrefs, alt_ids, subset, hpo_release_version
            ) "#,
        );

        qb.push_values(terms, |mut b, term| {
            b.push_bind(data_source_id)
                .push_bind(&term.hpo_id)
                .push_bind(term.hpo_accession)
                .push_bind(&term.name)
                .push_bind(&term.definition)
                .push_bind(&term.comment)
                .push_bind(term.is_obsolete)
                .push_bind(&term.replaced_by)
                .push_bind(serde_json::to_value(&term.synonyms).unwrap_or(serde_json::json!([])))
                .push_bind(serde_json::to_value(&term.xrefs).unwrap_or(serde_json::json!([])))
                .push_bind(serde_json::to_value(&term.alt_ids).unwrap_or(serde_json::json!([])))
                .push_bind(serde_json::to_value(&term.subset).unwrap_or(serde_json::json!([])))
                .push_bind(&term.hpo_release_version);
        });

        qb.push(
            r#" ON CONFLICT (hpo_id, hpo_release_version)
            DO UPDATE SET
                name = EXCLUDED.name,
                definition = EXCLUDED.definition,
                comment = EXCLUDED.comment,
                is_obsolete = EXCLUDED.is_obsolete,
                replaced_by = EXCLUDED.replaced_by,
                synonyms = EXCLUDED.synonyms,
                xrefs = EXCLUDED.xrefs,
                alt_ids = EXCLUDED.alt_ids,
                subset = EXCLUDED.subset,
                updated_at = NOW()"#,
        );

        qb.build().execute(&mut **tx).await?;
        Ok(())
    }

    /// Store HPO relationships in batches
    async fn store_relationships(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        relationships: &[HpoRelationship],
    ) -> Result<usize> {
        let chunk_size = self.relationship_chunk_size;
        let total_chunks = relationships.len().div_ceil(chunk_size);
        let mut stored = 0;

        for (idx, chunk) in relationships.chunks(chunk_size).enumerate() {
            info!(
                "Storing HPO relationships chunk {} / {} ({} rels)",
                idx + 1,
                total_chunks,
                chunk.len()
            );
            self.batch_insert_relationships(tx, chunk).await?;
            stored += chunk.len();
        }

        Ok(stored)
    }

    /// Batch insert HPO relationships
    async fn batch_insert_relationships(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        relationships: &[HpoRelationship],
    ) -> Result<()> {
        if relationships.is_empty() {
            return Ok(());
        }

        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            r#"INSERT INTO hpo_relationships (
                subject_hpo_id, object_hpo_id, relationship_type, hpo_release_version
            ) "#,
        );

        qb.push_values(relationships, |mut b, rel| {
            b.push_bind(&rel.subject_hpo_id)
                .push_bind(&rel.object_hpo_id)
                .push_bind(&rel.relationship_type)
                .push_bind(&rel.hpo_release_version);
        });

        qb.push(
            r#" ON CONFLICT (subject_hpo_id, object_hpo_id, relationship_type, hpo_release_version)
            DO NOTHING"#,
        );

        qb.build().execute(&mut **tx).await?;
        Ok(())
    }

    /// Batch insert disease-phenotype annotations
    async fn batch_insert_annotations(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        annotations: &[DiseaseAnnotation],
    ) -> Result<()> {
        if annotations.is_empty() {
            return Ok(());
        }

        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            r#"INSERT INTO disease_phenotype_annotations (
                disease_db, disease_id, disease_name, hpo_id, qualifier, reference,
                evidence, onset, frequency, sex, modifier, aspect, biocuration,
                hpo_release_version
            ) "#,
        );

        qb.push_values(annotations, |mut b, ann| {
            b.push_bind(&ann.disease_db)
                .push_bind(&ann.disease_id)
                .push_bind(&ann.disease_name)
                .push_bind(&ann.hpo_id)
                .push_bind(&ann.qualifier)
                .push_bind(&ann.reference)
                .push_bind(&ann.evidence)
                .push_bind(&ann.onset)
                .push_bind(&ann.frequency)
                .push_bind(&ann.sex)
                .push_bind(&ann.modifier)
                .push_bind(&ann.aspect)
                .push_bind(&ann.biocuration)
                .push_bind(&ann.hpo_release_version);
        });

        qb.push(
            r#" ON CONFLICT (disease_db, disease_id, hpo_id, hpo_release_version)
            DO UPDATE SET
                disease_name = EXCLUDED.disease_name,
                qualifier = EXCLUDED.qualifier,
                reference = EXCLUDED.reference,
                evidence = EXCLUDED.evidence,
                onset = EXCLUDED.onset,
                frequency = EXCLUDED.frequency,
                sex = EXCLUDED.sex,
                modifier = EXCLUDED.modifier,
                aspect = EXCLUDED.aspect,
                biocuration = EXCLUDED.biocuration"#,
        );

        qb.build().execute(&mut **tx).await?;
        Ok(())
    }
}
