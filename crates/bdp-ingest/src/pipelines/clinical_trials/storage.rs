use anyhow::Result;
use sqlx::PgPool;
use tracing::warn;

use crate::pipelines::clinical_trials::aact_loader::AactStudyRow;

pub struct ClinicalTrialsStorage {
    pool: PgPool,
}

impl ClinicalTrialsStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_studies(&self, rows: &[AactStudyRow]) -> Result<usize> {
        let mut count = 0usize;
        for chunk in rows.chunks(200) {
            let nct_ids: Vec<&str> = chunk.iter().map(|r| r.nct_id.as_str()).collect();
            let titles: Vec<Option<&str>> =
                chunk.iter().map(|r| r.brief_title.as_deref()).collect();
            let statuses: Vec<Option<&str>> =
                chunk.iter().map(|r| r.overall_status.as_deref()).collect();
            let phases: Vec<Option<&str>> = chunk.iter().map(|r| r.phase.as_deref()).collect();

            let result = sqlx::query(
                r#"INSERT INTO clinical_trials (nct_id, title, status, phase)
                   SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[])
                   AS t(nct_id, title, status, phase)
                   ON CONFLICT (nct_id) DO UPDATE
                   SET title = EXCLUDED.title, status = EXCLUDED.status,
                       phase = EXCLUDED.phase, updated_at = NOW()"#,
            )
            .bind(&nct_ids)
            .bind(&titles)
            .bind(&statuses)
            .bind(&phases)
            .execute(&self.pool)
            .await;

            match result {
                Ok(r) => count += r.rows_affected() as usize,
                Err(e) => warn!("clinical_trials upsert error: {}", e),
            }
        }
        Ok(count)
    }
}
