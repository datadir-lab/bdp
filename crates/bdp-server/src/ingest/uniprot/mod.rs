//! UniProt data ingestion module
//!
//! This module provides functionality to download and parse UniProt data from FTP.
//!
//! # Features
//! - Parse UniProt DAT (flat file) format
//! - Download from UniProt FTP server
//! - Handle gzip compression
//! - Generate FASTA and JSON outputs
//!
//! # Example
//! ```no_run
//! use bdp_server::ingest::uniprot::{config::UniProtFtpConfig, parser::DatParser, ftp::UniProtFtp};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Configure UniProt source
//! let config = UniProtFtpConfig::new().with_parse_limit(100);
//!
//! // Download from FTP
//! let ftp = UniProtFtp::new(config.clone());
//! let release_notes = ftp.download_release_notes("2024_01").await?;
//! let release_info = ftp.parse_release_notes(&release_notes)?;
//!
//! // Parse DAT file
//! let parser = DatParser::with_limit(100);
//! let dat_data = ftp.download_dat_file("2024_01").await?;
//! let entries = parser.parse_bytes(&dat_data)?;
//!
//! // Generate FASTA
//! for entry in &entries {
//!     let fasta = entry.to_fasta();
//!     println!("{}", fasta);
//! }
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod ftp;
pub mod models;
pub mod parser;
pub mod pipeline;
pub mod storage;
pub mod parser_adapter;
pub mod storage_adapter;
pub mod version_discovery;

// Re-export commonly used types
pub use config::{ReleaseType, UniProtFtpConfig};
pub use ftp::UniProtFtp;
pub use models::{LicenseInfo, ReleaseInfo, UniProtEntry};
pub use parser::DatParser;
pub use pipeline::UniProtPipeline;
pub use storage::UniProtStorage;
pub use parser_adapter::{UniProtParser, UniProtFormatter};
pub use storage_adapter::UniProtStorageAdapter;
pub use version_discovery::{DiscoveredVersion, VersionDiscovery};
