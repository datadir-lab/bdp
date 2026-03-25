use anyhow::Result;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::warn;
use uuid::Uuid;

pub struct OpenTargetsStorage {
    pool: PgPool,
}

impl OpenTargetsStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn build_gene_id_map(
        &self,
        ensembl_to_uniprot: &HashMap<String, String>,
    ) -> Result<HashMap<String, Uuid>> {
        let uniprot_accs: Vec<String> = ensembl_to_uniprot.values().cloned().collect();
        if uniprot_accs.is_empty() {
            return Ok(HashMap::new());
        }

        let rows = sqlx::query(
            "SELECT external_id, id FROM data_sources WHERE external_id = ANY($1) AND source_type = 'protein'"
        )
        .bind(&uniprot_accs)
        .fetch_all(&self.pool)
        .await?;

        let uniprot_to_uuid: HashMap<String, Uuid> = rows
            .iter()
            .filter_map(|r| {
                let ext: Option<String> = r.try_get("external_id").ok()?;
                let id: Uuid = r.try_get("id").ok()?;
                Some((ext?, id))
            })
            .collect();

        let map: HashMap<String, Uuid> = ensembl_to_uniprot
            .iter()
            .filter_map(|(ensg, uniprot)| {
                uniprot_to_uuid.get(uniprot).map(|&id| (ensg.clone(), id))
            })
            .collect();

        Ok(map)
    }

    pub async fn build_disease_id_map(
        &self,
        mondo_ids: &[String],
    ) -> Result<HashMap<String, Uuid>> {
        if mondo_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query(
            "SELECT mondo_id, id FROM disease_terms WHERE mondo_id = ANY($1) AND is_obsolete = FALSE"
        )
        .bind(mondo_ids)
        .fetch_all(&self.pool)
        .await?;

        let map: HashMap<String, Uuid> = rows
            .iter()
            .filter_map(|r| {
                let mondo_id: String = r.try_get("mondo_id").ok()?;
                let id: Uuid = r.try_get("id").ok()?;
                Some((mondo_id, id))
            })
            .collect();

        Ok(map)
    }

    pub async fn insert_associations(
        &self,
        rows: &[(Uuid, Uuid, f32)],
        source_version: &str,
    ) -> Result<usize> {
        let mut inserted = 0usize;
        for chunk in rows.chunks(500) {
            let gene_ids: Vec<Uuid> = chunk.iter().map(|r| r.0).collect();
            let disease_ids: Vec<Uuid> = chunk.iter().map(|r| r.1).collect();
            let scores: Vec<f32> = chunk.iter().map(|r| r.2).collect();
            let versions: Vec<&str> = chunk.iter().map(|_| source_version).collect();

            let result = sqlx::query(
                r#"INSERT INTO gene_disease_associations
                   (gene_id, disease_term_id, score, source, source_version)
                   SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::float4[], $4::text[], $5::text[])
                   AS t(gene_id, disease_term_id, score, source, source_version)
                   ON CONFLICT (gene_id, disease_term_id, source) DO UPDATE
                   SET score = EXCLUDED.score, source_version = EXCLUDED.source_version"#,
            )
            .bind(&gene_ids)
            .bind(&disease_ids)
            .bind(&scores)
            .bind(vec!["open_targets"; chunk.len()])
            .bind(&versions)
            .execute(&self.pool)
            .await;

            match result {
                Ok(r) => inserted += r.rows_affected() as usize,
                Err(e) => warn!("open_targets insert chunk error: {}", e),
            }
        }
        Ok(inserted)
    }
}
