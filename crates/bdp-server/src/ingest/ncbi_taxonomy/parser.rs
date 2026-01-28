//! NCBI Taxonomy taxdump parser
//!
//! Parses NCBI Taxonomy dump files in .dmp format:
//! - rankedlineage.dmp: Main taxonomy data with full lineage
//! - merged.dmp: Merged taxonomy IDs (old → new)
//! - delnodes.dmp: Deleted taxonomy IDs
//!
//! # File Format
//! The .dmp files use a tab-delimited format with pipe separators: `\t|\t`
//! Lines end with `\t|` and newline.

use anyhow::{Context, Result};
use tracing::{debug, warn};

use super::models::{DeletedTaxon, MergedTaxon, TaxdumpData, TaxonomyEntry};

/// Parser for NCBI Taxonomy taxdump files
pub struct TaxdumpParser {
    /// Maximum number of entries to parse (None for unlimited)
    parse_limit: Option<usize>,
}

impl TaxdumpParser {
    /// Create a new parser with no limit
    pub fn new() -> Self {
        Self { parse_limit: None }
    }

    /// Create a parser with a limit
    pub fn with_limit(limit: usize) -> Self {
        Self {
            parse_limit: Some(limit),
        }
    }

    /// Parse all taxdump files into TaxdumpData
    ///
    /// # Arguments
    /// - rankedlineage_content: Contents of rankedlineage.dmp
    /// - merged_content: Contents of merged.dmp
    /// - delnodes_content: Contents of delnodes.dmp
    /// - external_version: FTP timestamp (e.g., "2026-01-15")
    pub fn parse(
        &self,
        rankedlineage_content: &str,
        merged_content: &str,
        delnodes_content: &str,
        external_version: String,
    ) -> Result<TaxdumpData> {
        debug!("Parsing rankedlineage.dmp");
        let entries = self.parse_rankedlineage(rankedlineage_content)?;
        debug!("Parsed {} taxonomy entries", entries.len());

        debug!("Parsing merged.dmp");
        let merged = self.parse_merged(merged_content)?;
        debug!("Parsed {} merged taxa", merged.len());

        debug!("Parsing delnodes.dmp");
        let deleted = self.parse_delnodes(delnodes_content)?;
        debug!("Parsed {} deleted taxa", deleted.len());

        Ok(TaxdumpData::new(entries, merged, deleted, external_version))
    }

    /// Parse rankedlineage.dmp file
    ///
    /// # Format
    /// ```text
    /// tax_id | tax_name | species | genus | family | order | class | phylum | kingdom | superkingdom |
    /// 9606 | Homo sapiens | Homo sapiens | Homo | Hominidae | Primates | Mammalia | Chordata | Metazoa | Eukaryota |
    /// ```
    ///
    /// Fields are separated by `\t|\t` and lines end with `\t|`
    pub fn parse_rankedlineage(&self, content: &str) -> Result<Vec<TaxonomyEntry>> {
        let mut entries = Vec::new();
        let mut line_num = 0;

        for line in content.lines() {
            line_num += 1;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse entry
            match self.parse_rankedlineage_line(line, line_num) {
                Ok(Some(entry)) => {
                    entries.push(entry);

                    // Check parse limit
                    if let Some(limit) = self.parse_limit {
                        if entries.len() >= limit {
                            debug!("Reached parse limit of {} entries", limit);
                            break;
                        }
                    }
                },
                Ok(None) => {
                    // Skipped entry (e.g., invalid rank)
                },
                Err(e) => {
                    warn!("Failed to parse line {}: {} - Error: {}", line_num, line, e);
                    // Continue parsing other entries
                },
            }
        }

        Ok(entries)
    }

