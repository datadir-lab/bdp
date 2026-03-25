use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AactStudyRow {
    pub nct_id: String,
    pub brief_title: Option<String>,
    pub overall_status: Option<String>,
    pub phase: Option<String>,
    pub start_date: Option<String>,
    pub completion_date: Option<String>,
    pub source: Option<String>,
}

pub fn parse_studies_csv(content: &str) -> Result<Vec<AactStudyRow>> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    let mut rows = Vec::new();
    for result in rdr.deserialize() {
        match result {
            Ok(row) => rows.push(row),
            Err(e) => tracing::warn!("AACT CSV parse error (row skipped): {}", e),
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_studies_csv_minimal() {
        let csv = "nct_id|brief_title|overall_status|phase|start_date|completion_date|source\n\
                   NCT00000001|Test Study|Completed|Phase 2|2020-01-01|2022-06-30|Sponsor\n";
        let rows = parse_studies_csv(csv).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].nct_id, "NCT00000001");
        assert_eq!(rows[0].phase.as_deref(), Some("Phase 2"));
    }
}
