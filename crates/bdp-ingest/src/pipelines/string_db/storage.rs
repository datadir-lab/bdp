// crates/bdp-ingest/src/pipelines/string_db/storage.rs

use anyhow::Result;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
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
            let neighborhood: Vec<i16> = chunk.iter().map(|r| r.2).collect();
            let fusion: Vec<i16> = chunk.iter().map(|r| r.3).collect();
            let cooccurrence: Vec<i16> = chunk.iter().map(|r| r.4).collect();
            let coexpression: Vec<i16> = chunk.iter().map(|r| r.5).collect();
            let experimental: Vec<i16> = chunk.iter().map(|r| r.6).collect();
            let database: Vec<i16> = chunk.iter().map(|r| r.7).collect();
            let textmining: Vec<i16> = chunk.iter().map(|r| r.8).collect();
            let combined: Vec<i16> = chunk.iter().map(|r| r.9).collect();

            let result = sqlx::query(
                r#"INSERT INTO protein_interactions
                   (protein_a_id, protein_b_id, score_neighborhood, score_fusion,
                    score_cooccurrence, score_coexpression, score_experimental,
                    score_database, score_textmining, combined_score)
                   SELECT * FROM UNNEST(
                       $1::uuid[], $2::uuid[],
                       $3::smallint[], $4::smallint[], $5::smallint[], $6::smallint[],
                       $7::smallint[], $8::smallint[], $9::smallint[], $10::smallint[]
                   ) AS t(protein_a_id, protein_b_id, score_neighborhood, score_fusion,
                          score_cooccurrence, score_coexpression, score_experimental,
                          score_database, score_textmining, combined_score)
                   ON CONFLICT (protein_a_id, protein_b_id) DO NOTHING"#,
            )
            .bind(&a_ids)
            .bind(&b_ids)
            .bind(&neighborhood)
            .bind(&fusion)
            .bind(&cooccurrence)
            .bind(&coexpression)
            .bind(&experimental)
            .bind(&database)
            .bind(&textmining)
            .bind(&combined)
            .execute(&self.pool)
            .await;

            match result {
                Ok(r) => inserted += r.rows_affected() as usize,
                Err(e) => return Err(anyhow::anyhow!("string_db insert failed: {}", e)),
            }
        }
        Ok(inserted)
    }
}
