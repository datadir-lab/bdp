// MONDO OBO Parser
//
// Parses MONDO OBO 1.4 format into DiseaseTerm / DiseaseRelationship.
// MONDO terms are prefixed with "MONDO:"; non-MONDO terms are skipped.

use crate::pipelines::mondo::models::{
    DiseaseRelationType, DiseaseRelationship, DiseaseSynonym, DiseaseTerm, DiseaseXref, ParsedMondo,
};
use tracing::{info, warn};

/// Parse a MONDO OBO file into domain models.
///
/// # Arguments
/// * `content` - Raw OBO file content
/// * `release` - Release label stored on each term/relationship (e.g., "2026-03-01")
/// * `limit` - Optional cap on the number of MONDO-prefixed terms to parse
pub fn parse_obo(
    content: &str,
    release: &str,
    limit: Option<usize>,
) -> Result<ParsedMondo, String> {
    let mut parsed = ParsedMondo::default();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    info!("Starting MONDO OBO parsing (limit: {:?})", limit);

    // Skip header until first [Term]
    while i < lines.len() {
        if lines[i].trim() == "[Term]" {
            break;
        }
        i += 1;
    }

    // Parse term stanzas
    while i < lines.len() {
        if let Some(max_terms) = limit {
            if parsed.terms.len() >= max_terms {
                info!("Reached MONDO parse limit of {} terms", max_terms);
                break;
            }
        }

        if lines[i].trim() == "[Term]" {
            match parse_term_stanza(&lines, &mut i, release) {
                Ok(Some((term, rels))) => {
                    parsed.relationships.extend(rels);
                    parsed.terms.push(term);
                },
                Ok(None) => {
                    // Non-MONDO term — skip
                },
                Err(e) => {
                    warn!("Skipping MONDO term stanza due to parse error: {}", e);
                },
            }
        } else {
            i += 1;
        }
    }

    info!(
        "MONDO OBO parsed: {} terms, {} relationships",
        parsed.term_count(),
        parsed.relationship_count()
    );

    Ok(parsed)
}

/// Parse a single [Term] stanza.
/// Returns `Ok(None)` for non-MONDO terms (e.g., CHEBI:, HP:, etc.).
fn parse_term_stanza(
    lines: &[&str],
    i: &mut usize,
    release: &str,
) -> Result<Option<(DiseaseTerm, Vec<DiseaseRelationship>)>, String> {
    *i += 1; // skip the "[Term]" line

    let mut id: Option<String> = None;
    let mut name: Option<String> = None;
    let mut definition: Option<String> = None;
    let mut is_obsolete = false;
    let mut comment: Option<String> = None;
    let mut synonyms: Vec<DiseaseSynonym> = Vec::new();
    let mut raw_xrefs: Vec<String> = Vec::new();
    let mut is_a_ids: Vec<String> = Vec::new();
    let mut relationships: Vec<(String, String)> = Vec::new(); // (rel_type, target_id)

    while *i < lines.len() {
        let line = lines[*i].trim();

        // End of stanza
        if line.is_empty() || line.starts_with('[') {
            break;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            // Note: values may contain colons (e.g., "MONDO:0004992"), so we use the full remainder
            let value = value.trim();

            match key {
                "id" => id = Some(value.to_string()),
                "name" => name = Some(value.to_string()),
                "def" => definition = Some(extract_quoted_text(value)),
                "is_obsolete" => is_obsolete = value == "true",
                "comment" => comment = Some(value.to_string()),
                "synonym" => {
                    if let Some(syn) = parse_synonym(value) {
                        synonyms.push(syn);
                    }
                },
                "xref" => {
                    // Strip inline comment: "OMIM:114500 {source=\"...\"}" → "OMIM:114500"
                    let xref_val = value.split_whitespace().next().unwrap_or("").to_string();
                    if !xref_val.is_empty() {
                        raw_xrefs.push(xref_val);
                    }
                },
                "is_a" => {
                    // Format: "MONDO:0000001 ! disease"
                    if let Some(parent_id) = value.split_whitespace().next() {
                        is_a_ids.push(parent_id.to_string());
                    }
                },
                "relationship" => {
                    // Format: "part_of MONDO:0000001 ! disease"
                    let parts: Vec<&str> = value.split_whitespace().collect();
                    if parts.len() >= 2 {
                        relationships.push((parts[0].to_string(), parts[1].to_string()));
                    }
                },
                _ => {},
            }
        }

        *i += 1;
    }

    let term_id = match id {
        Some(ref s) if s.starts_with("MONDO:") => s.clone(),
        _ => return Ok(None), // Non-MONDO term
    };

    let name = name.ok_or_else(|| format!("Missing name for {}", term_id))?;

    // Parse numeric accession
    let mondo_accession: i64 = term_id.trim_start_matches("MONDO:").parse().unwrap_or(0);

    // Build structured xrefs
    let xrefs: Vec<DiseaseXref> = raw_xrefs
        .iter()
        .filter_map(|x| {
            let (db, xid) = x.split_once(':')?;
            Some(DiseaseXref {
                source_db: db.to_string(),
                source_id: xid.to_string(),
            })
        })
        .collect();

    // Denormalized first OMIM / Orphanet xrefs
    let omim_id = xrefs
        .iter()
        .find(|x| x.source_db == "OMIM")
        .map(|x| x.source_id.clone());

    let orphanet_id = xrefs
        .iter()
        .find(|x| x.source_db == "ORPHA" || x.source_db == "Orphanet")
        .map(|x| x.source_id.clone());

    let term = DiseaseTerm {
        mondo_id: term_id.clone(),
        mondo_accession,
        name,
        definition,
        is_obsolete,
        comment,
        omim_id,
        orphanet_id,
        synonyms,
        xrefs,
        mondo_release: release.to_string(),
    };

    // Build relationships: is_a
    let mut rels: Vec<DiseaseRelationship> = is_a_ids
        .into_iter()
        .filter(|p| p.starts_with("MONDO:"))
        .map(|parent| DiseaseRelationship {
            subject_mondo_id: term_id.clone(),
            object_mondo_id: parent,
            relationship_type: DiseaseRelationType::IsA,
            mondo_release: release.to_string(),
        })
        .collect();

    // Other typed relationships
    for (rel_type, target) in relationships {
        if target.starts_with("MONDO:") {
            rels.push(DiseaseRelationship {
                subject_mondo_id: term_id.clone(),
                object_mondo_id: target,
                relationship_type: DiseaseRelationType::from_str(&rel_type),
                mondo_release: release.to_string(),
            });
        }
    }

    Ok(Some((term, rels)))
}

