use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ActivityInsertRow {
    pub compound_id: Uuid,
    pub target_gene_id: Uuid,
    pub activity_type: Option<String>,
    pub activity_value: Option<f32>,
    pub activity_unit: Option<String>,
    pub relation: Option<String>,
    pub assay_type: Option<String>,
    pub chembl_assay_id: Option<String>,
    pub chembl_doc_id: Option<String>,
    pub confidence: Option<i16>,
    pub source_version: String,
}

pub struct ChemblStorage {
    pool: PgPool,
}

impl ChemblStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_activities(&self, rows: &[ActivityInsertRow]) -> Result<usize> {
        let mut inserted = 0usize;
        for chunk in rows.chunks(500) {
            let compound_ids: Vec<Uuid> = chunk.iter().map(|r| r.compound_id).collect();
            let target_ids: Vec<Uuid> = chunk.iter().map(|r| r.target_gene_id).collect();
            let types: Vec<Option<&str>> =
                chunk.iter().map(|r| r.activity_type.as_deref()).collect();
            let values: Vec<Option<f32>> = chunk.iter().map(|r| r.activity_value).collect();
            let units: Vec<Option<&str>> =
                chunk.iter().map(|r| r.activity_unit.as_deref()).collect();
            let relations: Vec<Option<&str>> =
                chunk.iter().map(|r| r.relation.as_deref()).collect();
            let assay_types: Vec<Option<&str>> =
                chunk.iter().map(|r| r.assay_type.as_deref()).collect();
            let assay_ids: Vec<Option<&str>> =
                chunk.iter().map(|r| r.chembl_assay_id.as_deref()).collect();
            let doc_ids: Vec<Option<&str>> =
                chunk.iter().map(|r| r.chembl_doc_id.as_deref()).collect();
            let confidences: Vec<Option<i16>> = chunk.iter().map(|r| r.confidence).collect();
            let versions: Vec<&str> = chunk.iter().map(|r| r.source_version.as_str()).collect();

            let result = sqlx::query(
                r#"INSERT INTO drug_target_activities
                   (compound_id, target_gene_id, activity_type, activity_value, activity_unit,
                    relation, assay_type, chembl_assay_id, chembl_doc_id, confidence, source_version)
                   SELECT * FROM UNNEST(
                       $1::uuid[], $2::uuid[], $3::text[], $4::float4[], $5::text[],
                       $6::text[], $7::text[], $8::text[], $9::text[], $10::int2[], $11::text[]
                   ) AS t(compound_id, target_gene_id, activity_type, activity_value, activity_unit,
                           relation, assay_type, chembl_assay_id, chembl_doc_id, confidence, source_version)
                   ON CONFLICT (compound_id, target_gene_id, chembl_assay_id) DO NOTHING"#,
            )
            .bind(&compound_ids)
            .bind(&target_ids)
            .bind(&types)
            .bind(&values)
            .bind(&units)
            .bind(&relations)
            .bind(&assay_types)
            .bind(&assay_ids)
            .bind(&doc_ids)
            .bind(&confidences)
            .bind(&versions)
            .execute(&self.pool)
            .await;

            match result {
                Ok(r) => inserted += r.rows_affected() as usize,
                Err(e) => return Err(anyhow::anyhow!("chembl insert error: {}", e)),
            }
        }
        Ok(inserted)
    }
}
