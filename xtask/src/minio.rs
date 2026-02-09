use crate::utils::*;
/// MinIO operations
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum MinioCommand {
    /// Start MinIO
    Up,
    /// Stop MinIO
    Down,
    /// MinIO logs
    Logs,
}

pub fn handle(cmd: MinioCommand) -> Result<()> {
    match cmd {
        MinioCommand::Up => up(),
        MinioCommand::Down => down(),
        MinioCommand::Logs => logs(),
    }
}

fn up() -> Result<()> {
    info("📦 Starting MinIO...");
    run("docker", &["compose", "up", "-d", "minio", "minio-init"], "Start MinIO")?;
    success("MinIO ready at http://localhost:9001");
    Ok(())
}

fn down() -> Result<()> {
    run("docker", &["compose", "down", "minio", "minio-init"], "Stop MinIO")
}

fn logs() -> Result<()> {
    run_streaming("docker", &["compose", "logs", "-f", "minio"], "MinIO logs")
}
