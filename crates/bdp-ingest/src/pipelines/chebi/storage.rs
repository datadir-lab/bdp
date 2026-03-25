// crates/bdp-ingest/src/pipelines/chebi/storage.rs

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction};
use tracing::info;
use uuid::Uuid;

use crate::common::batch::BatchConfig;
use crate::pipelines::chebi::models::*;

pub struct ChebiStorage {
    pool: PgPool,
    batch: BatchConfig,
}

impl ChebiStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            batch: BatchConfig::new(200), // smaller chunks for large ChEBI
        }
    }

    pub async fn ingest_release(
        &self,
        org_id: Uuid,
        release: &str,
        parsed: &ParsedChebi,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let data_source_id = self.upsert_registry(&mut tx, org_id).await?;
        self.upsert_version(&mut tx, data_source_id, release)
            .await?;

        info!(count = parsed.terms.len(), "storing ChEBI terms");
        self.store_terms(&mut tx, data_source_id, &parsed.terms)
            .await?;

        info!(count = parsed.relationships.len(), "storing ChEBI relationships");
        self.store_relationships(&mut tx, &parsed.relationships)
            .await?;

        tx.commit().await?;
        info!(release, "ChEBI ingest complete");
        Ok(())
    }

    async fn upsert_registry(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        org_id: Uuid,
    ) -> Result<Uuid> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO registry_entries (organization_id, slug, name, entry_type)
             VALUES ($1, 'chebi', 'ChEBI Chemical Entities of Biological Interest', 'data_source')
             ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name RETURNING id",
        )
        .bind(org_id)
        .fetch_one(&mut **tx)
        .await?;

        sqlx::query(
            "INSERT INTO data_sources (id, source_type, external_id)
             VALUES ($1, 'compound', 'chebi')
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .execute(&mut **tx)
        .await?;

        Ok(id)
    }

    async fn upsert_version(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ds_id: Uuid,
        release: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO versions (entry_id, version, release_date)
             VALUES ($1, $2, CURRENT_DATE)
             ON CONFLICT (entry_id, version) DO NOTHING",
        )
        .bind(ds_id)
        .bind(release)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn store_terms(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ds_id: Uuid,
        terms: &[CompoundTerm],
    ) -> Result<()> {
        for chunk in terms.chunks(self.batch.chunk_size) {
            for t in chunk {
                sqlx::query(
                    "INSERT INTO compound_terms
                     (data_source_id, chebi_id, chebi_accession, name, definition, comment,
                      is_obsolete, inchikey, smiles, inchi, formula, mass_mono, charge, chebi_release)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
                     ON CONFLICT (chebi_id, chebi_release)
                     DO UPDATE SET name=EXCLUDED.name, inchikey=EXCLUDED.inchikey,
                                   smiles=EXCLUDED.smiles, formula=EXCLUDED.formula,
                                   mass_mono=EXCLUDED.mass_mono, is_obsolete=EXCLUDED.is_obsolete",
                )
                .bind(ds_id)
                .bind(&t.chebi_id)
                .bind(t.chebi_accession)
                .bind(&t.name)
                .bind(&t.definition)
                .bind(&t.comment)
                .bind(t.is_obsolete)
                .bind(&t.inchikey)
                .bind(&t.smiles)
                .bind(&t.inchi)
                .bind(&t.formula)
                .bind(t.mass_mono)
                .bind(t.charge)
                .bind(&t.chebi_release)
                .execute(&mut **tx)
                .await
                .context("insert compound term")?;
            }
        }
        Ok(())
    }

    async fn store_relationships(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        rels: &[CompoundRelationship],
    ) -> Result<()> {
        for chunk in rels.chunks(self.batch.chunk_size) {
            for r in chunk {
                sqlx::query(
                    "INSERT INTO compound_relationships
                     (subject_chebi_id, object_chebi_id, relationship_type, chebi_release)
                     VALUES ($1,$2,$3,$4)
                     ON CONFLICT (subject_chebi_id, object_chebi_id, relationship_type, chebi_release)
                     DO NOTHING",
                )
                .bind(&r.subject_chebi_id)
                .bind(&r.object_chebi_id)
                .bind(&r.relationship_type)
                .bind(&r.chebi_release)
                .execute(&mut **tx)
                .await?;
            }
        }
        Ok(())
    }
}
