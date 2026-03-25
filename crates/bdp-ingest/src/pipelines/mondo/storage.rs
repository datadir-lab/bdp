// MONDO Storage Layer
//
// Writes parsed MONDO data to PostgreSQL following the BDP registry chain:
//   registry_entries → data_sources → versions → disease_terms + disease_relationships

use crate::pipelines::mondo::models::{DiseaseRelationship, DiseaseTerm, ParsedMondo};
use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction};
use tracing::info;
use uuid::Uuid;

const TERM_CHUNK_SIZE: usize = 500;
const REL_CHUNK_SIZE: usize = 500;
const SYN_CHUNK_SIZE: usize = 500;
const XREF_CHUNK_SIZE: usize = 500;

/// Storage statistics for a MONDO ingest run.
#[derive(Debug, Clone, Default)]
pub struct MondoStorageStats {
    pub terms_stored: usize,
    pub relationships_stored: usize,
    pub synonyms_stored: usize,
    pub xrefs_stored: usize,
}

/// Handles all PostgreSQL writes for a MONDO ingest run.
pub struct MondoStorage {
    db: PgPool,
    organization_id: Uuid,
}

impl MondoStorage {
    pub fn new(db: PgPool, organization_id: Uuid) -> Self {
        Self {
            db,
            organization_id,
        }
    }

    /// Full ingest: upsert registry, store terms + relationships + synonyms + xrefs.
    pub async fn store_release(
        &self,
        release: &str,
        parsed: &ParsedMondo,
    ) -> Result<MondoStorageStats> {
        info!(
            release,
            terms = parsed.term_count(),
            rels = parsed.relationship_count(),
            "Storing MONDO release"
        );

        let mut tx = self.db.begin().await.context("begin transaction")?;

        let data_source_id = self
            .upsert_registry(&mut tx, release)
            .await
            .context("upsert registry")?;

        let (terms_stored, term_ids) = self
            .store_terms(&mut tx, data_source_id, &parsed.terms)
            .await
            .context("store disease terms")?;

        let synonyms_stored = self
            .store_synonyms(&mut tx, &parsed.terms, &term_ids)
            .await
            .context("store synonyms")?;

        let xrefs_stored = self
            .store_xrefs(&mut tx, &parsed.terms, &term_ids)
            .await
            .context("store xrefs")?;

        let relationships_stored = self
            .store_relationships(&mut tx, &parsed.relationships)
            .await
            .context("store relationships")?;

        tx.commit().await.context("commit transaction")?;

        let stats = MondoStorageStats {
            terms_stored,
            relationships_stored,
            synonyms_stored,
            xrefs_stored,
        };

        info!(
            release,
            terms = stats.terms_stored,
            rels = stats.relationships_stored,
            "MONDO release stored"
        );

        Ok(stats)
    }

    /// Upsert registry_entry, data_source, and version for MONDO.
    async fn upsert_registry(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        release: &str,
    ) -> Result<Uuid> {
        // registry_entries
        let entry_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
            VALUES ($1, 'mondo', 'MONDO Disease Ontology',
                    'Monarch Disease Ontology — unified disease terminology', 'data_source')
            ON CONFLICT (slug)
            DO UPDATE SET description = EXCLUDED.description
            RETURNING id
            "#,
        )
        .bind(self.organization_id)
        .fetch_one(&mut **tx)
        .await
        .context("upsert registry_entries")?;

        // data_sources (shared PK with registry_entries)
        sqlx::query(
            r#"
            INSERT INTO data_sources (id, source_type, external_id)
            VALUES ($1, 'disease', 'mondo')
            ON CONFLICT (id) DO UPDATE SET external_id = EXCLUDED.external_id
            "#,
        )
        .bind(entry_id)
        .execute(&mut **tx)
        .await
        .context("upsert data_sources")?;

        // versions
        sqlx::query(
            r#"
            INSERT INTO versions (entry_id, version, external_version, additional_metadata)
            VALUES ($1, $2, $2, $3)
            ON CONFLICT (entry_id, version) DO NOTHING
            "#,
        )
        .bind(entry_id)
        .bind(release)
        .bind(serde_json::json!({ "release_date": release, "ontology_type": "MONDO OBO" }))
        .execute(&mut **tx)
        .await
        .context("upsert versions")?;

        info!(
            data_source_id = %entry_id,
            release,
            "Upserted MONDO registry entry"
        );

