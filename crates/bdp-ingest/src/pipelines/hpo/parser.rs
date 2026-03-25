// HPO Parsers — OBO format (hp.obo) and HPOA TSV (phenotype.hpoa)

use crate::pipelines::hpo::models::{
    DiseaseAnnotation, HpoRelationship, HpoSynonym, HpoTerm, SynonymScope,
};
use anyhow::{anyhow, Result};
use tracing::{debug, info, warn};

// ============================================================================
// Combined output from parsing hp.obo
// ============================================================================

#[derive(Debug)]
pub struct ParsedHpo {
    pub terms: Vec<HpoTerm>,
    pub relationships: Vec<HpoRelationship>,
}

// ============================================================================
// Main HPO Parser (OBO + HPOA)
// ============================================================================

pub struct HpoParser;

impl HpoParser {
    /// Parse hp.obo content into HPO terms and relationships
    pub fn parse_obo(
        content: &str,
        release_version: &str,
        limit: Option<usize>,
    ) -> Result<ParsedHpo> {
        OboParser::parse(content, release_version, limit)
    }

    /// Parse phenotype.hpoa TSV content into disease annotations
    pub fn parse_hpoa(
        content: &str,
        release_version: &str,
        limit: Option<usize>,
    ) -> Result<Vec<DiseaseAnnotation>> {
        HpoaParser::parse(content, release_version, limit)
    }
}

// ============================================================================
// OBO Parser (hp.obo)
// ============================================================================

pub struct OboParser;

impl OboParser {
    /// Parse OBO format hp.obo file
    pub fn parse(content: &str, release_version: &str, limit: Option<usize>) -> Result<ParsedHpo> {
        let mut terms = Vec::new();
        let mut relationships = Vec::new();

        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        info!("Starting HPO OBO parsing (limit: {:?})", limit);

        // Skip header until first [Term]
        while i < lines.len() {
            if lines[i].trim() == "[Term]" {
                break;
            }
            i += 1;
        }

        // Parse [Term] stanzas
        while i < lines.len() {
            if let Some(max) = limit {
                if terms.len() >= max {
                    info!("Reached parse limit of {} terms", max);
                    break;
                }
            }

            if lines[i].trim() == "[Term]" {
                match Self::parse_term_stanza(&lines, &mut i, release_version) {
                    Ok((term, term_rels)) => {
                        terms.push(term);
                        relationships.extend(term_rels);
                    }
                    Err(e) => {
                        warn!("Failed to parse HPO term stanza at line {}: {}", i, e);
                        // Advance past the stanza to avoid infinite loop
                        i += 1;
                    }
                }
            } else {
                i += 1;
            }
        }

        info!(
            "Parsed {} HPO terms and {} relationships",
            terms.len(),
            relationships.len()
        );

        Ok(ParsedHpo {
            terms,
            relationships,
        })
    }

