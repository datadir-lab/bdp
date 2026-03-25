// crates/bdp-ingest/src/common/obo.rs
//
// Generic OBO 1.4 format parser.
// Returns RawOboTerm structs — each pipeline maps these to its domain types.
//
// Reference: https://owlcollab.github.io/oboformat/doc/GO.format.obo-1_4.html

use tracing::{debug, info};

/// A synonym entry from an OBO term.
#[derive(Debug, Clone)]
pub struct RawOboSynonym {
    /// EXACT, BROAD, NARROW, or RELATED
    pub scope: String,
    pub text: String,
    /// Optional synonym type name (e.g., "systematic_synonym")
    pub synonym_type: Option<String>,
}

/// A typed relationship from an OBO term (not is_a).
#[derive(Debug, Clone)]
pub struct RawOboRelationship {
    /// Relation type: "part_of", "regulates", "positively_regulates", etc.
    pub rel_type: String,
    /// Target term ID: "GO:0006955", "MONDO:0004992"
    pub target: String,
}

/// A raw, parsed OBO term with no domain-specific interpretation.
/// All fields are strings/vecs — the pipeline adapter does the semantic mapping.
#[derive(Debug, Clone, Default)]
pub struct RawOboTerm {
    /// Primary identifier: "GO:0008150", "MONDO:0004992", "HP:0000001"
    pub id: String,
    pub name: String,
    /// Raw namespace string: "biological_process", "disease", "HP"
    pub namespace: Option<String>,
    /// Term definition (the `def:` field, quotes stripped)
    pub definition: Option<String>,
    pub is_obsolete: bool,
    pub synonyms: Vec<RawOboSynonym>,
    /// Raw xref strings: "OMIM:123456", "Wikipedia:Immune_response"
    pub xrefs: Vec<String>,
    /// Alternative IDs for the same concept
    pub alt_ids: Vec<String>,
    pub comment: Option<String>,
    /// Parent term IDs from `is_a:` lines
    pub is_a: Vec<String>,
    /// Other typed relationships
    pub relationships: Vec<RawOboRelationship>,
    /// Property-value pairs: ("inchikey", "AAAA..."), ("smiles", "C(=O)...")
    pub property_values: Vec<(String, String)>,
}

#[derive(Debug, thiserror::Error)]
pub enum OboParseError {
    #[error("OBO parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
}

/// Generic OBO 1.4 parser.
/// Parses `[Term]` stanzas only (ignores `[Typedef]`).
pub struct OboParser;