    /// Parse a single line from rankedlineage.dmp
    pub fn parse_rankedlineage_line(
        &self,
        line: &str,
        line_num: usize,
    ) -> Result<Option<TaxonomyEntry>> {
        // Split by \t|\t separator
        let fields: Vec<&str> = line
            .split("\t|\t")
            .map(|f| f.trim().trim_end_matches('|').trim())
            .collect();

        // We need at least 10 fields (tax_id through superkingdom)
        if fields.len() < 10 {
            return Err(anyhow::anyhow!(
                "Line {}: Expected at least 10 fields, got {}",
                line_num,
                fields.len()
            ));
        }

        // Parse fields
        let taxonomy_id: i32 = fields[0]
            .parse()
            .with_context(|| format!("Line {}: Invalid taxonomy_id: {}", line_num, fields[0]))?;

        let scientific_name = fields[1].to_string();

        // Determine rank based on which field has a value
        // Fields: species, genus, family, order, class, phylum, kingdom, superkingdom
        let (rank, common_name) = if !fields[2].is_empty() {
            ("species", Some(fields[2].to_string()))
        } else if !fields[3].is_empty() {
            ("genus", None)
        } else if !fields[4].is_empty() {
            ("family", None)
        } else if !fields[5].is_empty() {
            ("order", None)
        } else if !fields[6].is_empty() {
            ("class", None)
        } else if !fields[7].is_empty() {
            ("phylum", None)
        } else if !fields[8].is_empty() {
            ("kingdom", None)
        } else if !fields[9].is_empty() {
            ("superkingdom", None)
        } else {
            // No rank specified, use "no rank"
            ("no rank", None)
        };

        // Build lineage from bottom to top (superkingdom → species)
        let mut lineage_parts = Vec::new();

        // Add non-empty ranks in order
        for i in (2..=9).rev() {
            if !fields[i].is_empty() && fields[i] != scientific_name {
                lineage_parts.push(fields[i].to_string());
            }
        }

        // Add scientific name at the end
        lineage_parts.push(scientific_name.clone());

        let lineage = lineage_parts.join(";");

        Ok(Some(TaxonomyEntry::new(
            taxonomy_id,
            scientific_name,
            common_name,
            rank.to_string(),
            lineage,
        )))
    }

    /// Parse merged.dmp file
    ///
    /// # Format
    /// ```text
    /// old_tax_id | new_tax_id |
    /// 123 | 456 |
    /// ```
    fn parse_merged(&self, content: &str) -> Result<Vec<MergedTaxon>> {
        let mut merged = Vec::new();
        let mut line_num = 0;

        for line in content.lines() {
            line_num += 1;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse entry
            match self.parse_merged_line(line, line_num) {
                Ok(taxon) => merged.push(taxon),
                Err(e) => {
                    warn!("Failed to parse merged line {}: {} - Error: {}", line_num, line, e);
                    // Continue parsing other entries
                },
            }
        }

        Ok(merged)
    }

    /// Parse a single line from merged.dmp
    pub fn parse_merged_line(&self, line: &str, line_num: usize) -> Result<MergedTaxon> {
        // Split by \t|\t separator
        let fields: Vec<&str> = line.split("\t|\t").map(|f| f.trim()).collect();

        // We need at least 2 fields (old_tax_id, new_tax_id)
        if fields.len() < 2 {
            return Err(anyhow::anyhow!(
                "Line {}: Expected at least 2 fields, got {}",
                line_num,
                fields.len()
            ));
        }

        let old_taxonomy_id: i32 = fields[0]
            .trim_end_matches('|')
            .trim()
            .parse()
            .with_context(|| {
                format!("Line {}: Invalid old_taxonomy_id: {}", line_num, fields[0])
            })?;

        let new_taxonomy_id: i32 = fields[1]
            .trim_end_matches('|')
            .trim()
            .parse()
            .with_context(|| {
                format!("Line {}: Invalid new_taxonomy_id: {}", line_num, fields[1])
            })?;

        Ok(MergedTaxon::new(old_taxonomy_id, new_taxonomy_id))
    }

    /// Parse delnodes.dmp file
    ///
    /// # Format
    /// ```text
    /// tax_id |
    /// 789 |
    /// ```
    fn parse_delnodes(&self, content: &str) -> Result<Vec<DeletedTaxon>> {
        let mut deleted = Vec::new();
        let mut line_num = 0;

        for line in content.lines() {
            line_num += 1;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse entry
            match self.parse_delnodes_line(line, line_num) {
                Ok(taxon) => deleted.push(taxon),
                Err(e) => {
                    warn!("Failed to parse delnodes line {}: {} - Error: {}", line_num, line, e);
                    // Continue parsing other entries
                },
            }
        }

        Ok(deleted)
    }

