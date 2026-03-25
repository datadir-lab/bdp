use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_env("RUST_LOG")
                .add_directive("bdp_mcp=info".parse()?),
        )
        .init();

    let cfg = bdp_mcp::config::Config::parse();
    tracing::info!(transport = ?cfg.transport, port = cfg.port, "bdp-mcp starting");

    Ok(())
}
