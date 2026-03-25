pub mod models;
pub mod parser;
pub mod runner;
pub mod storage;
pub use runner::{ChebiConfig, ChebiPipelineRunner};
pub const CHEBI_OBO_URL: &str = "https://ftp.ebi.ac.uk/pub/databases/chebi/ontology/chebi.obo";