impl OboParser {
    /// Parse an OBO format string into a list of raw terms.
    ///
    /// # Arguments
    /// * `content` - Full text of the .obo file
    /// * `limit` - Optional maximum number of terms to parse (for testing)
    pub fn parse(content: &str, limit: Option<usize>) -> Result<Vec<RawOboTerm>, OboParseError> {
        let mut terms = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        info!("OBO parse start: {} lines", lines.len());

        // Skip header
        while i < lines.len() && lines[i].trim() != "[Term]" {
            i += 1;
        }

        while i < lines.len() {
            if let Some(max) = limit {
                if terms.len() >= max {
                    info!("OBO parse limit reached: {} terms", max);
                    break;
                }
            }

            let line = lines[i].trim();

            if line == "[Term]" {
                i += 1;
                let (term, next_i) = Self::parse_stanza(&lines, i)?;
                if !term.id.is_empty() {
                    terms.push(term);
                }
                i = next_i;
            } else if line == "[Typedef]" {
                // Skip typedef stanzas
                i += 1;
                while i < lines.len() {
                    let l = lines[i].trim();
                    if l == "[Term]" || l == "[Typedef]" {
                        break;
                    }
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        info!("OBO parse complete: {} terms", terms.len());
        Ok(terms)
    }

    fn parse_stanza(lines: &[&str], start: usize) -> Result<(RawOboTerm, usize), OboParseError> {
        let mut term = RawOboTerm::default();
        let mut i = start;

        while i < lines.len() {
            let line = lines[i].trim();

            if line.is_empty() || line == "[Term]" || line == "[Typedef]" {
                break;
            }

            if let Some(rest) = line.strip_prefix("id: ") {
                term.id = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("name: ") {
                term.name = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("namespace: ") {
                term.namespace = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("def: ") {
                // def: "definition text" [xref1, xref2]
                // Strip leading/trailing quote
                let def = rest.trim().trim_start_matches('"');
                let def = if let Some(end) = def.find("\" [") {
                    &def[..end]
                } else if let Some(end) = def.rfind('"') {
                    &def[..end]
                } else {
                    def
                };
                term.definition = Some(def.to_string());
            } else if let Some(rest) = line.strip_prefix("is_obsolete: ") {
                term.is_obsolete = rest.trim() == "true";
            } else if let Some(rest) = line.strip_prefix("comment: ") {
                term.comment = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("synonym: ") {
                if let Some(syn) = Self::parse_synonym(rest.trim()) {
                    term.synonyms.push(syn);
                }
            } else if let Some(rest) = line.strip_prefix("xref: ") {
                // Take only the xref ID part, ignore trailing description
                let xref = rest
                    .trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                if !xref.is_empty() {
                    term.xrefs.push(xref);
                }
            } else if let Some(rest) = line.strip_prefix("alt_id: ") {
                term.alt_ids.push(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("is_a: ") {
                // is_a: GO:0006950 ! response to stress
                let parent_id = rest
                    .trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                if !parent_id.is_empty() {
                    term.is_a.push(parent_id);
                }
            } else if let Some(rest) = line.strip_prefix("relationship: ") {
                // relationship: part_of GO:0006950 ! response to stress
                let mut parts = rest.trim().splitn(3, ' ');
                if let (Some(rel_type), Some(target)) = (parts.next(), parts.next()) {
                    term.relationships.push(RawOboRelationship {
                        rel_type: rel_type.to_string(),
                        target: target
                            .split('!')
                            .next()
                            .unwrap_or(target)
                            .trim()
                            .to_string(),
                    });
                }
            } else if let Some(rest) = line.strip_prefix("property_value: ") {
                // property_value: inchikey "UHOVQNZJYSORNB-UHFFFAOYSA-N" xsd:string
                let mut parts = rest.trim().splitn(3, ' ');
                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                    let value = value.trim_matches('"').to_string();
                    term.property_values.push((key.to_string(), value));
                }
            } else {
                debug!("OBO: unhandled field at line {}: {}", i, line);
            }

            i += 1;
        }

        Ok((term, i))
    }

    fn parse_synonym(text: &str) -> Option<RawOboSynonym> {
        // Format: "synonym text" EXACT [xrefs]
        // or: "synonym text" EXACT synonym_type_name [xrefs]
        let text = text.trim_start_matches('"');
        let end_quote = text.find('"')?;
        let synonym_text = text[..end_quote].to_string();
        let rest = text[end_quote + 1..].trim();

        let mut parts = rest.split_whitespace();
        let scope = parts.next()?.to_string();

        // Optional synonym type name (before the '[' of xrefs)
        let synonym_type = {
            let next = parts.next()?;
            if next.starts_with('[') {
                None
            } else {
                Some(next.to_string())
            }
        };

        Some(RawOboSynonym {
            scope,
            text: synonym_text,
            synonym_type,
        })
    }

    /// Split "DB:ID" xref strings into (db, id) pairs.
    /// "OMIM:604606" → ("OMIM", "604606")
    /// "Wikipedia:Immune_response" → ("Wikipedia", "Immune_response")
    pub fn split_xref(xref: &str) -> (String, String) {
        if let Some(colon) = xref.find(':') {
            (xref[..colon].to_string(), xref[colon + 1..].to_string())
        } else {
            ("unknown".to_string(), xref.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_OBO: &str = r#"
format-version: 1.2
ontology: test

[Term]
id: GO:0006955
name: immune response
namespace: biological_process
def: "A defense reaction by which organisms protect against infection." [GOC:mah]
synonym: "immune reactions" EXACT []
synonym: "immunity" BROAD []
xref: Wikipedia:Immune_response
xref: KEGG:ko04620
is_a: GO:0006950 ! response to stress
relationship: part_of GO:0002376 ! immune system process
alt_id: GO:0001234

[Term]
id: GO:0000000
name: obsolete term
namespace: biological_process
is_obsolete: true
"#;

    #[test]
    fn test_parse_basic_term() {
        let terms = OboParser::parse(SAMPLE_OBO, None).unwrap();
        assert_eq!(terms.len(), 2);

        let t = &terms[0];
        assert_eq!(t.id, "GO:0006955");
        assert_eq!(t.name, "immune response");
        assert_eq!(t.namespace.as_deref(), Some("biological_process"));
        assert!(!t.is_obsolete);
        assert_eq!(t.synonyms.len(), 2);
        assert_eq!(t.synonyms[0].scope, "EXACT");
        assert_eq!(t.synonyms[0].text, "immune reactions");
        assert_eq!(t.synonyms[1].scope, "BROAD");
        assert_eq!(t.xrefs.len(), 2);
        assert!(t.xrefs.contains(&"Wikipedia:Immune_response".to_string()));
        assert_eq!(t.is_a.len(), 1);
        assert_eq!(t.is_a[0], "GO:0006950");
        assert_eq!(t.alt_ids.len(), 1);
        assert_eq!(t.alt_ids[0], "GO:0001234");
        assert_eq!(t.relationships.len(), 1);
        assert_eq!(t.relationships[0].rel_type, "part_of");
    }

    #[test]
    fn test_parse_obsolete_term() {
        let terms = OboParser::parse(SAMPLE_OBO, None).unwrap();
        assert!(terms[1].is_obsolete);
    }

    #[test]
    fn test_parse_limit() {
        let terms = OboParser::parse(SAMPLE_OBO, Some(1)).unwrap();
        assert_eq!(terms.len(), 1);
    }

    #[test]
    fn test_split_xref() {
        assert_eq!(
            OboParser::split_xref("OMIM:604606"),
            ("OMIM".to_string(), "604606".to_string())
        );
        assert_eq!(
            OboParser::split_xref("Wikipedia:Immune_response"),
            ("Wikipedia".to_string(), "Immune_response".to_string())
        );
        assert_eq!(
            OboParser::split_xref("nocolon"),
            ("unknown".to_string(), "nocolon".to_string())
        );
    }

    #[test]
    fn test_parse_synonym_types() {
        let syn = OboParser::parse_synonym(r#""exact name" EXACT []"#);
        assert!(syn.is_some());
        let syn = syn.unwrap();
        assert_eq!(syn.scope, "EXACT");
        assert_eq!(syn.text, "exact name");
        assert!(syn.synonym_type.is_none());
    }
}
