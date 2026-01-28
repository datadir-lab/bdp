// Gene Ontology Parsers (OBO and GAF formats)

use crate::ingest::gene_ontology::{
    EntityType, EvidenceCode, GoAnnotation, GoRelationship, GoTerm, Namespace, RelationshipType,
    Result, Synonym, SynonymScope,
};
use chrono::NaiveDate;
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

// ============================================================================
// Go Parser (combines OBO and GAF parsers)
// ============================================================================

pub struct GoParser;

impl GoParser {
    /// Parse OBO file
    pub fn parse_obo(
        content: &str,
        go_release_version: &str,
        limit: Option<usize>,
    ) -> Result<ParsedObo> {
        OboParser::parse(content, go_release_version, limit)
    }

    /// Parse GAF file
    pub fn parse_gaf(
        content: &str,
        goa_release_version: &str,
        protein_lookup: &HashMap<String, Uuid>,
        limit: Option<usize>,
    ) -> Result<Vec<GoAnnotation>> {
        GafParser::parse(content, goa_release_version, protein_lookup, limit)
    }
}

// ============================================================================
// OBO Parser (GO terms and relationships)
// ============================================================================

#[derive(Debug)]
pub struct ParsedObo {
    pub terms: Vec<GoTerm>,
    pub relationships: Vec<GoRelationship>,
}

pub struct OboParser;

impl OboParser {
    /// Parse OBO format file
    pub fn parse(
        content: &str,
        go_release_version: &str,
        limit: Option<usize>,
    ) -> Result<ParsedObo> {
        let mut terms = Vec::new();
        let mut relationships = Vec::new();

        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        info!("Starting OBO parsing (limit: {:?})", limit);

        // Skip header until first [Term]
        while i < lines.len() {
            if lines[i].trim() == "[Term]" {
                break;
            }
            i += 1;
        }

        // Parse terms
        while i < lines.len() {
            if let Some(max_terms) = limit {
                if terms.len() >= max_terms {
                    info!("Reached parse limit of {} terms", max_terms);
                    break;
                }
            }

            if lines[i].trim() == "[Term]" {
                match Self::parse_term_stanza(&lines, &mut i, go_release_version) {
                    Ok((term, term_relationships)) => {
                        terms.push(term);
                        relationships.extend(term_relationships);
                    },
                    Err(e) => {
                        warn!("Failed to parse term stanza: {}", e);
                    },
                }
            } else {
                i += 1;
            }
        }

        info!("Parsed {} GO terms and {} relationships", terms.len(), relationships.len());

        Ok(ParsedObo {
            terms,
            relationships,
        })
    }

    /// Parse a single [Term] stanza
    fn parse_term_stanza(
        lines: &[&str],
        i: &mut usize,
        go_release_version: &str,
    ) -> Result<(GoTerm, Vec<GoRelationship>)> {
        *i += 1; // Skip [Term] line

        let mut go_id: Option<String> = None;
        let mut name: Option<String> = None;
        let mut namespace: Option<Namespace> = None;
        let mut definition: Option<String> = None;
        let mut is_obsolete = false;
        let mut synonyms = Vec::new();
        let mut xrefs = Vec::new();
        let mut alt_ids = Vec::new();
        let mut comments = Vec::new();
        let mut relationships = Vec::new();

        // Parse term fields
        while *i < lines.len() {
            let line = lines[*i].trim();

            // End of stanza
            if line.is_empty() || line.starts_with('[') {
                break;
            }

            // Parse field
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "id" => go_id = Some(value.to_string()),
                    "name" => name = Some(value.to_string()),
                    "namespace" => {
                        namespace = Some(
                            Namespace::from_str(value)
                                .map_err(|e| crate::ingest::gene_ontology::GoError::Parse(e))?,
                        );
                    },
                    "def" => {
                        definition = Some(Self::extract_quoted_text(value));
                    },
                    "is_obsolete" => {
                        is_obsolete = value == "true";
                    },
                    "synonym" => {
                        if let Ok(syn) = Self::parse_synonym(value) {
                            synonyms.push(syn);
                        }
                    },
                    "xref" => {
                        xrefs.push(value.to_string());
                    },
                    "alt_id" => {
                        alt_ids.push(value.to_string());
                    },
                    "comment" => {
                        comments.push(value.to_string());
                    },
                    "is_a" => {
                        // Extract GO ID from "GO:0008150 ! biological_process"
                        let Some(parent_id) = value.split_whitespace().next() else {
                            continue;
                        };
                        let Some(ref subject_id) = go_id else {
                            continue;
                        };
                        relationships.push(GoRelationship::new(
                            subject_id.clone(),
                            parent_id.to_string(),
                            RelationshipType::IsA,
                            go_release_version.to_string(),
                        ));
                    },
                    "relationship" => {
                        // Format: "part_of GO:0008150 ! biological_process"
                        let parts: Vec<&str> = value.split_whitespace().collect();
                        if parts.len() < 2 {
                            continue;
                        }
                        let Ok(rel_type) = RelationshipType::from_str(parts[0]) else {
                            continue;
                        };
                        let Some(ref subject_id) = go_id else {
                            continue;
                        };
                        relationships.push(GoRelationship::new(
                            subject_id.clone(),
                            parts[1].to_string(),
                            rel_type,
                            go_release_version.to_string(),
                        ));
                    },
                    _ => {}, // Ignore other fields
                }
            }

