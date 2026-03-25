// HPO Data Models

use serde::{Deserialize, Serialize};

// ============================================================================
// HPO Term
// ============================================================================

/// Represents a Human Phenotype Ontology term
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HpoTerm {
    /// HPO identifier (e.g., "HP:0000001")
    pub hpo_id: String,

    /// Numeric accession (e.g., 1)
    pub hpo_accession: i64,

    /// Term name (e.g., "All")
    pub name: String,

    /// Term definition
    pub definition: Option<String>,

    /// Curator comment
    pub comment: Option<String>,

    /// Whether the term is obsolete
    pub is_obsolete: bool,

    /// Replacement HPO ID when obsolete
    pub replaced_by: Option<String>,

    /// Synonyms
    pub synonyms: Vec<HpoSynonym>,

    /// Cross-references to other databases
    pub xrefs: Vec<String>,

    /// Alternative HPO IDs
    pub alt_ids: Vec<String>,

    /// Subset membership (e.g., "hposlim_core")
    pub subset: Vec<String>,

    /// HPO release version
    pub hpo_release_version: String,
}

impl HpoTerm {
    /// Parse HPO ID to numeric accession
    /// Example: "HP:0000001" -> 1
    pub fn parse_accession(hpo_id: &str) -> std::result::Result<i64, String> {
        let accession_str = hpo_id
            .strip_prefix("HP:")
            .ok_or_else(|| format!("Invalid HPO ID format: {}", hpo_id))?;
        accession_str
            .parse::<i64>()
            .map_err(|e| format!("Failed to parse HPO accession '{}': {}", accession_str, e))
    }

    /// Validate HPO ID format: must be "HP:" followed by 7 digits
    pub fn validate_hpo_id(hpo_id: &str) -> bool {
        if let Some(digits) = hpo_id.strip_prefix("HP:") {
            digits.len() == 7 && digits.chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    }

    /// Create a new HpoTerm with validation
    pub fn new(
        hpo_id: String,
        name: String,
        hpo_release_version: String,
    ) -> std::result::Result<Self, String> {
        if !Self::validate_hpo_id(&hpo_id) {
            return Err(format!("Invalid HPO ID: {}", hpo_id));
        }

        let hpo_accession = Self::parse_accession(&hpo_id)?;

        Ok(HpoTerm {
            hpo_id,
            hpo_accession,
            name,
            definition: None,
            comment: None,
            is_obsolete: false,
            replaced_by: None,
            synonyms: Vec::new(),
            xrefs: Vec::new(),
            alt_ids: Vec::new(),
            subset: Vec::new(),
            hpo_release_version,
        })
    }
}

// ============================================================================
// HPO Synonym
// ============================================================================

/// Synonym scope options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SynonymScope {
    Exact,
    Broad,
    Narrow,
    Related,
}

impl SynonymScope {
    pub fn from_str(s: &str) -> std::result::Result<Self, String> {
        match s.to_uppercase().as_str() {
            "EXACT" => Ok(SynonymScope::Exact),
            "BROAD" => Ok(SynonymScope::Broad),
            "NARROW" => Ok(SynonymScope::Narrow),
            "RELATED" => Ok(SynonymScope::Related),
            _ => Err(format!("Unknown synonym scope: {}", s)),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SynonymScope::Exact => "EXACT",
            SynonymScope::Broad => "BROAD",
            SynonymScope::Narrow => "NARROW",
            SynonymScope::Related => "RELATED",
        }
    }
}

/// HPO term synonym
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HpoSynonym {
    /// Synonym scope (EXACT, BROAD, NARROW, RELATED)
    pub scope: SynonymScope,

    /// Synonym text
    pub text: String,
}

// ============================================================================
// HPO Relationship
// ============================================================================

/// Represents a DAG edge between two HPO terms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HpoRelationship {
    /// Subject (child) HPO ID
    pub subject_hpo_id: String,

    /// Object (parent) HPO ID
    pub object_hpo_id: String,

    /// Relationship type (e.g., "is_a", "part_of")
    pub relationship_type: String,

    /// HPO release version
    pub hpo_release_version: String,
}

// ============================================================================
// Disease Annotation (HPOA)
// ============================================================================

