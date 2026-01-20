// Gene Ontology Data Models

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;

// ============================================================================
// GO Term
// ============================================================================

/// Represents a Gene Ontology term
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoTerm {
    /// GO identifier (e.g., "GO:0008150")
    pub go_id: String,

    /// Numeric accession (e.g., 8150)
    pub go_accession: i64,

    /// Term name (e.g., "biological_process")
    pub name: String,

    /// Term definition
    pub definition: Option<String>,

    /// Namespace (biological_process, molecular_function, cellular_component)
    pub namespace: Namespace,

    /// Whether the term is obsolete
    pub is_obsolete: bool,

    /// Synonyms
    pub synonyms: Vec<Synonym>,

    /// Cross-references to other databases
    pub xrefs: Vec<String>,

    /// Alternative GO IDs
    pub alt_ids: Vec<String>,

    /// Comments
    pub comments: Option<String>,

    /// GO release version (e.g., "2026-01-01")
    pub go_release_version: String,
}

impl GoTerm {
    /// Parse GO ID into accession number
    /// Example: "GO:0008150" -> 8150
    pub fn parse_accession(go_id: &str) -> Result<i64, String> {
        if !go_id.starts_with("GO:") {
            return Err(format!("Invalid GO ID format: {}", go_id));
        }

        let accession_str = &go_id[3..];
        accession_str
            .parse::<i64>()
            .map_err(|e| format!("Failed to parse GO accession: {}", e))
    }

    /// Validate GO ID format
    pub fn validate_go_id(go_id: &str) -> bool {
        go_id.starts_with("GO:") && go_id.len() == 10 && go_id[3..].chars().all(|c| c.is_ascii_digit())
    }

    /// Create new GoTerm with validation
    pub fn new(
        go_id: String,
        name: String,
        namespace: Namespace,
        go_release_version: String,
    ) -> Result<Self, String> {
        if !Self::validate_go_id(&go_id) {
            return Err(format!("Invalid GO ID: {}", go_id));
        }

        let go_accession = Self::parse_accession(&go_id)?;

        Ok(GoTerm {
            go_id,
            go_accession,
            name,
            definition: None,
            namespace,
            is_obsolete: false,
            synonyms: Vec::new(),
            xrefs: Vec::new(),
            alt_ids: Vec::new(),
            comments: None,
            go_release_version,
        })
    }
}

// ============================================================================
// GO Namespace
// ============================================================================

/// GO Namespace (ontology type)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Namespace {
    BiologicalProcess,
    MolecularFunction,
    CellularComponent,
}

impl Namespace {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "biological_process" => Ok(Namespace::BiologicalProcess),
            "molecular_function" => Ok(Namespace::MolecularFunction),
            "cellular_component" => Ok(Namespace::CellularComponent),
            _ => Err(format!("Unknown namespace: {}", s)),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Namespace::BiologicalProcess => "biological_process",
            Namespace::MolecularFunction => "molecular_function",
            Namespace::CellularComponent => "cellular_component",
        }
    }
}

impl std::fmt::Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Synonym
// ============================================================================

/// GO term synonym
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Synonym {
    /// Synonym scope (EXACT, BROAD, NARROW, RELATED)
    pub scope: SynonymScope,

    /// Synonym text
    pub text: String,

    /// Synonym type (optional, e.g., "systematic_synonym")
    pub synonym_type: Option<String>,

    /// Cross-references supporting this synonym
    pub xrefs: Vec<String>,
}

/// Synonym scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SynonymScope {
    Exact,
    Broad,
    Narrow,
    Related,
}

impl SynonymScope {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "EXACT" => Ok(SynonymScope::Exact),
            "BROAD" => Ok(SynonymScope::Broad),
            "NARROW" => Ok(SynonymScope::Narrow),
            "RELATED" => Ok(SynonymScope::Related),
            _ => Err(format!("Unknown synonym scope: {}", s)),
        }
    }
}

// ============================================================================
// GO Relationship
// ============================================================================

/// Represents a relationship between two GO terms (DAG edge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoRelationship {
    /// Subject (child) GO ID
    pub subject_go_id: String,

    /// Object (parent) GO ID
    pub object_go_id: String,

    /// Relationship type
    pub relationship_type: RelationshipType,

    /// GO release version
    pub go_release_version: String,
}

impl GoRelationship {
    pub fn new(
        subject_go_id: String,
        object_go_id: String,
        relationship_type: RelationshipType,
        go_release_version: String,
    ) -> Self {
        GoRelationship {
            subject_go_id,
            object_go_id,
            relationship_type,
            go_release_version,
        }
    }
}

// ============================================================================
// Relationship Type
// ============================================================================