        Ok(entry_id)
    }

    /// Insert disease_terms, returning (count, Vec<Uuid>) in the same order as `terms`.
    async fn store_terms(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        data_source_id: Uuid,
        terms: &[DiseaseTerm],
    ) -> Result<(usize, Vec<Uuid>)> {
        let total_chunks = (terms.len() + TERM_CHUNK_SIZE - 1).max(1) / TERM_CHUNK_SIZE;
        let mut ids: Vec<Uuid> = Vec::with_capacity(terms.len());

        for (chunk_idx, chunk) in terms.chunks(TERM_CHUNK_SIZE).enumerate() {
            info!(
                "Storing disease_terms chunk {}/{} ({} terms)",
                chunk_idx + 1,
                total_chunks,
                chunk.len()
            );

            for term in chunk {
                let id: Uuid = sqlx::query_scalar(
                    r#"
                    INSERT INTO disease_terms (
                        data_source_id, mondo_id, mondo_accession, name, definition,
                        is_obsolete, comment, omim_id, orphanet_id, mondo_release
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                    ON CONFLICT (mondo_id, mondo_release)
                    DO UPDATE SET
                        name        = EXCLUDED.name,
                        definition  = EXCLUDED.definition,
                        is_obsolete = EXCLUDED.is_obsolete,
                        comment     = EXCLUDED.comment,
                        omim_id     = EXCLUDED.omim_id,
                        orphanet_id = EXCLUDED.orphanet_id,
                        updated_at  = NOW()
                    RETURNING id
                    "#,
                )
                .bind(data_source_id)
                .bind(&term.mondo_id)
                .bind(term.mondo_accession)
                .bind(&term.name)
                .bind(&term.definition)
                .bind(term.is_obsolete)
                .bind(&term.comment)
                .bind(&term.omim_id)
                .bind(&term.orphanet_id)
                .bind(&term.mondo_release)
                .fetch_one(&mut **tx)
                .await
                .context("insert disease_term")?;

                ids.push(id);
            }
        }

        Ok((ids.len(), ids))
    }

    /// Insert disease_term_synonyms.
    async fn store_synonyms(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        terms: &[DiseaseTerm],
        term_ids: &[Uuid],
    ) -> Result<usize> {
        let mut total = 0;

        // Collect all (term_id, scope, text) tuples
        let mut rows: Vec<(Uuid, String, String)> = Vec::new();
        for (term, &term_id) in terms.iter().zip(term_ids.iter()) {
            for syn in &term.synonyms {
                rows.push((term_id, syn.scope.clone(), syn.text.clone()));
            }
        }

        for chunk in rows.chunks(SYN_CHUNK_SIZE) {
            for (term_id, scope, text) in chunk {
                sqlx::query(
                    r#"
                    INSERT INTO disease_term_synonyms (term_id, scope, text)
                    VALUES ($1, $2, $3)
                    ON CONFLICT DO NOTHING
                    "#,
                )
                .bind(term_id)
                .bind(scope)
                .bind(text)
                .execute(&mut **tx)
                .await
                .context("insert disease_term_synonym")?;

                total += 1;
            }
        }

        Ok(total)
    }

    /// Insert disease_term_xrefs.
    async fn store_xrefs(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        terms: &[DiseaseTerm],
        term_ids: &[Uuid],
    ) -> Result<usize> {
        let mut total = 0;

        let mut rows: Vec<(Uuid, String, String)> = Vec::new();
        for (term, &term_id) in terms.iter().zip(term_ids.iter()) {
            for xref in &term.xrefs {
                rows.push((term_id, xref.source_db.clone(), xref.source_id.clone()));
            }
        }

        for chunk in rows.chunks(XREF_CHUNK_SIZE) {
            for (term_id, source_db, source_id) in chunk {
                sqlx::query(
                    r#"
                    INSERT INTO disease_term_xrefs (term_id, source_db, source_id)
                    VALUES ($1, $2, $3)
                    ON CONFLICT DO NOTHING
                    "#,
                )
                .bind(term_id)
                .bind(source_db)
                .bind(source_id)
                .execute(&mut **tx)
                .await
                .context("insert disease_term_xref")?;

                total += 1;
            }
        }

        Ok(total)
    }

    /// Insert disease_relationships.
    async fn store_relationships(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        relationships: &[DiseaseRelationship],
    ) -> Result<usize> {
        let total_chunks = (relationships.len() + REL_CHUNK_SIZE - 1).max(1) / REL_CHUNK_SIZE;
        let mut stored = 0;

        for (chunk_idx, chunk) in relationships.chunks(REL_CHUNK_SIZE).enumerate() {
            info!(
                "Storing disease_relationships chunk {}/{} ({} rels)",
                chunk_idx + 1,
                total_chunks,
                chunk.len()
            );

            for rel in chunk {
                sqlx::query(
                    r#"
                    INSERT INTO disease_relationships (
                        subject_mondo_id, object_mondo_id, relationship_type, mondo_release
                    ) VALUES ($1, $2, $3, $4)
                    ON CONFLICT (subject_mondo_id, object_mondo_id, relationship_type, mondo_release)
                    DO NOTHING
                    "#,
                )
                .bind(&rel.subject_mondo_id)
                .bind(&rel.object_mondo_id)
                .bind(rel.relationship_type.as_str())
                .bind(&rel.mondo_release)
                .execute(&mut **tx)
                .await
                .context("insert disease_relationship")?;

                stored += 1;
            }
        }

        Ok(stored)
    }
}
