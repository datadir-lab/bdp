// GenBank/RefSeq ingestion module
//
// This module handles ingestion of nucleotide sequences from NCBI GenBank and RefSeq databases.
// Storage: S3 for sequences (FASTA), PostgreSQL for metadata
// Performance: Batch operations (500 chunks) + parallel processing

pub mod config;
pub mod ftp;
pub mod models;
pub mod orchestrator;
pub mod parser;
pub mod pipeline;
pub mod storage;
pub mod version_discovery;

pub use config::GenbankFtpConfig;
pub use models::{CdsFeature, Feature, GenbankRecord, SourceFeature};
pub use orchestrator::GenbankOrchestrator;
pub use parser::GenbankParser;
pub use pipeline::GenbankPipeline;
pub use storage::GenbankStorage;
pub use version_discovery::{DiscoveredVersion, VersionDiscovery};
