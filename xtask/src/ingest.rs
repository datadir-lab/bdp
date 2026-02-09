use crate::utils::*;
/// Data ingestion
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum IngestCommand {
    /// Run UniProt ingestion
    Uniprot,
    /// Run NCBI ingestion (future)
    Ncbi,
    /// Run all ingestion
    All,
}

pub fn handle(cmd: IngestCommand) -> Result<()> {
    match cmd {
        IngestCommand::Uniprot => uniprot(),
        IngestCommand::Ncbi => ncbi(),
        IngestCommand::All => all(),
    }
}

fn uniprot() -> Result<()> {
    info("🔬 Starting UniProt ingestion...");
    run_streaming("cargo", &["run", "--bin", "bdp-ingest", "--", "uniprot"], "UniProt ingestion")
}

fn ncbi() -> Result<()> {
    info("🔬 Starting NCBI ingestion...");
    run_streaming("cargo", &["run", "--bin", "bdp-ingest", "--", "ncbi"], "NCBI ingestion")
}

fn all() -> Result<()> {
    uniprot()?;
    success("All ingestion complete");
    Ok(())
}
