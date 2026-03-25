//! Gene Ontology (GO) ingestion pipeline for bdp-ingest.
//!
//! Downloads and parses `go-basic.obo` from the Gene Ontology Consortium,
//! producing typed [`GoTerm`] / [`GoRelationship`] values.
//!
//! # Pipeline stages
//!
//! 1. Download `go-basic.obo` via HTTP (retry-safe)
//! 2. Parse with the generic [`OboParser`](crate::common::obo::OboParser)
//! 3. Map each [`RawOboTerm`](crate::common::obo::RawOboTerm) to domain types
//! 4. Return [`PipelineStats`](crate::framework::PipelineStats)
//!
//! Database persistence is delegated to `bdp-server`; this crate is
//! responsible only for download + parse.

pub mod models;
pub mod parser;
pub mod runner;

pub use models::{GoRelationship, GoTerm, Namespace, RelationshipType};
pub use runner::GoPipelineRunner;

/// URL for the current GO basic OBO release.
pub const GO_BASIC_OBO_URL: &str = "https://current.geneontology.org/ontology/go-basic.obo";
