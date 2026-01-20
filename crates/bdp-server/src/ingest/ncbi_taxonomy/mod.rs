//! NCBI Taxonomy data ingestion module
//!
//! This module provides functionality to download and parse NCBI Taxonomy data from FTP.
//!
//! # Features
//! - Parse NCBI Taxonomy taxdump files (rankedlineage.dmp, merged.dmp, delnodes.dmp)
//! - Download from NCBI FTP server
//! - Handle gzip and tar.gz compression
//! - Generate JSON and TSV outputs
//! - Track merged and deleted taxa
//!
//! # Example
//! ```no_run
//! use bdp_server::ingest::ncbi_taxonomy::{config::NcbiTaxonomyFtpConfig, parser::TaxdumpParser, ftp::NcbiTaxonomyFtp};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Configure NCBI source
//! let config = NcbiTaxonomyFtpConfig::new();
//!
//! // Download from FTP
//! let ftp = NcbiTaxonomyFtp::new(config.clone());
//! let taxdump_data = ftp.download_taxdump().await?;
//!
//! // Parse taxdump files
//! let parser = TaxdumpParser::new();
//! let entries = parser.parse(&taxdump_data)?;
//!
//! // Generate JSON
//! for entry in &entries {
//!     let json = entry.to_json()?;
//!     println!("{}", json);
//! }
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod ftp;
pub mod models;
pub mod orchestrator;
pub mod parser;
pub mod pipeline;
pub mod storage;
pub mod version_discovery;

// Re-export commonly used types
pub use config::NcbiTaxonomyFtpConfig;
pub use ftp::{NcbiTaxonomyFtp, TaxdumpFiles};
pub use models::{DeletedTaxon, MergedTaxon, TaxdumpData, TaxdumpStats, TaxonomyEntry};
pub use orchestrator::NcbiTaxonomyOrchestrator;
pub use parser::TaxdumpParser;
pub use pipeline::{NcbiTaxonomyPipeline, PipelineResult};
pub use storage::{NcbiTaxonomyStorage, StorageStats};
pub use version_discovery::{DiscoveredTaxonomyVersion, TaxonomyVersionDiscovery};
