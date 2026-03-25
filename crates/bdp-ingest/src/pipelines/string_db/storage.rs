// crates/bdp-ingest/src/pipelines/string_db/storage.rs

use anyhow::Result;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::warn;
use uuid::Uuid;

pub struct StringStorage {
    pool: PgPool,
}

impl StringStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn build_protein_map(
        &self,
        ensp_to_uniprot: &HashMap<String, String>,
    ) -> Result<HashMap<String, Uuid>> {
        let uniprots: Vec<String> = ensp_to_uniprot.values().cloned().collect();
        if uniprots.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query(
            "SELECT external_id, id FROM data_sources WHERE external_id = ANY($1) AND source_type = 'protein'"
        )
        .bind(&uniprots)
        .fetch_all(&self.pool)
        .await?;

        let uniprot_map: HashMap<String, Uuid> = rows
            .iter()
            .filter_map(|r| {
                let ext: Option<String> = r.try_get("external_id").ok()?;
                let id: Uuid = r.try_get("id").ok()?;
                Some((ext?, id))
            })
            .collect();

        Ok(ensp_to_uniprot
            .iter()
            .filter_map(|(ensp, uniprot)| {
                uniprot_map.get(uniprot).map(|&id| (ensp.clone(), id))
            })
            .collect())
    }

    pub async fn insert_interactions(
        &self,
        rows: &[(Uuid, Uuid, i16, i16, i16, i16, i16, i16, i16, i16)],
    ) -> Result<usize> {
        let mut inserted = 0usize;
        for chunk in rows.chunks(1000) {
            let a_ids: Vec<Uuid> = chunk.iter().map(|r| r.0).collect();
            let b_ids: Vec<Uuid> = chunk.iter().map(|r| r.1).collect();
            let combined: Vec<i16> = chunk.iter().map(|r| r.9).collect();
            let experimental: Vec<i16> = chunk.iter().map(|r| r.6).collect();

            let result = sqlx::query(
                r#"INSERT INTO protein_interactions
                   (protein_a_id, protein_b_id, score_experimental, combined_score)
                   SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::smallint[], $4::smallint[])
                   AS t(protein_a_id, protein_b_id, score_experimental, combined_score)
                   ON CONFLICT (protein_a_id, protein_b_id) DO NOTHING"#,
            )
            .bind(&a_ids)
            .bind(&b_ids)
            .bind(&experimental)
            .bind(&combined)
            .execute(&self.pool)
            .await;

            match result {
                Ok(r) => inserted += r.rows_affected() as usize,
                Err(e) => warn!("string insert error: {}", e),
            }
        }
        Ok(inserted)
    }
}