    /// Parse a single [Term] stanza from lines[i..]
    fn parse_term_stanza(
        lines: &[&str],
        i: &mut usize,
        release_version: &str,
    ) -> Result<(HpoTerm, Vec<HpoRelationship>)> {
        *i += 1; // skip [Term] line

        let mut hpo_id: Option<String> = None;
        let mut name: Option<String> = None;
        let mut definition: Option<String> = None;
        let mut comment: Option<String> = None;
        let mut is_obsolete = false;
        let mut replaced_by: Option<String> = None;
        let mut synonyms: Vec<HpoSynonym> = Vec::new();
        let mut xrefs: Vec<String> = Vec::new();
        let mut alt_ids: Vec<String> = Vec::new();
        let mut subset: Vec<String> = Vec::new();
        let mut relationships: Vec<HpoRelationship> = Vec::new();

        while *i < lines.len() {
            let line = lines[*i].trim();

            // End of stanza
            if line.is_empty() || line.starts_with('[') {
                break;
            }

            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                // Value may have trailing space; we re-split for multi-part fields
                let value = value.trim();

                match key {
                    "id" => hpo_id = Some(value.to_string()),
                    "name" => name = Some(value.to_string()),
                    "def" => {
                        definition = Some(extract_quoted_text(value));
                    }
                    "comment" => comment = Some(value.to_string()),
                    "is_obsolete" => is_obsolete = value == "true",
                    "replaced_by" => replaced_by = Some(value.to_string()),
                    "synonym" => {
                        if let Ok(syn) = parse_synonym(value) {
                            synonyms.push(syn);
                        }
                    }
                    "xref" => {
                        xrefs.push(value.to_string());
                    }
                    "alt_id" => {
                        alt_ids.push(value.to_string());
                    }
                    "subset" => {
                        subset.push(value.to_string());
                    }
                    "is_a" => {
                        // "HP:0000001 ! All"  → take first token
                        if let Some(parent_id) = value.split_whitespace().next() {
                            if let Some(ref subject_id) = hpo_id {
                                relationships.push(HpoRelationship {
                                    subject_hpo_id: subject_id.clone(),
                                    object_hpo_id: parent_id.to_string(),
                                    relationship_type: "is_a".to_string(),
                                    hpo_release_version: release_version.to_string(),
                                });
                            }
                        }
                    }
                    "relationship" => {
                        // "part_of HP:0000001 ! All"
                        let parts: Vec<&str> = value.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Some(ref subject_id) = hpo_id {
                                relationships.push(HpoRelationship {
                                    subject_hpo_id: subject_id.clone(),
                                    object_hpo_id: parts[1].to_string(),
                                    relationship_type: parts[0].to_string(),
                                    hpo_release_version: release_version.to_string(),
                                });
                            }
                        }
                    }
                    _ => {} // Ignore unrecognised fields
                }
            }

            *i += 1;
        }

        let hpo_id =
            hpo_id.ok_or_else(|| anyhow!("Missing HPO ID in [Term] stanza"))?;

        let name =
            name.ok_or_else(|| anyhow!("Missing name for term {}", hpo_id))?;

        // Build term (skip ID validation for alt/obsolete terms that may not match pattern)
        let hpo_accession = HpoTerm::parse_accession(&hpo_id).unwrap_or(0);

        let term = HpoTerm {
            hpo_id,
            hpo_accession,
            name,
            definition,
            comment,
            is_obsolete,
            replaced_by,
            synonyms,
            xrefs,
            alt_ids,
            subset,
            hpo_release_version: release_version.to_string(),
        };

        debug!("Parsed HPO term: {} - {}", term.hpo_id, term.name);

        Ok((term, relationships))
    }
}

// ============================================================================
// HPOA TSV Parser (phenotype.hpoa)
// ============================================================================
// Column order (tab-separated):
//   0: database_id   (e.g., "OMIM:114500")
//   1: disease_name
//   2: qualifier     (e.g., "NOT", empty)
//   3: hpo_id        (e.g., "HP:0000001")
//   4: reference
//   5: evidence
//   6: onset
//   7: frequency
//   8: sex
//   9: modifier
//  10: aspect
//  11: biocuration

pub struct HpoaParser;

impl HpoaParser {
    /// Parse phenotype.hpoa TSV content
    pub fn parse(
        content: &str,
        release_version: &str,
        limit: Option<usize>,
    ) -> Result<Vec<DiseaseAnnotation>> {
        let mut annotations = Vec::new();
        let mut lines_processed = 0usize;
        let mut skipped = 0usize;

        info!("Starting HPOA TSV parsing (limit: {:?})", limit);

        for line in content.lines() {
            // Skip comment / header lines (start with '#')
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }

            lines_processed += 1;

            if let Some(max) = limit {
                if annotations.len() >= max {
                    info!("Reached parse limit of {} annotations", max);
                    break;
                }
            }

            match Self::parse_hpoa_line(line, release_version) {
                Ok(annotation) => annotations.push(annotation),
                Err(e) => {
                    debug!("Failed to parse HPOA line {}: {}", lines_processed, e);
                    skipped += 1;
                }
            }

            if lines_processed % 100_000 == 0 {
                info!(
                    "HPOA: processed {} lines, {} annotations, {} skipped",
                    lines_processed,
                    annotations.len(),
                    skipped
                );
            }
        }

        info!(
            "Parsed {} disease-phenotype annotations ({} skipped)",
            annotations.len(),
            skipped
        );

