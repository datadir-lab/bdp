// MONDO Disease Ontology pipeline for bdp-ingest.
pub mod models;
pub mod parser;
pub mod runner;
pub mod storage;

pub use runner::{MondoConfig, MondoPipelineRunner};
pub const MONDO_OBO_URL: &str = "https://purl.obolibrary.org/obo/mondo.obo";
