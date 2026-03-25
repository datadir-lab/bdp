use anyhow::Result;
use tracing::info;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("bdp_mcp=debug".parse()?),
        )
        .init();

    info!("bdp-mcp starting (stub — implementation in progress)");

    // TODO: will be replaced in Task 8 (stdio transport)
    Ok(())
}