/// GO relationship types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    IsA,
    PartOf,
    Regulates,
    PositivelyRegulates,
    NegativelyRegulates,
    HasPart,
    OccursIn,
    EndsDuring,
}

impl RelationshipType {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "is_a" => Ok(RelationshipType::IsA),
            "part_of" => Ok(RelationshipType::PartOf),
            "regulates" => Ok(RelationshipType::Regulates),
            "positively_regulates" => Ok(RelationshipType::PositivelyRegulates),
            "negatively_regulates" => Ok(RelationshipType::NegativelyRegulates),
            "has_part" => Ok(RelationshipType::HasPart),
            "occurs_in" => Ok(RelationshipType::OccursIn),
            "ends_during" => Ok(RelationshipType::EndsDuring),
            _ => Err(format!("Unknown relationship type: {}", s)),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RelationshipType::IsA => "is_a",
            RelationshipType::PartOf => "part_of",
            RelationshipType::Regulates => "regulates",
            RelationshipType::PositivelyRegulates => "positively_regulates",
            RelationshipType::NegativelyRegulates => "negatively_regulates",
            RelationshipType::HasPart => "has_part",
            RelationshipType::OccursIn => "occurs_in",
            RelationshipType::EndsDuring => "ends_during",
        }
    }
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// GO Annotation
// ============================================================================

/// Represents a GO annotation linking a protein/gene to a GO term
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoAnnotation {
    /// Entity type (protein or gene)
    pub entity_type: EntityType,

    /// Entity ID (data_source_id from protein_metadata or gene_metadata)
    pub entity_id: Uuid,

    /// GO term ID
    pub go_id: String,

    /// Evidence code (e.g., "IDA", "IEA")
    pub evidence_code: EvidenceCode,

    /// Qualifier (e.g., "NOT", "contributes_to")
    pub qualifier: Option<String>,

    /// Reference (e.g., "PMID:12345678")
    pub reference: Option<String>,

    /// With/From supporting evidence
    pub with_from: Option<String>,

    /// Annotation source (e.g., "UniProtKB")
    pub annotation_source: Option<String>,

    /// Assigned by (e.g., "UniProt")
    pub assigned_by: Option<String>,

    /// Annotation date
    pub annotation_date: Option<NaiveDate>,

    /// Taxonomy ID (NCBI)
    pub taxonomy_id: Option<i64>,

    /// Annotation extension (complex properties)
    pub annotation_extension: Option<serde_json::Value>,

    /// Gene product form ID (isoform/variant)
    pub gene_product_form_id: Option<String>,

    /// GOA release version
    pub goa_release_version: String,
}

// ============================================================================
// Entity Type
// ============================================================================

/// Type of entity being annotated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Protein,
    Gene,
}

impl EntityType {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "protein" => Ok(EntityType::Protein),
            "gene" => Ok(EntityType::Gene),
            _ => Err(format!("Unknown entity type: {}", s)),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Protein => "protein",
            EntityType::Gene => "gene",
        }
    }
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Evidence Code
// ============================================================================

/// GO evidence codes
/// See: http://geneontology.org/docs/guide-go-evidence-codes/
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceCode(pub String);

impl EvidenceCode {
    /// Experimental evidence codes
    pub const EXP: &'static str = "EXP"; // Inferred from Experiment
    pub const IDA: &'static str = "IDA"; // Inferred from Direct Assay
    pub const IPI: &'static str = "IPI"; // Inferred from Physical Interaction
    pub const IMP: &'static str = "IMP"; // Inferred from Mutant Phenotype
    pub const IGI: &'static str = "IGI"; // Inferred from Genetic Interaction
    pub const IEP: &'static str = "IEP"; // Inferred from Expression Pattern

    /// Computational analysis evidence
    pub const ISS: &'static str = "ISS"; // Inferred from Sequence/Structural Similarity
    pub const ISO: &'static str = "ISO"; // Inferred from Sequence Orthology
    pub const ISA: &'static str = "ISA"; // Inferred from Sequence Alignment
    pub const ISM: &'static str = "ISM"; // Inferred from Sequence Model
    pub const IGC: &'static str = "IGC"; // Inferred from Genomic Context
    pub const IBA: &'static str = "IBA"; // Inferred from Biological aspect of Ancestor
    pub const IBD: &'static str = "IBD"; // Inferred from Biological aspect of Descendant
    pub const IKR: &'static str = "IKR"; // Inferred from Key Residues
    pub const IRD: &'static str = "IRD"; // Inferred from Rapid Divergence

    /// Author statement evidence
    pub const TAS: &'static str = "TAS"; // Traceable Author Statement
    pub const NAS: &'static str = "NAS"; // Non-traceable Author Statement