            *i += 1;
        }

        // Validate required fields
        let go_id = go_id.ok_or_else(|| {
            crate::ingest::gene_ontology::GoError::Parse("Missing GO ID".to_string())
        })?;

        let name = name.ok_or_else(|| {
            crate::ingest::gene_ontology::GoError::Parse("Missing name".to_string())
        })?;

        let namespace = namespace.ok_or_else(|| {
            crate::ingest::gene_ontology::GoError::Parse("Missing namespace".to_string())
        })?;

        // Create term
        let mut term = GoTerm::new(go_id, name, namespace, go_release_version.to_string())
            .map_err(|e| crate::ingest::gene_ontology::GoError::Parse(e))?;

        term.definition = definition;
        term.is_obsolete = is_obsolete;
        term.synonyms = synonyms;
        term.xrefs = xrefs;
        term.alt_ids = alt_ids;
        term.comments = if comments.is_empty() {
            None
        } else {
            Some(comments.join("; "))
        };

        Ok((term, relationships))
    }

    /// Extract quoted text from definition
    /// Example: "\"biological_process\" [GO:curators]" -> "biological_process"
    fn extract_quoted_text(text: &str) -> String {
        if let Some(start) = text.find('"') {
            if let Some(end) = text[start + 1..].find('"') {
                return text[start + 1..start + 1 + end].to_string();
            }
        }
        text.to_string()
    }

    /// Parse synonym line
    /// Example: "EXACT [GO:curators]" "synonym text"
    fn parse_synonym(text: &str) -> Result<Synonym> {
        // Format: "text" SCOPE [xrefs]
        // Example: "GO:0000003" EXACT []

        let parts: Vec<&str> = text.split('"').collect();
        if parts.len() < 2 {
            return Err(crate::ingest::gene_ontology::GoError::Parse(
                "Invalid synonym format".to_string(),
            ));
        }

        let syn_text = parts[1].to_string();

        // Extract scope (EXACT, BROAD, etc.)
        let remainder = if parts.len() > 2 { parts[2] } else { "" };
        let scope_str = remainder
            .split_whitespace()
            .next()
            .unwrap_or("EXACT")
            .trim();

        let scope = SynonymScope::from_str(scope_str).unwrap_or(SynonymScope::Exact);

        Ok(Synonym {
            scope,
            text: syn_text,
            synonym_type: None,
            xrefs: Vec::new(),
        })
    }
}

// ============================================================================
// GAF Parser (GO annotations)
// ============================================================================

pub struct GafParser;

