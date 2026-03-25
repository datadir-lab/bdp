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

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(cfg.db_max_connections)
        .connect(&cfg.database_url)
        .await?;

    tracing::info!(transport = ?cfg.transport, "bdp-mcp starting");

    match cfg.transport {
        bdp_mcp::config::Transport::Stdio => {
            use rmcp::ServiceExt;
            let server = bdp_mcp::server::BdpMcpServer::new(pool);
            server
                .serve(rmcp::transport::stdio())
                .await
                .map_err(|e| anyhow::anyhow!("stdio transport error: {e}"))?
                .waiting()
                .await
                .map_err(|e| anyhow::anyhow!("stdio transport join error: {e}"))?;
        }
        bdp_mcp::config::Transport::Http => {
            anyhow::bail!("HTTP transport not yet implemented — use --transport stdio");
        }
    }

    Ok(())
}
