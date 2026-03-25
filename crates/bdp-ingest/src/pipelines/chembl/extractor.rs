use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;

pub struct ActivityRow {
    pub inchikey: String,
    pub target_chembl_id: String,
    pub activity_type: Option<String>,
    pub activity_value: Option<f64>,
    pub activity_unit: Option<String>,
    pub relation: Option<String>,
    pub assay_type: Option<String>,
    pub assay_chembl_id: Option<String>,
    pub doc_id: Option<String>,
    pub confidence: Option<i64>,
}

pub fn extract_activities(conn: &Connection, limit: Option<usize>) -> Result<Vec<ActivityRow>> {
    let limit_clause = limit.map(|n| format!("LIMIT {n}")).unwrap_or_default();
    let sql = format!(
        r#"SELECT cs.standard_inchi_key, td.chembl_id,
                  act.standard_type, act.standard_value, act.standard_units,
                  act.standard_relation, a.assay_type, a.chembl_id as assay_cid,
                  CAST(act.doc_id AS TEXT), a.confidence_score
           FROM activities act
           JOIN compound_structures cs ON cs.molregno = act.molregno
           JOIN assays a ON a.assay_id = act.assay_id
           JOIN target_dictionary td ON td.tid = a.tid
           WHERE act.standard_value IS NOT NULL
             AND cs.standard_inchi_key IS NOT NULL
             AND td.target_type = 'SINGLE PROTEIN'
           {limit_clause}"#
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(ActivityRow {
            inchikey: row.get(0)?,
            target_chembl_id: row.get(1)?,
            activity_type: row.get(2)?,
            activity_value: row.get(3)?,
            activity_unit: row.get(4)?,
            relation: row.get(5)?,
            assay_type: row.get(6)?,
            assay_chembl_id: row.get(7)?,
            doc_id: row.get(8)?,
            confidence: row.get(9)?,
        })
    })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn parse_uniprot_mapping(content: &str) -> HashMap<String, String> {
    content
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .filter_map(|l| {
            let mut cols = l.splitn(4, '\t');
            let chembl_id = cols.next()?.to_string();
            let uniprot_ac = cols.next()?.to_string();
            Some((chembl_id, uniprot_ac))
        })
        .collect()
}

pub fn is_valid_inchikey(key: &str) -> bool {
    let parts: Vec<&str> = key.split('-').collect();
    parts.len() == 3
        && parts[0].len() == 14
        && parts[1].len() == 10
        && parts[2].len() == 1
        && parts
            .iter()
            .all(|p| p.chars().all(|c| c.is_ascii_uppercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inchikey_validate() {
        assert!(is_valid_inchikey("BQJCRHHNABKAKU-KBQPJGBKSA-N"));
        assert!(!is_valid_inchikey("not-a-key"));
        assert!(!is_valid_inchikey(""));
    }

    #[test]
    fn test_parse_uniprot_mapping() {
        let content = "CHEMBL612545\tP00519\tABL1\tSINGLE PROTEIN\n";
        let map = parse_uniprot_mapping(content);
        assert_eq!(map.get("CHEMBL612545").map(|s| s.as_str()), Some("P00519"));
    }
}
