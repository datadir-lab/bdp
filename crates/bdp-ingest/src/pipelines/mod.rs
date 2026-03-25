// crates/bdp-ingest/src/pipelines/mod.rs
//
// Pipeline registry — add new pipelines here.
// Each submodule implements PipelineRunner.
//
// Current pipelines:
//   gene_ontology — downloads and parses go-basic.obo
//
// Existing pipelines (in bdp-server, pending migration):
//   uniprot, ncbi_taxonomy, genbank, interpro

pub mod chebi;
pub mod gene_ontology;
pub mod reactome;

// Uncomment as pipelines are migrated from bdp-server:
// pub mod mondo;
// pub mod hpo;
