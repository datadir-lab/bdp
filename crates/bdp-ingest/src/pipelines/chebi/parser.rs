// crates/bdp-ingest/src/pipelines/chebi/parser.rs

use crate::common::obo::{OboParseError, OboParser};
use crate::pipelines::chebi::models::*;

pub fn parse_obo(
    content: &str,
    release: &str,
    limit: Option<usize>,
) -> Result<ParsedChebi, OboParseError> {
    let raw = OboParser::parse(content, limit)?;
    let mut parsed = ParsedChebi::default();

    for raw_term in raw {
        if !raw_term.id.starts_with("CHEBI:") {
            continue;
        }

        let accession: i64 = raw_term
            .id
            .trim_start_matches("CHEBI:")
            .parse()
            .unwrap_or(0);

        // Extract chemical properties from property_values
        let mut term = CompoundTerm {
            chebi_id: raw_term.id.clone(),
            chebi_accession: accession,
            name: raw_term.name.clone(),
            definition: raw_term.definition.clone(),
            comment: raw_term.comment.clone(),
            is_obsolete: raw_term.is_obsolete,
            chebi_release: release.to_string(),
            ..Default::default()
        };

        for (key, value) in &raw_term.property_values {
            // ChEBI uses multiple formats depending on version:
            //   - Old: full URIs like http://purl.obolibrary.org/obo/chebi/inchikey
            //   - New (2024+): prefixed like chemrof:inchi_key_string
            // Take the part after the last '/' or ':' for matching.
            let short_key = key
                .rsplit('/')
                .next()
                .unwrap_or(key)
                .rsplit(':')
                .next()
                .unwrap_or(key);
            match short_key {
                // Old-style URI keys
                "inchikey"
                // New-style chemrof keys
                | "inchi_key_string" => term.inchikey = Some(value.clone()),
                "smiles" | "smiles_string" => term.smiles = Some(value.clone()),
                "inchi" | "inchi_string" => term.inchi = Some(value.clone()),
                "formula" | "generalized_empirical_formula" => {
                    term.formula = Some(value.clone())
                }
                "monoisotopicmass" | "monoisotopic_mass" => {
                    term.mass_mono = value.parse().ok();
                }
                "mass" => {
                    // Only set mass_mono from "mass" if not already set by monoisotopic_mass
                    if term.mass_mono.is_none() {
                        term.mass_mono = value.parse().ok();
                    }
                }
                "charge" => {
                    term.charge = value.parse().ok();
                }
                _ => {}
            }
        }

        parsed.terms.push(term);

        // is_a relationships
        for parent_id in &raw_term.is_a {
            if !parent_id.starts_with("CHEBI:") {
                continue;
            }
            parsed.relationships.push(CompoundRelationship {
                subject_chebi_id: raw_term.id.clone(),
                object_chebi_id: parent_id.clone(),
                relationship_type: "is_a".to_string(),
                chebi_release: release.to_string(),
            });
        }

        // Other relationships (has_role, is_conjugate_acid_of, etc.)
        for rel in &raw_term.relationships {
            if !rel.target.starts_with("CHEBI:") {
                continue;
            }
            parsed.relationships.push(CompoundRelationship {
                subject_chebi_id: raw_term.id.clone(),
                object_chebi_id: rel.target.clone(),
                relationship_type: rel.rel_type.clone(),
                chebi_release: release.to_string(),
            });
        }
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
format-version: 1.2
ontology: chebi

[Term]
id: CHEBI:33709
name: amino acid
def: "Organic acid with amino group." [CHEBI]
is_a: CHEBI:25703 ! organic acid
property_value: http://purl.obolibrary.org/obo/chebi/formula "C2H5NO2" xsd:string
property_value: http://purl.obolibrary.org/obo/chebi/mass "75.032" xsd:double
property_value: http://purl.obolibrary.org/obo/chebi/inchikey "DHMQDGOQFOQNFH-UHFFFAOYSA-N" xsd:string

[Term]
id: CHEBI:25703
name: organic acid
"#;

    #[test]
    fn test_parse_compound() {
        let parsed = parse_obo(SAMPLE, "2026-03-01", None).unwrap();
        assert_eq!(parsed.terms.len(), 2);

        let aa = parsed
            .terms
            .iter()
            .find(|t| t.chebi_id == "CHEBI:33709")
            .unwrap();
        assert_eq!(aa.name, "amino acid");
        assert_eq!(aa.formula.as_deref(), Some("C2H5NO2"));
        assert!(aa.mass_mono.is_some());
        assert_eq!(
            aa.inchikey.as_deref(),
            Some("DHMQDGOQFOQNFH-UHFFFAOYSA-N")
        );

        assert_eq!(parsed.relationships.len(), 1);
        assert_eq!(
            parsed.relationships[0].subject_chebi_id,
            "CHEBI:33709"
        );
        assert_eq!(
            parsed.relationships[0].object_chebi_id,
            "CHEBI:25703"
        );
        assert_eq!(parsed.relationships[0].relationship_type, "is_a");
    }
}
