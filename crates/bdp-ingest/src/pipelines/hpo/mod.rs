// Human Phenotype Ontology pipeline for bdp-ingest.
pub mod models;
pub mod parser;
pub mod runner;
pub mod storage;

pub use runner::{HpoConfig, HpoPipelineRunner};

pub const HPO_OBO_URL: &str =
    "https://github.com/obophenotype/human-phenotype-ontology/releases/latest/download/hp.obo";
pub const HPO_HPOA_URL: &str =
    "https://github.com/obophenotype/human-phenotype-ontology/releases/latest/download/phenotype.hpoa";

pub const DEFAULT_TERM_CHUNK_SIZE: usize = 500;
pub const DEFAULT_RELATIONSHIP_CHUNK_SIZE: usize = 500;
pub const DEFAULT_ANNOTATION_CHUNK_SIZE: usize = 1000;