impl GafParser {
    /// Parse GAF 2.2 format file
    ///
    /// GAF format: Tab-delimited, 17 columns
    /// Column 1: DB (e.g., "UniProtKB")
    /// Column 2: DB Object ID (e.g., "P01308")
    /// Column 3: DB Object Symbol
    /// Column 4: Qualifier
    /// Column 5: GO ID
    /// Column 6: DB:Reference
    /// Column 7: Evidence Code
    /// Column 8: With (or) From
    /// Column 9: Aspect (P/F/C)
    /// Column 10: DB Object Name
    /// Column 11: DB Object Synonym
    /// Column 12: DB Object Type
    /// Column 13: Taxon
    /// Column 14: Date
    /// Column 15: Assigned By
    /// Column 16: Annotation Extension
    /// Column 17: Gene Product Form ID
    pub fn parse(
        content: &str,
        goa_release_version: &str,
        protein_lookup: &HashMap<String, Uuid>,
        limit: Option<usize>,
    ) -> Result<Vec<GoAnnotation>> {
        let mut annotations = Vec::new();
        let mut skipped = 0;
        let mut lines_processed = 0;

        info!("Starting GAF parsing (limit: {:?})", limit);

        for line in content.lines() {
            // Skip comments
            if line.starts_with('!') || line.trim().is_empty() {
                continue;
            }

            lines_processed += 1;

            // Check limit
            if let Some(max_annotations) = limit {
                if annotations.len() >= max_annotations {
                    info!("Reached parse limit of {} annotations", max_annotations);
                    break;
                }
            }

            // Parse line
            match Self::parse_gaf_line(line, goa_release_version, protein_lookup) {
                Ok(Some(annotation)) => {
                    annotations.push(annotation);
                },
                Ok(None) => {
                    skipped += 1;
                },
                Err(e) => {
                    debug!("Failed to parse GAF line: {}", e);
                    skipped += 1;
                },
            }

            // Progress logging
            if lines_processed % 1_000_000 == 0 {
                info!(
                    "Processed {} lines, {} annotations, {} skipped",
                    lines_processed,
                    annotations.len(),
                    skipped
                );
            }
        }

        info!(
            "Parsed {} annotations from {} lines ({} skipped)",
            annotations.len(),
            lines_processed,
            skipped
        );

        Ok(annotations)
    }

    /// Parse a single GAF line
    fn parse_gaf_line(
        line: &str,
        goa_release_version: &str,
        protein_lookup: &HashMap<String, Uuid>,
    ) -> Result<Option<GoAnnotation>> {
        let columns: Vec<&str> = line.split('\t').collect();

        if columns.len() < 15 {
            return Err(crate::ingest::gene_ontology::GoError::Parse(format!(
                "Invalid GAF line: expected 15+ columns, got {}",
                columns.len()
            )));
        }

        let db = columns[0].trim();
        let db_object_id = columns[1].trim(); // UniProt accession
        let qualifier = columns[3].trim();
        let go_id = columns[4].trim();
        let reference = columns[5].trim();
        let evidence_code = columns[6].trim();
        let with_from = columns[7].trim();
        let taxon_str = columns[12].trim();
        let date_str = columns[13].trim();
        let assigned_by = columns[14].trim();

        // Optional columns (GAF 2.1+)
        let annotation_extension = if columns.len() > 15 {
            columns[15].trim()
        } else {
            ""
        };
        let gene_product_form_id = if columns.len() > 16 {
            columns[16].trim()
        } else {
            ""
        };

        // Lookup protein entity_id
        let entity_id = match protein_lookup.get(db_object_id) {
            Some(id) => *id,
            None => {
                // Skip if protein not found
                return Ok(None);
            },
        };

        // Parse taxonomy ID (format: "taxon:9606")
        let taxonomy_id = Self::parse_taxonomy_id(taxon_str);

        // Parse date (format: "YYYYMMDD")
        let annotation_date = Self::parse_date(date_str);

        // Parse annotation extension (JSON if present)
        let annotation_extension_json = if annotation_extension.is_empty() {
            None
        } else {
            Some(serde_json::json!({"text": annotation_extension}))
        };

        Ok(Some(GoAnnotation {
            entity_type: EntityType::Protein,
            entity_id,
            go_id: go_id.to_string(),
            evidence_code: EvidenceCode::new(evidence_code.to_string()),
            qualifier: if qualifier.is_empty() {
                None
            } else {
                Some(qualifier.to_string())
            },
            reference: if reference.is_empty() {
                None
            } else {
                Some(reference.to_string())
            },
            with_from: if with_from.is_empty() {
                None
            } else {
                Some(with_from.to_string())
            },
            annotation_source: Some(db.to_string()),
            assigned_by: Some(assigned_by.to_string()),
            annotation_date,
            taxonomy_id,
            annotation_extension: annotation_extension_json,
            gene_product_form_id: if gene_product_form_id.is_empty() {
                None
            } else {
                Some(gene_product_form_id.to_string())
            },
            goa_release_version: goa_release_version.to_string(),
        }))
    }

