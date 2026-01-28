// Gene Ontology (GO) Ingestion Module
//
// This module handles the ingestion of Gene Ontology terms, relationships, and annotations
// from the Gene Ontology Consortium (http://geneontology.org/).
//
// GO provides functional annotations for proteins and genes across three ontologies:
// - Biological Process (BP)
// - Molecular Function (MF)
// - Cellular Component (CC)
//
// Architecture follows the BDP pattern used in NCBI Taxonomy and GenBank:
// - Download: HTTP client for OBO/GAF files
// - Parse: OBO format parser + GAF format parser
// - Store: Batch operations to PostgreSQL
// - Pipeline: Orchestration workflow
//
// Data sources:
// - GO Ontology: http://release.geneontology.org/{version}/ontology/go-basic.obo (~40MB)
// - GOA Annotations: http://geneontology.org/gene-associations/goa_uniprot_all.gaf.gz (~2GB)

pub mod config;
pub mod downloader;
pub mod models;
pub mod parser;
pub mod pipeline;
pub mod storage;
pub mod version_discovery;

// Re-export main types
pub use config::GoHttpConfig;
pub use downloader::GoDownloader;
pub use models::{
    EntityType, EvidenceCode, GoAnnotation, GoRelationship, GoTerm, Namespace, RelationshipType,
    Synonym, SynonymScope,
};
pub use parser::{GafParser, GoParser, OboParser, ParsedObo};
pub use pipeline::{GoPipeline, PipelineStats};
pub use storage::{GoStorage, StorageStats};
pub use version_discovery::{DiscoveredVersion, VersionDiscovery};

// Batch size constants
pub const DEFAULT_TERM_CHUNK_SIZE: usize = 500;
pub const DEFAULT_RELATIONSHIP_CHUNK_SIZE: usize = 500;
pub const DEFAULT_ANNOTATION_CHUNK_SIZE: usize = 1000;

/// Result type for GO operations
pub type Result<T> = std::result::Result<T, GoError>;

/// Error types for GO ingestion
#[derive(Debug, thiserror::Error)]
pub enum GoError {
    #[error("Download error: {0}")]
    Download(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Version error: {0}")]
    Version(String),

    #[error("Decompression error: {0}")]
    Decompression(String),
}

impl From<regex::Error> for GoError {
    fn from(err: regex::Error) -> Self {
        GoError::Parse(err.to_string())
    }
}

impl From<std::num::ParseIntError> for GoError {
    fn from(err: std::num::ParseIntError) -> Self {
        GoError::Parse(err.to_string())
    }
}
