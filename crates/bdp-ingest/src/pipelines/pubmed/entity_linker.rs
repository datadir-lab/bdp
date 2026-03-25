// crates/bdp-ingest/src/pipelines/pubmed/entity_linker.rs

use anyhow::Result;

#[derive(Debug)]
pub struct PubTatorEntry {
    pub pmid: i32,
    pub entity_type: String, // "Gene", "Disease", "Chemical"
    pub concept_id: String,
    pub name: Option<String>,
}

/// Parse a single PubTator3 line: pmid|type|concept_id|name|mentions
pub fn parse_pubtator_line(line: &str) -> Option<PubTatorEntry> {
    if line.starts_with('#') || line.trim().is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.splitn(5, '|').collect();
    if parts.len() < 3 {
        return None;
    }
    Some(PubTatorEntry {
        pmid: parts[0].trim().parse().ok()?,
        entity_type: parts[1].to_string(),
        concept_id: parts[2].to_string(),
        name: parts.get(3).map(|s| s.to_string()),
    })
}

/// Normalize concept IDs to BDP format.
pub fn normalize_entity_id(entity_type: &str, concept_id: &str) -> String {
    match entity_type {
        "Disease" | "Chemical" => {
            if concept_id.contains(':') {
                concept_id.to_string()
            } else if concept_id.starts_with('D') || concept_id.starts_with('C') {
                format!("MESH:{}", concept_id)
            } else {
                concept_id.to_string()
            }
        }
        _ => concept_id.to_string(),
    }
}

/// Fetch PubTator3 annotations for a batch of PMIDs.
/// Returns raw text lines from the biocjson export endpoint (pubtator format).
pub async fn fetch_pubtator_annotations(
    client: &reqwest::Client,
    pmids: &[i32],
) -> Result<Vec<PubTatorEntry>> {
    if pmids.is_empty() {
        return Ok(Vec::new());
    }
    let pmid_str: Vec<String> = pmids.iter().map(|p| p.to_string()).collect();
    let url = format!(
        "https://www.ncbi.nlm.nih.gov/research/pubtator3-api/publications/export/pubtator?pmids={}",
        pmid_str.join(",")
    );
    let resp = client.get(&url).send().await?.error_for_status()?;
    let text = resp.text().await?;
    let entries = text
        .lines()
        .filter_map(parse_pubtator_line)
        .collect();
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pubtator_line() {
        let line = "12345678|Gene|7157|TP53|TP53;p53";
        let entry = parse_pubtator_line(line).unwrap();
        assert_eq!(entry.pmid, 12345678);
        assert_eq!(entry.entity_type, "Gene");
        assert_eq!(entry.concept_id, "7157");
        assert_eq!(entry.name.as_deref(), Some("TP53"));
    }

    #[test]
    fn test_parse_pubtator_comment_line() {
        assert!(parse_pubtator_line("# this is a comment").is_none());
        assert!(parse_pubtator_line("").is_none());
        assert!(parse_pubtator_line("   ").is_none());
    }

    #[test]
    fn test_parse_pubtator_too_few_fields() {
        assert!(parse_pubtator_line("12345678|Gene").is_none());
    }

    #[test]
    fn test_normalize_mesh_disease() {
        assert_eq!(normalize_entity_id("Disease", "D009369"), "MESH:D009369");
        assert_eq!(
            normalize_entity_id("Disease", "MONDO:0005015"),
            "MONDO:0005015"
        );
    }

    #[test]
    fn test_normalize_mesh_chemical() {
        assert_eq!(normalize_entity_id("Chemical", "C000657245"), "MESH:C000657245");
    }

    #[test]
    fn test_normalize_gene_passthrough() {
        assert_eq!(normalize_entity_id("Gene", "7157"), "7157");
    }
}