    /// Parse a single line from delnodes.dmp
    pub fn parse_delnodes_line(&self, line: &str, line_num: usize) -> Result<DeletedTaxon> {
        // Split by \t|\t or \t| separator
        let tax_id_str = line
            .split('\t')
            .next()
            .context("Line is empty")?
            .trim()
            .trim_end_matches('|')
            .trim();

        let taxonomy_id: i32 = tax_id_str
            .parse()
            .with_context(|| format!("Line {}: Invalid taxonomy_id: {}", line_num, tax_id_str))?;

        Ok(DeletedTaxon::new(taxonomy_id))
    }
}

impl Default for TaxdumpParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rankedlineage_line() {
        let parser = TaxdumpParser::new();

        // Sample line from rankedlineage.dmp
        let line = "9606\t|\tHomo sapiens\t|\tHomo sapiens\t|\tHomo\t|\tHominidae\t|\tPrimates\t|\tMammalia\t|\tChordata\t|\tMetazoa\t|\tEukaryota\t|";

        let result = parser.parse_rankedlineage_line(line, 1).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.taxonomy_id, 9606);
        assert_eq!(entry.scientific_name, "Homo sapiens");
        assert_eq!(entry.common_name, Some("Homo sapiens".to_string()));
        assert_eq!(entry.rank, "species");
        assert!(entry.lineage.contains("Eukaryota"));
        assert!(entry.lineage.contains("Homo sapiens"));
    }

    #[test]
    fn test_parse_merged_line() {
        let parser = TaxdumpParser::new();

        // Sample line from merged.dmp
        let line = "123\t|\t456\t|";

        let merged = parser.parse_merged_line(line, 1).unwrap();
        assert_eq!(merged.old_taxonomy_id, 123);
        assert_eq!(merged.new_taxonomy_id, 456);
    }

    #[test]
    fn test_parse_delnodes_line() {
        let parser = TaxdumpParser::new();

        // Sample line from delnodes.dmp
        let line = "789\t|";

        let deleted = parser.parse_delnodes_line(line, 1).unwrap();
        assert_eq!(deleted.taxonomy_id, 789);
    }

    #[test]
    fn test_parse_with_limit() {
        let parser = TaxdumpParser::with_limit(2);

        let rankedlineage = "9606\t|\tHomo sapiens\t|\tHomo sapiens\t|\tHomo\t|\tHominidae\t|\tPrimates\t|\tMammalia\t|\tChordata\t|\tMetazoa\t|\tEukaryota\t|\n\
                            10090\t|\tMus musculus\t|\tMus musculus\t|\tMus\t|\tMuridae\t|\tRodentia\t|\tMammalia\t|\tChordata\t|\tMetazoa\t|\tEukaryota\t|\n\
                            7227\t|\tDrosophila melanogaster\t|\tDrosophila melanogaster\t|\tDrosophila\t|\tDrosophilidae\t|\tDiptera\t|\tInsecta\t|\tArthropoda\t|\tMetazoa\t|\tEukaryota\t|";

        let entries = parser.parse_rankedlineage(rankedlineage).unwrap();
        assert_eq!(entries.len(), 2); // Should stop at limit
    }

    #[test]
    fn test_parse_full_taxdump() {
        let parser = TaxdumpParser::new();

        let rankedlineage = "9606\t|\tHomo sapiens\t|\tHomo sapiens\t|\tHomo\t|\tHominidae\t|\tPrimates\t|\tMammalia\t|\tChordata\t|\tMetazoa\t|\tEukaryota\t|";
        let merged = "123\t|\t456\t|";
        let delnodes = "789\t|";

        let taxdump = parser
            .parse(rankedlineage, merged, delnodes, "2026-01-15".to_string())
            .unwrap();

        assert_eq!(taxdump.entries.len(), 1);
        assert_eq!(taxdump.merged.len(), 1);
        assert_eq!(taxdump.deleted.len(), 1);
        assert_eq!(taxdump.external_version, "2026-01-15");
    }
}