    /// Curator statement evidence
    pub const IC: &'static str = "IC"; // Inferred by Curator
    pub const ND: &'static str = "ND"; // No biological Data available

    /// Electronic annotation evidence
    pub const IEA: &'static str = "IEA"; // Inferred from Electronic Annotation

    pub fn new(code: String) -> Self {
        EvidenceCode(code)
    }

    pub fn is_experimental(&self) -> bool {
        matches!(
            self.0.as_str(),
            Self::EXP | Self::IDA | Self::IPI | Self::IMP | Self::IGI | Self::IEP
        )
    }

    pub fn is_computational(&self) -> bool {
        matches!(
            self.0.as_str(),
            Self::ISS
                | Self::ISO
                | Self::ISA
                | Self::ISM
                | Self::IGC
                | Self::IBA
                | Self::IBD
                | Self::IKR
                | Self::IRD
        )
    }

    pub fn is_electronic(&self) -> bool {
        self.0 == Self::IEA
    }
}

impl std::fmt::Display for EvidenceCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_go_accession() {
        assert_eq!(GoTerm::parse_accession("GO:0008150").unwrap(), 8150);
        assert_eq!(GoTerm::parse_accession("GO:0000001").unwrap(), 1);
        assert_eq!(GoTerm::parse_accession("GO:1234567").unwrap(), 1234567);
        assert!(GoTerm::parse_accession("INVALID").is_err());
    }

    #[test]
    fn test_validate_go_id() {
        assert!(GoTerm::validate_go_id("GO:0008150"));
        assert!(GoTerm::validate_go_id("GO:0000001"));
        assert!(!GoTerm::validate_go_id("GO:123")); // Too short
        assert!(!GoTerm::validate_go_id("GO:12345678")); // Too long
        assert!(!GoTerm::validate_go_id("INVALID"));
    }

    #[test]
    fn test_namespace_from_str() {
        assert_eq!(
            Namespace::from_str("biological_process").unwrap(),
            Namespace::BiologicalProcess
        );
        assert_eq!(
            Namespace::from_str("molecular_function").unwrap(),
            Namespace::MolecularFunction
        );
        assert_eq!(
            Namespace::from_str("cellular_component").unwrap(),
            Namespace::CellularComponent
        );
        assert!(Namespace::from_str("invalid").is_err());
    }

    #[test]
    fn test_relationship_type_from_str() {
        assert_eq!(
            RelationshipType::from_str("is_a").unwrap(),
            RelationshipType::IsA
        );
        assert_eq!(
            RelationshipType::from_str("part_of").unwrap(),
            RelationshipType::PartOf
        );
        assert_eq!(
            RelationshipType::from_str("regulates").unwrap(),
            RelationshipType::Regulates
        );
        assert!(RelationshipType::from_str("invalid").is_err());
    }

    #[test]
    fn test_entity_type_from_str() {
        assert_eq!(EntityType::from_str("protein").unwrap(), EntityType::Protein);
        assert_eq!(EntityType::from_str("gene").unwrap(), EntityType::Gene);
        assert_eq!(EntityType::from_str("PROTEIN").unwrap(), EntityType::Protein);
        assert!(EntityType::from_str("invalid").is_err());
    }

    #[test]
    fn test_evidence_code_classification() {
        let exp = EvidenceCode::new("IDA".to_string());
        assert!(exp.is_experimental());
        assert!(!exp.is_computational());
        assert!(!exp.is_electronic());

        let comp = EvidenceCode::new("ISS".to_string());
        assert!(!comp.is_experimental());
        assert!(comp.is_computational());
        assert!(!comp.is_electronic());

        let elec = EvidenceCode::new("IEA".to_string());
        assert!(!elec.is_experimental());
        assert!(!elec.is_computational());
        assert!(elec.is_electronic());
    }

    #[test]
    fn test_go_term_new() {
        let term = GoTerm::new(
            "GO:0008150".to_string(),
            "biological_process".to_string(),
            Namespace::BiologicalProcess,
            "2026-01-01".to_string(),
        )
        .unwrap();

        assert_eq!(term.go_id, "GO:0008150");
        assert_eq!(term.go_accession, 8150);
        assert_eq!(term.name, "biological_process");
        assert_eq!(term.namespace, Namespace::BiologicalProcess);
        assert!(!term.is_obsolete);
    }

    #[test]
    fn test_synonym_scope_from_str() {
        assert_eq!(SynonymScope::from_str("EXACT").unwrap(), SynonymScope::Exact);
        assert_eq!(SynonymScope::from_str("exact").unwrap(), SynonymScope::Exact);
        assert_eq!(SynonymScope::from_str("BROAD").unwrap(), SynonymScope::Broad);
        assert!(SynonymScope::from_str("invalid").is_err());
    }
}
