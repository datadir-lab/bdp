// crates/bdp-ingest/src/pipelines/string_db/parser.rs

use anyhow::{bail, Result};

pub struct LinksRow {
    pub protein1: String,
    pub protein2: String,
    pub score_neighborhood: i16,
    pub score_fusion: i16,
    pub score_cooccurrence: i16,
    pub score_coexpression: i16,
    pub score_experimental: i16,
    pub score_database: i16,
    pub score_textmining: i16,
    pub combined_score: i16,
}

pub fn parse_links_row(line: &str) -> Result<LinksRow> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 {
        bail!("invalid links row: {}", line);
    }
    Ok(LinksRow {
        protein1: parts[0].to_string(),
        protein2: parts[1].to_string(),
        score_neighborhood: parts[2].parse()?,
        score_fusion: parts[3].parse()?,
        score_cooccurrence: parts[4].parse()?,
        score_coexpression: parts[5].parse()?,
        score_experimental: parts[6].parse()?,
        score_database: parts[7].parse()?,
        score_textmining: parts[8].parse()?,
        combined_score: parts[9].parse()?,
    })
}

pub fn should_keep(p1: &str, p2: &str) -> bool {
    p1 < p2
}

pub fn parse_alias_row(line: &str) -> Option<(String, String)> {
    let mut cols = line.splitn(3, '\t');
    let ensp = cols.next()?.to_string();
    let alias = cols.next()?.to_string();
    let source = cols.next()?.trim();
    if source == "BLAST_UniProt_AC" {
        Some((ensp, alias))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_links_row() {
        let line = "9606.ENSP00000269696 9606.ENSP00000261509 0 0 0 50 300 0 200 450";
        let row = parse_links_row(line).unwrap();
        assert_eq!(row.protein1, "9606.ENSP00000269696");
        assert_eq!(row.combined_score, 450);
    }

    #[test]
    fn test_deduplicate_keeps_a_lt_b() {
        assert!(should_keep("9606.ENSP00000000001", "9606.ENSP00000000002"));
        assert!(!should_keep("9606.ENSP00000000002", "9606.ENSP00000000001"));
    }

    #[test]
    fn test_parse_alias_row() {
        let line = "9606.ENSP00000269696\tP12345\tBLAST_UniProt_AC";
        let result = parse_alias_row(line);
        assert!(result.is_some());
        let (ensp, uniprot) = result.unwrap();
        assert_eq!(ensp, "9606.ENSP00000269696");
        assert_eq!(uniprot, "P12345");
    }

    #[test]
    fn test_parse_alias_row_wrong_source() {
        let line = "9606.ENSP00000269696\tsome_alias\tOther_source";
        assert!(parse_alias_row(line).is_none());
    }
}
