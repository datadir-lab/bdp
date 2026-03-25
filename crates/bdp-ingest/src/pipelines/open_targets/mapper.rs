use anyhow::Result;
use arrow_array::{Array, Float32Array, RecordBatch, StringArray};
use std::collections::HashMap;

/// Normalize Open Targets disease IDs: "MONDO_0005015" → "MONDO:0005015"
pub fn normalize_disease_id(id: &str) -> String {
    if let Some(pos) = id.find('_') {
        let prefix = &id[..pos];
        if prefix.chars().all(|c| c.is_ascii_uppercase()) {
            return format!("{}:{}", prefix, &id[pos + 1..]);
        }
    }
    id.to_string()
}

pub struct AssociationRow {
    pub ensembl_id: String,
    pub disease_id: String, // normalized (colon)
    pub score: f32,
}

/// Extract association rows from a record batch.
pub fn extract_associations(batch: &RecordBatch) -> Result<Vec<AssociationRow>> {
    let target_col = batch
        .column_by_name("targetId")
        .and_then(|c| c.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| anyhow::anyhow!("missing targetId column"))?;

    let disease_col = batch
        .column_by_name("diseaseId")
        .and_then(|c| c.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| anyhow::anyhow!("missing diseaseId column"))?;

    let score_col = batch
        .column_by_name("score")
        .and_then(|c| c.as_any().downcast_ref::<Float32Array>())
        .ok_or_else(|| anyhow::anyhow!("missing score column"))?;

    let mut rows = Vec::with_capacity(batch.num_rows());
    for i in 0..batch.num_rows() {
        if target_col.is_null(i) || disease_col.is_null(i) {
            continue;
        }
        rows.push(AssociationRow {
            ensembl_id: target_col.value(i).to_string(),
            disease_id: normalize_disease_id(disease_col.value(i)),
            score: if score_col.is_null(i) {
                0.0
            } else {
                score_col.value(i)
            },
        });
    }
    Ok(rows)
}

/// Ensembl ID → UniProt accession lookup table.
pub type EnsemblToUniprot = HashMap<String, String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_disease_id() {
        assert_eq!(normalize_disease_id("MONDO_0005015"), "MONDO:0005015");
        assert_eq!(normalize_disease_id("EFO_0000400"), "EFO:0000400");
        assert_eq!(normalize_disease_id("MONDO:0005015"), "MONDO:0005015");
    }
}