/// Represents a disease-phenotype annotation from phenotype.hpoa
///
/// Column order: database_id, disease_name, qualifier, hpo_id, reference,
///               evidence, onset, frequency, sex, modifier, aspect, biocuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiseaseAnnotation {
    /// Source disease database (e.g., "OMIM", "ORPHA")
    pub disease_db: String,

    /// Disease identifier (e.g., "114500", "68335")
    pub disease_id: String,

    /// Disease name
    pub disease_name: String,

    /// HPO term ID
    pub hpo_id: String,

    /// Qualifier (e.g., "NOT" when phenotype is absent)
    pub qualifier: Option<String>,

    /// Reference (e.g., "PMID:12345678")
    pub reference: Option<String>,

    /// Evidence code (e.g., "IEA", "PCS", "TAS")
    pub evidence: Option<String>,

    /// Onset HPO term (e.g., "HP:0003577")
    pub onset: Option<String>,

    /// Frequency (HPO term or value like "HP:0040280", "33%")
    pub frequency: Option<String>,

    /// Sex-limited phenotype ("MALE" or "FEMALE")
    pub sex: Option<String>,

    /// Modifier HPO term
    pub modifier: Option<String>,

    /// Aspect: P=phenotype, I=inheritance, C=onset/clinical, M=modifier
    pub aspect: Option<String>,

    /// Biocuration attribution
    pub biocuration: Option<String>,

    /// HPO release version
    pub hpo_release_version: String,
}

impl DiseaseAnnotation {
    /// Parse database_id like "OMIM:114500" into (disease_db, disease_id)
    pub fn parse_database_id(
        database_id: &str,
    ) -> std::result::Result<(String, String), String> {
        let (db, id) = database_id.split_once(':').ok_or_else(|| {
            format!(
                "Invalid database_id format '{}': expected 'DB:ID'",
                database_id
            )
        })?;
        Ok((db.to_string(), id.to_string()))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hpo_accession() {
        assert_eq!(HpoTerm::parse_accession("HP:0000001").unwrap(), 1);
        assert_eq!(HpoTerm::parse_accession("HP:0000118").unwrap(), 118);
        assert_eq!(HpoTerm::parse_accession("HP:9999999").unwrap(), 9999999);
        assert!(HpoTerm::parse_accession("INVALID").is_err());
        assert!(HpoTerm::parse_accession("GO:0000001").is_err());
    }

    #[test]
    fn test_validate_hpo_id() {
        assert!(HpoTerm::validate_hpo_id("HP:0000001"));
        assert!(HpoTerm::validate_hpo_id("HP:0001234"));
        assert!(HpoTerm::validate_hpo_id("HP:9999999"));
        assert!(!HpoTerm::validate_hpo_id("HP:123")); // Too short
        assert!(!HpoTerm::validate_hpo_id("HP:12345678")); // Too long
        assert!(!HpoTerm::validate_hpo_id("GO:0000001")); // Wrong prefix
        assert!(!HpoTerm::validate_hpo_id("INVALID"));
        assert!(!HpoTerm::validate_hpo_id(""));
    }

    #[test]
    fn test_hpo_term_new() {
        let term = HpoTerm::new(
            "HP:0000001".to_string(),
            "All".to_string(),
            "2026-03-01".to_string(),
        )
        .unwrap();

        assert_eq!(term.hpo_id, "HP:0000001");
        assert_eq!(term.hpo_accession, 1);
        assert_eq!(term.name, "All");
        assert!(!term.is_obsolete);
        assert!(term.synonyms.is_empty());
    }

    #[test]
    fn test_hpo_term_new_invalid_id() {
        let result = HpoTerm::new(
            "INVALID".to_string(),
            "All".to_string(),
            "2026-03-01".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_synonym_scope_from_str() {
        assert_eq!(SynonymScope::from_str("EXACT").unwrap(), SynonymScope::Exact);
        assert_eq!(SynonymScope::from_str("exact").unwrap(), SynonymScope::Exact);
        assert_eq!(SynonymScope::from_str("BROAD").unwrap(), SynonymScope::Broad);
        assert_eq!(
            SynonymScope::from_str("NARROW").unwrap(),
            SynonymScope::Narrow
        );
        assert_eq!(
            SynonymScope::from_str("RELATED").unwrap(),
            SynonymScope::Related
        );
        assert!(SynonymScope::from_str("invalid").is_err());
    }

    #[test]
    fn test_parse_database_id() {
        let (db, id) = DiseaseAnnotation::parse_database_id("OMIM:114500").unwrap();
        assert_eq!(db, "OMIM");
        assert_eq!(id, "114500");

        let (db, id) = DiseaseAnnotation::parse_database_id("ORPHA:68335").unwrap();
        assert_eq!(db, "ORPHA");
        assert_eq!(id, "68335");

        assert!(DiseaseAnnotation::parse_database_id("INVALID").is_err());
    }
}
