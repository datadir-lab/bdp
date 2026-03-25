//! Domain types for the Gene Ontology pipeline.
//!
//! These are lightweight, database-agnostic representations of GO concepts.
//! The bdp-server GO pipeline has richer types including SQLx-derived fields;
//! these are the portable versions suitable for use in bdp-ingest.

use serde::{Deserialize, Serialize};

// ============================================================================
// Namespace
// ============================================================================

/// The three GO sub-ontologies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Namespace {
    BiologicalProcess,
    MolecularFunction,
    CellularComponent,
}

impl Namespace {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "biological_process" => Some(Namespace::BiologicalProcess),
            "molecular_function" => Some(Namespace::MolecularFunction),
            "cellular_component" => Some(Namespace::CellularComponent),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Namespace::BiologicalProcess => "biological_process",
            Namespace::MolecularFunction => "molecular_function",
            Namespace::CellularComponent => "cellular_component",
        }
    }
}

impl std::fmt::Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// RelationshipType
// ============================================================================

/// Edge types in the GO DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    IsA,
    PartOf,
    Regulates,
    PositivelyRegulates,
    NegativelyRegulates,
    HasPart,
    OccursIn,
    /// Catch-all for any other relation encountered in the file.
    Other(String),
}

impl RelationshipType {
    pub fn parse(s: &str) -> Self {
        match s {
            "is_a" => RelationshipType::IsA,
            "part_of" => RelationshipType::PartOf,
            "regulates" => RelationshipType::Regulates,
            "positively_regulates" => RelationshipType::PositivelyRegulates,
            "negatively_regulates" => RelationshipType::NegativelyRegulates,
            "has_part" => RelationshipType::HasPart,
            "occurs_in" => RelationshipType::OccursIn,
            other => RelationshipType::Other(other.to_string()),
        }
    }
}

// ============================================================================
// GoTerm
// ============================================================================

/// A parsed GO term.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoTerm {
    /// Primary identifier, e.g. `GO:0008150`.
    pub go_id: String,
    /// Human-readable label.
    pub name: String,
    /// Ontology namespace.
    pub namespace: Namespace,
    /// Term definition (stripped of surrounding quotes).
    pub definition: Option<String>,
    pub is_obsolete: bool,
    /// Alternative IDs for the same concept.
    pub alt_ids: Vec<String>,
    pub comment: Option<String>,
}

// ============================================================================
// GoRelationship
// ============================================================================

/// A directed edge between two GO terms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoRelationship {
    /// Subject (child) GO ID.
    pub subject_go_id: String,
    /// Object (parent / target) GO ID.
    pub object_go_id: String,
    pub relationship_type: RelationshipType,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_round_trip() {
        for (s, ns) in [
            ("biological_process", Namespace::BiologicalProcess),
            ("molecular_function", Namespace::MolecularFunction),
            ("cellular_component", Namespace::CellularComponent),
        ] {
            assert_eq!(Namespace::parse(s), Some(ns));
            assert_eq!(ns.as_str(), s);
        }
        assert_eq!(Namespace::parse("unknown"), None);
    }

    #[test]
    fn test_relationship_type_parse() {
        assert_eq!(RelationshipType::parse("is_a"), RelationshipType::IsA);
        assert_eq!(RelationshipType::parse("part_of"), RelationshipType::PartOf);
        assert_eq!(RelationshipType::parse("regulates"), RelationshipType::Regulates);
        assert!(matches!(RelationshipType::parse("novel_rel"), RelationshipType::Other(_)));
    }
}
