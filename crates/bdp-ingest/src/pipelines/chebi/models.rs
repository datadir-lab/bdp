// crates/bdp-ingest/src/pipelines/chebi/models.rs

#[derive(Debug, Clone, Default)]
pub struct CompoundTerm {
    pub chebi_id: String, // "CHEBI:33709"
    pub chebi_accession: i64,
    pub name: String,
    pub definition: Option<String>,
    pub comment: Option<String>,
    pub is_obsolete: bool,
    pub inchikey: Option<String>,
    pub smiles: Option<String>,
    pub inchi: Option<String>,
    pub formula: Option<String>,
    pub mass_mono: Option<f64>,
    pub charge: Option<i32>,
    pub chebi_release: String,
}

#[derive(Debug, Clone)]
pub struct CompoundRelationship {
    pub subject_chebi_id: String,
    pub object_chebi_id: String,
    pub relationship_type: String,
    pub chebi_release: String,
}

#[derive(Debug, Default)]
pub struct ParsedChebi {
    pub terms: Vec<CompoundTerm>,
    pub relationships: Vec<CompoundRelationship>,
}
