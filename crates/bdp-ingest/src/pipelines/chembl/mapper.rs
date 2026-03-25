use anyhow::Result;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use uuid::Uuid;

pub async fn build_compound_map(pool: &PgPool, inchikeys: &[String]) -> Result<HashMap<String, Uuid>> {
    if inchikeys.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query(
        "SELECT ct.inchikey, ct.id FROM compound_terms ct WHERE ct.inchikey = ANY($1)",
    )
    .bind(inchikeys)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .filter_map(|r| {
            let k: String = r.try_get("inchikey").ok()?;
            let id: Uuid = r.try_get("id").ok()?;
            Some((k, id))
        })
        .collect())
}

pub async fn build_target_map(
    pool: &PgPool,
    chembl_to_uniprot: &HashMap<String, String>,
) -> Result<HashMap<String, Uuid>> {
    let uniprots: Vec<String> = chembl_to_uniprot.values().cloned().collect();
    if uniprots.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query(
        "SELECT external_id, id FROM data_sources WHERE external_id = ANY($1) AND source_type = 'protein'",
    )
    .bind(&uniprots)
    .fetch_all(pool)
    .await?;
    let uniprot_map: HashMap<String, Uuid> = rows
        .iter()
        .filter_map(|r| {
            let ext: Option<String> = r.try_get("external_id").ok()?;
            let id: Uuid = r.try_get("id").ok()?;
            Some((ext?, id))
        })
        .collect();
    Ok(chembl_to_uniprot
        .iter()
        .filter_map(|(cid, uniprot)| uniprot_map.get(uniprot).map(|&id| (cid.clone(), id)))
        .collect())
}