/// Extract quoted text: `"text" [xrefs]` → `text`
fn extract_quoted_text(text: &str) -> String {
    if let Some(start) = text.find('"') {
        if let Some(end) = text[start + 1..].find('"') {
            return text[start + 1..start + 1 + end].to_string();
        }
    }
    text.to_string()
}

/// Parse synonym line: `"text" SCOPE [xrefs]`
fn parse_synonym(text: &str) -> Option<DiseaseSynonym> {
    let parts: Vec<&str> = text.split('"').collect();
    if parts.len() < 3 {
        return None;
    }

    let syn_text = parts[1].to_string();
    let scope_str = parts[2].split_whitespace().next().unwrap_or("EXACT");

    Some(DiseaseSynonym {
        scope: scope_str.to_string(),
        text: syn_text,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MONDO: &str = r#"format-version: 1.2
ontology: mondo

[Term]
id: MONDO:0004992
name: cancer
def: "A disease involving uncontrolled cell growth." [HPO:probinson]
synonym: "malignant neoplasm" EXACT []
synonym: "malignancy" BROAD []
xref: OMIM:114500
xref: ORPHA:68335
is_a: MONDO:0000001 ! disease

[Term]
id: MONDO:0000001
name: disease
def: "A disease is a disposition to undergo pathological processes." [OGMS:0000031]
"#;

    #[test]
    fn test_parse_basic_disease() {
        let parsed = parse_obo(SAMPLE_MONDO, "2026-03-01", None).unwrap();
        assert_eq!(parsed.terms.len(), 2, "expected 2 MONDO terms");

        let cancer = parsed
            .terms
            .iter()
            .find(|t| t.mondo_id == "MONDO:0004992")
            .expect("cancer term not found");

        assert_eq!(cancer.name, "cancer");
        assert_eq!(cancer.mondo_accession, 4992);
        assert_eq!(cancer.omim_id.as_deref(), Some("114500"));
        assert_eq!(cancer.orphanet_id.as_deref(), Some("68335"));
        assert!(!cancer.is_obsolete);
        assert_eq!(cancer.synonyms.len(), 2);
        assert!(cancer.definition.is_some());
    }

    #[test]
    fn test_parse_relationships() {
        let parsed = parse_obo(SAMPLE_MONDO, "2026-03-01", None).unwrap();
        assert_eq!(parsed.relationships.len(), 1, "expected 1 is_a relationship");
        let rel = &parsed.relationships[0];
        assert_eq!(rel.subject_mondo_id, "MONDO:0004992");
        assert_eq!(rel.object_mondo_id, "MONDO:0000001");
        assert_eq!(rel.relationship_type, DiseaseRelationType::IsA);
    }

    #[test]
    fn test_parse_limit() {
        let parsed = parse_obo(SAMPLE_MONDO, "2026-03-01", Some(1)).unwrap();
        assert_eq!(parsed.terms.len(), 1, "limit should cap at 1 term");
    }

    #[test]
    fn test_skip_non_mondo_terms() {
        let content = r#"
[Term]
id: HP:0000001
name: All
def: "Root of all terms." [HP:curators]

[Term]
id: MONDO:0000001
name: disease
"#;
        let parsed = parse_obo(content, "2026-03-01", None).unwrap();
        assert_eq!(parsed.terms.len(), 1, "HP: term should be skipped");
        assert_eq!(parsed.terms[0].mondo_id, "MONDO:0000001");
    }

    #[test]
    fn test_parse_obsolete_term() {
        let content = r#"
[Term]
id: MONDO:0000999
name: obsolete condition
is_obsolete: true
"#;
        let parsed = parse_obo(content, "2026-03-01", None).unwrap();
        assert_eq!(parsed.terms.len(), 1);
        assert!(parsed.terms[0].is_obsolete);
    }

    #[test]
    fn test_xref_parsing() {
        let content = r#"
[Term]
id: MONDO:0000042
name: test disease
xref: OMIM:123456
xref: ORPHA:99999
xref: MeSH:D000001
"#;
        let parsed = parse_obo(content, "2026-03-01", None).unwrap();
        let term = &parsed.terms[0];
        assert_eq!(term.omim_id.as_deref(), Some("123456"));
        assert_eq!(term.orphanet_id.as_deref(), Some("99999"));
        assert_eq!(term.xrefs.len(), 3);
    }

    #[test]
    fn test_typed_relationship() {
        let content = r#"
[Term]
id: MONDO:0000100
name: child disease
relationship: part_of MONDO:0000001 ! disease
"#;
        let parsed = parse_obo(content, "2026-03-01", None).unwrap();
        assert_eq!(parsed.relationships.len(), 1);
        assert_eq!(parsed.relationships[0].relationship_type, DiseaseRelationType::PartOf);
    }
}
