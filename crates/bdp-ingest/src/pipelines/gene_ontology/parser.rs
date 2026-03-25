//! Maps generic [`RawOboTerm`] values to GO domain types.
//!
//! The generic [`OboParser`](crate::common::obo::OboParser) handles the
//! byte-level OBO format. This module adds GO-specific semantics:
//! namespace validation, relationship extraction, and struct construction.

use crate::common::obo::{OboParser, RawOboTerm};
use crate::pipelines::gene_ontology::models::{GoRelationship, GoTerm, Namespace, RelationshipType};
use tracing::{debug, warn};

/// Parsed output from the GO OBO file.
#[derive(Debug, Default)]
pub struct ParsedGo {
    pub terms: Vec<GoTerm>,
    pub relationships: Vec<GoRelationship>,
}

impl ParsedGo {
    pub fn term_count(&self) -> usize {
        self.terms.len()
    }
    pub fn relationship_count(&self) -> usize {
        self.relationships.len()
    }
}

/// Parse a `go-basic.obo` file into domain types.
///
/// # Arguments
/// * `content` – Full text of the `.obo` file.
/// * `limit`   – Optional cap on the number of terms (useful in tests).
pub fn parse_obo(content: &str, limit: Option<usize>) -> anyhow::Result<ParsedGo> {
    let raw_terms = OboParser::parse(content, limit)?;
    let mut result = ParsedGo::default();

    for raw in raw_terms {
        // Skip terms with no valid GO namespace
        let Some(namespace) = raw.namespace.as_deref().and_then(Namespace::parse) else {
            debug!(id = %raw.id, "skipping GO term with unknown namespace");
            continue;
        };

        let term = map_term(&raw, namespace);
        let relationships = extract_relationships(&raw);

        result.terms.push(term);
        result.relationships.extend(relationships);
    }

    Ok(result)
}

fn map_term(raw: &RawOboTerm, namespace: Namespace) -> GoTerm {
    if !raw.id.starts_with("GO:") {
        warn!(id = %raw.id, "non-GO term id encountered in GO OBO file");
    }

    GoTerm {
        go_id: raw.id.clone(),
        name: raw.name.clone(),
        namespace,
        definition: raw.definition.clone(),
        is_obsolete: raw.is_obsolete,
        alt_ids: raw.alt_ids.clone(),
        comment: raw.comment.clone(),
    }
}

fn extract_relationships(raw: &RawOboTerm) -> Vec<GoRelationship> {
    let mut rels = Vec::new();

    // `is_a:` lines — treated as IsA edges
    for parent in &raw.is_a {
        rels.push(GoRelationship {
            subject_go_id: raw.id.clone(),
            object_go_id: parent.clone(),
            relationship_type: RelationshipType::IsA,
        });
    }

    // `relationship:` lines — typed edges
    for rel in &raw.relationships {
        rels.push(GoRelationship {
            subject_go_id: raw.id.clone(),
            object_go_id: rel.target.clone(),
            relationship_type: RelationshipType::parse(&rel.rel_type),
        });
    }

    rels
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipelines::gene_ontology::models::Namespace;

    const SAMPLE_OBO: &str = r#"
format-version: 1.2
ontology: go

[Term]
id: GO:0008150
name: biological_process
namespace: biological_process
def: "Any process specifically pertinent to the functioning of integrated living units." [GOC:go_curators]
is_a: GO:0003674 ! molecular_function

[Term]
id: GO:0005575
name: cellular_component
namespace: cellular_component
def: "The part of a cell or its extracellular environment in which a gene product is located." [GOC:go_curators]
relationship: part_of GO:0005623 ! cell

[Term]
id: GO:0999999
name: obsolete thing
namespace: biological_process
is_obsolete: true

[Term]
id: HP:0000001
name: not a GO term
namespace: biological_process
"#;

    #[test]
    fn test_parse_terms() {
        let parsed = parse_obo(SAMPLE_OBO, None).unwrap();
        // All 4 terms have a valid namespace; 1 is not a GO: id but still parsed
        assert_eq!(parsed.term_count(), 4);
    }

    #[test]
    fn test_obsolete_flag() {
        let parsed = parse_obo(SAMPLE_OBO, None).unwrap();
        let obsolete = parsed.terms.iter().find(|t| t.go_id == "GO:0999999").unwrap();
        assert!(obsolete.is_obsolete);
    }

    #[test]
    fn test_is_a_relationships() {
        let parsed = parse_obo(SAMPLE_OBO, None).unwrap();
        let isa_rels: Vec<_> = parsed
            .relationships
            .iter()
            .filter(|r| r.relationship_type == RelationshipType::IsA)
            .collect();
        assert_eq!(isa_rels.len(), 1);
        assert_eq!(isa_rels[0].subject_go_id, "GO:0008150");
        assert_eq!(isa_rels[0].object_go_id, "GO:0003674");
    }

    #[test]
    fn test_typed_relationships() {
        let parsed = parse_obo(SAMPLE_OBO, None).unwrap();
        let part_of: Vec<_> = parsed
            .relationships
            .iter()
            .filter(|r| r.relationship_type == RelationshipType::PartOf)
            .collect();
        assert_eq!(part_of.len(), 1);
        assert_eq!(part_of[0].subject_go_id, "GO:0005575");
    }

    #[test]
    fn test_namespace_mapping() {
        let parsed = parse_obo(SAMPLE_OBO, None).unwrap();
        let bp = parsed.terms.iter().find(|t| t.go_id == "GO:0008150").unwrap();
        assert_eq!(bp.namespace, Namespace::BiologicalProcess);
        let cc = parsed.terms.iter().find(|t| t.go_id == "GO:0005575").unwrap();
        assert_eq!(cc.namespace, Namespace::CellularComponent);
    }

    #[test]
    fn test_parse_limit() {
        let parsed = parse_obo(SAMPLE_OBO, Some(2)).unwrap();
        assert_eq!(parsed.term_count(), 2);
    }
}
