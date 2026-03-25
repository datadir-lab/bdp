use anyhow::Result;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

pub struct ChemblStorage {
    pool: PgPool,
}

impl ChemblStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_activities(
        &self,
        rows: &[(Uuid, Uuid, Option<String>, Option<f32>, String)],
    ) -> Result<usize> {
        let mut inserted = 0usize;
        for chunk in rows.chunks(500) {
            let compound_ids: Vec<Uuid> = chunk.iter().map(|r| r.0).collect();
            let target_ids: Vec<Uuid> = chunk.iter().map(|r| r.1).collect();
            let types: Vec<Option<&str>> = chunk.iter().map(|r| r.2.as_deref()).collect();
            let values: Vec<Option<f32>> = chunk.iter().map(|r| r.3).collect();
            let versions: Vec<&str> = chunk.iter().map(|r| r.4.as_str()).collect();

            let result = sqlx::query(
                r#"INSERT INTO drug_target_activities
                   (compound_id, target_gene_id, activity_type, activity_value, source_version)
                   SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::text[], $4::float4[], $5::text[])
                   AS t(compound_id, target_gene_id, activity_type, activity_value, source_version)
                   ON CONFLICT DO NOTHING"#,
            )
            .bind(&compound_ids)
            .bind(&target_ids)
            .bind(&types)
            .bind(&values)
            .bind(&versions)
            .execute(&self.pool)
            .await;

            match result {
                Ok(r) => inserted += r.rows_affected() as usize,
                Err(e) => warn!("chembl insert error: {}", e),
            }
        }
        Ok(inserted)
    }
}