        Ok(annotations)
    }

    fn parse_hpoa_line(line: &str, release_version: &str) -> Result<DiseaseAnnotation> {
        let cols: Vec<&str> = line.split('\t').collect();

        if cols.len() < 4 {
            return Err(anyhow!(
                "Expected ≥4 columns, got {} in line: {}",
                cols.len(),
                &line[..line.len().min(80)]
            ));
        }

        let database_id = cols[0].trim();
        let disease_name = cols[1].trim();
        let qualifier = cols[2].trim();
        let hpo_id = cols[3].trim();

        let (disease_db, disease_id) = DiseaseAnnotation::parse_database_id(database_id)
            .map_err(|e| anyhow!("{}", e))?;

        Ok(DiseaseAnnotation {
            disease_db,
            disease_id,
            disease_name: disease_name.to_string(),
            hpo_id: hpo_id.to_string(),
            qualifier: opt_str(qualifier),
            reference: cols.get(4).copied().map(str::trim).and_then(opt_str),
            evidence: cols.get(5).copied().map(str::trim).and_then(opt_str),
            onset: cols.get(6).copied().map(str::trim).and_then(opt_str),
            frequency: cols.get(7).copied().map(str::trim).and_then(opt_str),
            sex: cols.get(8).copied().map(str::trim).and_then(opt_str),
            modifier: cols.get(9).copied().map(str::trim).and_then(opt_str),
            aspect: cols.get(10).copied().map(str::trim).and_then(opt_str),
            biocuration: cols.get(11).copied().map(str::trim).and_then(opt_str),
            hpo_release_version: release_version.to_string(),
        })
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Return None for empty strings, Some otherwise
fn opt_str(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Extract first quoted string from OBO def/synonym field
/// Example: `"Abnormality of the integument" [HPO:probinson]` → `Abnormality of the integument`
fn extract_quoted_text(text: &str) -> String {
    if let Some(start) = text.find('"') {
        if let Some(end) = text[start + 1..].find('"') {
            return text[start + 1..start + 1 + end].to_string();
        }
    }
    text.to_string()
}

/// Parse synonym line: `"Skin abnormality" EXACT []`
fn parse_synonym(text: &str) -> std::result::Result<HpoSynonym, String> {
    let parts: Vec<&str> = text.split('"').collect();
    if parts.len() < 2 {
        return Err("Invalid synonym format".to_string());
    }

    let syn_text = parts[1].to_string();
    let remainder = if parts.len() > 2 { parts[2] } else { "" };
    let scope_str = remainder.split_whitespace().next().unwrap_or("EXACT").trim();
    let scope = SynonymScope::from_str(scope_str).unwrap_or(SynonymScope::Exact);

    Ok(HpoSynonym { scope, text: syn_text })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_OBO: &str = r#"format-version: 1.2
data-version: hp/releases/2026-03-01

[Term]
id: HP:0000001
name: All
comment: Root of all HPO terms.
is_a: HP:0000001 ! All

[Term]
id: HP:0000118
name: Phenotypic abnormality
def: "A phenotypic abnormality." [HPO:probinson]
synonym: "Organ abnormality" EXACT []
is_a: HP:0000001 ! All

[Term]
id: HP:0001507
name: Growth abnormality
is_a: HP:0000118 ! Phenotypic abnormality
relationship: part_of HP:0000118 ! Phenotypic abnormality

[Term]
id: HP:0040064
name: Abnormality of limbs
is_a: HP:0001507 ! Growth abnormality
"#;

    #[test]
    fn test_parse_obo_basic() {
        let parsed = HpoParser::parse_obo(SAMPLE_OBO, "2026-03-01", None).unwrap();
        assert_eq!(parsed.terms.len(), 4);
        assert_eq!(parsed.terms[0].hpo_id, "HP:0000001");
        assert_eq!(parsed.terms[1].hpo_id, "HP:0000118");
        assert_eq!(parsed.terms[2].hpo_id, "HP:0001507");
    }

    #[test]
    fn test_parse_obo_relationships() {
        let parsed = HpoParser::parse_obo(SAMPLE_OBO, "2026-03-01", None).unwrap();
        // HP:0000118 is_a HP:0000001
        let rel = parsed
            .relationships
            .iter()
            .find(|r| r.subject_hpo_id == "HP:0000118" && r.object_hpo_id == "HP:0000001")
            .unwrap();
        assert_eq!(rel.relationship_type, "is_a");

        // HP:0001507 part_of HP:0000118
        let part_of = parsed
            .relationships
            .iter()
            .find(|r| r.subject_hpo_id == "HP:0001507" && r.relationship_type == "part_of")
            .unwrap();
        assert_eq!(part_of.object_hpo_id, "HP:0000118");
    }

    #[test]
    fn test_parse_obo_definition_and_synonym() {
        let parsed = HpoParser::parse_obo(SAMPLE_OBO, "2026-03-01", None).unwrap();
        let term = parsed
            .terms
            .iter()
            .find(|t| t.hpo_id == "HP:0000118")
            .unwrap();
        assert_eq!(term.definition.as_deref(), Some("A phenotypic abnormality."));
        assert_eq!(term.synonyms.len(), 1);
        assert_eq!(term.synonyms[0].text, "Organ abnormality");
        assert_eq!(term.synonyms[0].scope, SynonymScope::Exact);
    }

    #[test]
    fn test_parse_obo_limit() {
        let parsed = HpoParser::parse_obo(SAMPLE_OBO, "2026-03-01", Some(2)).unwrap();
        assert_eq!(parsed.terms.len(), 2);
    }

    #[test]
    fn test_parse_hpoa_basic() {
        let content = "\
#description: HPO disease annotations
#DatabaseID\tDiseaseName\tQualifier\tHPO_ID\tReference\tEvidence\tOnset\tFrequency\tSex\tModifier\tAspect\tBiocuration
OMIM:114500\tBreast-ovarian cancer\t\tHP:0003002\tOMIM:114500\tPCS\t\tHP:0040280\t\t\tP\tHPO:probinson[2009-02-17]
ORPHA:68335\tNeonatal diabetes\tNOT\tHP:0000819\tPMID:12345678\tIEA\t\t\t\t\tP\t
";
        let annotations = HpoParser::parse_hpoa(content, "2026-03-01", None).unwrap();
        assert_eq!(annotations.len(), 2);

        let a0 = &annotations[0];
        assert_eq!(a0.disease_db, "OMIM");
        assert_eq!(a0.disease_id, "114500");
        assert_eq!(a0.hpo_id, "HP:0003002");
        assert!(a0.qualifier.is_none());
        assert_eq!(a0.evidence.as_deref(), Some("PCS"));

        let a1 = &annotations[1];
        assert_eq!(a1.disease_db, "ORPHA");
        assert_eq!(a1.disease_id, "68335");
        assert_eq!(a1.qualifier.as_deref(), Some("NOT"));
    }

    #[test]
    fn test_parse_hpoa_skips_comments() {
        let content = "\
#comment line\n\
# another comment\n\
OMIM:114500\tBreast cancer\t\tHP:0003002\tOMIM:114500\tPCS\t\t\t\t\tP\t\n\
";
        let annotations = HpoParser::parse_hpoa(content, "2026-03-01", None).unwrap();
        assert_eq!(annotations.len(), 1);
    }

    #[test]
    fn test_parse_hpoa_limit() {
        let rows: Vec<String> = (0..10)
            .map(|i| {
                format!(
                    "OMIM:{:06}\tDisease{i}\t\tHP:0000001\tOMIM:{i}\tPCS\t\t\t\t\tP\t",
                    i
                )
            })
            .collect();
        let content = rows.join("\n");
        let annotations = HpoParser::parse_hpoa(&content, "2026-03-01", Some(5)).unwrap();
        assert_eq!(annotations.len(), 5);
    }

    #[test]
    fn test_extract_quoted_text() {
        assert_eq!(
            extract_quoted_text("\"A phenotypic abnormality.\" [HPO:probinson]"),
            "A phenotypic abnormality."
        );
        assert_eq!(extract_quoted_text("no quotes"), "no quotes");
    }

    #[test]
    fn test_opt_str() {
        assert_eq!(opt_str(""), None);
        assert_eq!(opt_str("hello"), Some("hello".to_string()));
    }
}
