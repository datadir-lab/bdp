// MONDO Disease Ontology Models

/// A single disease term from MONDO OBO.
#[derive(Debug, Clone)]
pub struct DiseaseTerm {
    pub mondo_id: String,     // "MONDO:0004992"
    pub mondo_accession: i64, // 4992
    pub name: String,
    pub definition: Option<String>,
    pub is_obsolete: bool,
    pub comment: Option<String>,
    pub omim_id: Option<String>,     // first OMIM xref, if any
    pub orphanet_id: Option<String>, // first ORPHA xref, if any
    pub synonyms: Vec<DiseaseSynonym>,
    pub xrefs: Vec<DiseaseXref>,
    pub mondo_release: String,
}

/// A synonym for a disease term.
#[derive(Debug, Clone)]
pub struct DiseaseSynonym {
    pub scope: String, // "EXACT", "BROAD", "NARROW", "RELATED"
    pub text: String,
}

/// A cross-reference for a disease term.
#[derive(Debug, Clone)]
pub struct DiseaseXref {
    pub source_db: String, // "OMIM", "ORPHA", "MeSH", etc.
    pub source_id: String,
}

/// Relationship type between disease terms.
#[derive(Debug, Clone, PartialEq)]
pub enum DiseaseRelationType {
    IsA,
    PartOf,
    SubClassOf,
    Other(String),
}

impl DiseaseRelationType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::IsA => "is_a",
            Self::SubClassOf => "subClassOf",
            Self::PartOf => "part_of",
            Self::Other(s) => s,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "is_a" => Self::IsA,
            "subClassOf" => Self::SubClassOf,
            "part_of" => Self::PartOf,
            other => Self::Other(other.to_string()),
        }
    }
}

/// A hierarchical relationship between two disease terms.
#[derive(Debug, Clone)]
pub struct DiseaseRelationship {
    pub subject_mondo_id: String, // child
    pub object_mondo_id: String,  // parent
    pub relationship_type: DiseaseRelationType,
    pub mondo_release: String,
}

/// Result of parsing a MONDO OBO file.
#[derive(Debug, Default)]
pub struct ParsedMondo {
    pub terms: Vec<DiseaseTerm>,
    pub relationships: Vec<DiseaseRelationship>,
}

impl ParsedMondo {
    pub fn term_count(&self) -> usize {
        self.terms.len()
    }

    pub fn relationship_count(&self) -> usize {
        self.relationships.len()
    }

    pub fn obsolete_count(&self) -> usize {
        self.terms.iter().filter(|t| t.is_obsolete).count()
    }
}
