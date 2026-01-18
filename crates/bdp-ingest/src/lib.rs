//! BDP Ingest Library
//!
//! Tools for ingesting biological datasets from various sources.
//!
//! # Supported Data Sources
//!
//! - **UniProt**: Protein sequence and functional information
//! - **NCBI**: National Center for Biotechnology Information databases
//! - **Ensembl**: Genome annotation and comparative genomics
//!
//! # Example
//!
//! ```no_run
//! use bdp_ingest::uniprot;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Ingest UniProt data
//!     uniprot::ingest("./data/uniprot", Some("2024_01")).await?;
//!     Ok(())
//! }
//! ```

pub mod ensembl;
pub mod ncbi;
pub mod uniprot;
pub mod version_mapping;
