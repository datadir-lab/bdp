pub mod models;
pub mod parser;
pub mod runner;
pub mod storage;
pub use runner::{ReactomeConfig, ReactomePipelineRunner};

pub const REACTOME_PATHWAYS_URL: &str =
    "https://reactome.org/download/current/ReactomePathways.txt";
pub const REACTOME_UNIPROT_URL: &str =
    "https://reactome.org/download/current/UniProt2Reactome.txt";
// Human-only mapping (smaller, faster for initial testing):
pub const REACTOME_UNIPROT_HUMAN_URL: &str =
    "https://reactome.org/download/current/UniProt2Reactome_All_Levels.txt";
