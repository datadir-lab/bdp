// crates/bdp-ingest/src/pipelines/reactome/storage.rs

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use crate::common::batch::BatchConfig;
use crate::pipelines::reactome::models::*;

pub struct ReactomeStorage {
    pool: PgPool,
    batch: BatchConfig,
}

impl ReactomeStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            batch: BatchConfig::default(),
        }
    }

    pub async fn ingest_release(
        &self,
        org_id: Uuid,
        release: &str,
        pathways: &[Pathway],
        links: &[ProteinPathwayLink],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let data_source_id = self.upsert_registry(&mut tx, org_id).await?;
        self.upsert_version(&mut tx, data_source_id, release)
            .await?;

        info!(count = pathways.len(), "storing Reactome pathways");
        // Store pathways and collect reactome_id -> UUID map
        let pathway_id_map = self
            .store_pathways(&mut tx, data_source_id, pathways)
            .await?;

        info!(count = links.len(), "storing protein->pathway associations");
        self.store_links(&mut tx, &pathway_id_map, links).await?;

        tx.commit().await?;
        info!(release, "Reactome ingest complete");
        Ok(())
    }

    async fn upsert_registry(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        org_id: Uuid,
    ) -> Result<Uuid> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO registry_entries (organization_id, slug, name, entry_type)
             VALUES ($1, 'reactome', 'Reactome Pathway Database', 'data_source')
             ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name RETURNING id",
        )
        .bind(org_id)
        .fetch_one(&mut **tx)
        .await?;

        sqlx::query(
            "INSERT INTO data_sources (id, source_type, external_id) \
             VALUES ($1, 'pathway', 'reactome') ON CONFLICT (id) DO NOTHING",
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
            "INSERT INTO versions (entry_id, version, release_date) \
             VALUES ($1, $2, CURRENT_DATE) ON CONFLICT (entry_id, version) DO NOTHING",
        )
        .bind(ds_id)
        .bind(release)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Store pathways and return a map of reactome_id -> UUID.
    async fn store_pathways(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ds_id: Uuid,
        pathways: &[Pathway],
    ) -> Result<HashMap<String, Uuid>> {
        let mut id_map = HashMap::with_capacity(pathways.len());

        for chunk in pathways.chunks(self.batch.chunk_size) {
            for p in chunk {
                let id: Uuid = sqlx::query_scalar(
                    "INSERT INTO pathway_terms \
                     (data_source_id, reactome_id, name, species_name, reactome_release)
                     VALUES ($1, $2, $3, $4, $5)
                     ON CONFLICT (reactome_id, reactome_release)
                     DO UPDATE SET name = EXCLUDED.name, species_name = EXCLUDED.species_name
                     RETURNING id",
                )
                .bind(ds_id)
                .bind(&p.reactome_id)
                .bind(&p.name)
                .bind(&p.species_name)
                .bind(&p.reactome_release)
                .fetch_one(&mut **tx)
                .await
                .context("insert pathway term")?;

                id_map.insert(p.reactome_id.clone(), id);
            }
        }

        Ok(id_map)
    }

    async fn store_links(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        pathway_id_map: &HashMap<String, Uuid>,
        links: &[ProteinPathwayLink],
    ) -> Result<()> {
        let mut skipped = 0usize;

        for chunk in links.chunks(self.batch.chunk_size) {
            for link in chunk {
                let pathway_uuid = match pathway_id_map.get(&link.reactome_id) {
                    Some(id) => id,
                    None => {
                        // Pathway not in pathway_terms (e.g., cross-species link)
                        skipped += 1;
                        continue;
                    },
                };

                sqlx::query(
                    "INSERT INTO protein_pathway_associations
                     (uniprot_acc, pathway_id, reactome_id, evidence_type, species_name, reactome_release)
                     VALUES ($1, $2, $3, $4, $5, $6)
                     ON CONFLICT (uniprot_acc, pathway_id, reactome_release) DO NOTHING",
                )
                .bind(&link.uniprot_acc)
                .bind(pathway_uuid)
                .bind(&link.reactome_id)
                .bind(&link.evidence_type)
                .bind(&link.species_name)
                .bind(&link.reactome_release)
                .execute(&mut **tx)
                .await
                .context("insert protein_pathway_association")?;
            }
        }

        if skipped > 0 {
            info!(skipped, "skipped links for unknown pathways");
        }

        Ok(())
    }
}