    /// Parse taxonomy ID from "taxon:9606" format
    fn parse_taxonomy_id(taxon_str: &str) -> Option<i64> {
        taxon_str
            .strip_prefix("taxon:")
            .and_then(|s| s.split('|').next())
            .and_then(|s| s.parse::<i64>().ok())
    }

    /// Parse date from "YYYYMMDD" format
    fn parse_date(date_str: &str) -> Option<NaiveDate> {
        if date_str.len() != 8 {
            return None;
        }

        let year = date_str[0..4].parse::<i32>().ok()?;
        let month = date_str[4..6].parse::<u32>().ok()?;
        let day = date_str[6..8].parse::<u32>().ok()?;

        NaiveDate::from_ymd_opt(year, month, day)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_obo_header() {
        let obo_content = r#"format-version: 1.2
data-version: releases/2026-01-01

[Term]
id: GO:0008150
name: biological_process
namespace: biological_process
def: "A biological process represents a specific objective." [GOC:jl]

[Term]
id: GO:0003674
name: molecular_function
namespace: molecular_function
"#;

        let result = OboParser::parse(obo_content, "2026-01-01", None);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.terms.len(), 2);
        assert_eq!(parsed.terms[0].go_id, "GO:0008150");
        assert_eq!(parsed.terms[0].name, "biological_process");
        assert_eq!(parsed.terms[0].namespace, Namespace::BiologicalProcess);
    }

    #[test]
    fn test_parse_obo_with_relationships() {
        let obo_content = r#"
[Term]
id: GO:0006955
name: immune response
namespace: biological_process
is_a: GO:0008150 ! biological_process
relationship: part_of GO:0002376 ! immune system process
"#;

        let result = OboParser::parse(obo_content, "2026-01-01", None);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.terms.len(), 1);
        assert_eq!(parsed.relationships.len(), 2);
        assert_eq!(parsed.relationships[0].subject_go_id, "GO:0006955");
        assert_eq!(parsed.relationships[0].object_go_id, "GO:0008150");
        assert_eq!(parsed.relationships[0].relationship_type, RelationshipType::IsA);
    }

    #[test]
    fn test_parse_gaf_line() {
        let protein_id = Uuid::new_v4();
        let mut protein_lookup = HashMap::new();
        protein_lookup.insert("P01308".to_string(), protein_id);

        let gaf_line = "UniProtKB\tP01308\tINS\t\tGO:0006955\tPMID:12345678\tIDA\t\tP\tinsulin\t\tprotein\ttaxon:9606\t20260115\tUniProt";

        let result = GafParser::parse_gaf_line(gaf_line, "2026-01-15", &protein_lookup);
        assert!(result.is_ok());

        let annotation = result.unwrap().unwrap();
        assert_eq!(annotation.entity_id, protein_id);
        assert_eq!(annotation.go_id, "GO:0006955");
        assert_eq!(annotation.evidence_code.0, "IDA");
        assert_eq!(annotation.taxonomy_id, Some(9606));
    }

    #[test]
    fn test_parse_taxonomy_id() {
        assert_eq!(GafParser::parse_taxonomy_id("taxon:9606"), Some(9606));
        assert_eq!(GafParser::parse_taxonomy_id("taxon:9606|taxon:10090"), Some(9606));
        assert_eq!(GafParser::parse_taxonomy_id("invalid"), None);
    }

    #[test]
    fn test_parse_date() {
        assert_eq!(GafParser::parse_date("20260115"), NaiveDate::from_ymd_opt(2026, 1, 15));
        assert_eq!(GafParser::parse_date("invalid"), None);
        assert_eq!(GafParser::parse_date("2026"), None);
    }

    #[test]
    fn test_extract_quoted_text() {
        assert_eq!(
            OboParser::extract_quoted_text("\"biological_process\" [GO:curators]"),
            "biological_process"
        );
        assert_eq!(OboParser::extract_quoted_text("no quotes"), "no quotes");
    }
}
