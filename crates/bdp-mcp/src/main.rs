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
        },
        bdp_mcp::config::Transport::Http => {
            use rmcp::transport::streamable_http_server::{
                session::local::LocalSessionManager, StreamableHttpServerConfig,
                StreamableHttpService,
            };
            use std::sync::Arc;

            let session_manager = Arc::new(LocalSessionManager::default());
            let config = StreamableHttpServerConfig::default();
            let cancellation_token = config.cancellation_token.clone();

            let service: StreamableHttpService<bdp_mcp::server::BdpMcpServer, LocalSessionManager> =
                StreamableHttpService::new(
                    {
                        let pool = pool.clone();
                        move || Ok(bdp_mcp::server::BdpMcpServer::new(pool.clone()))
                    },
                    session_manager,
                    config,
                );

            let app = axum::Router::new()
                .nest_service("/mcp", service)
                .route("/health", axum::routing::get(|| async { axum::http::StatusCode::OK }));

            let bind_addr = format!("0.0.0.0:{}", cfg.port);
            let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
            tracing::info!(addr = %bind_addr, "bdp-mcp HTTP listening");

            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    cancellation_token.cancelled().await;
                })
                .await
                .map_err(|e| anyhow::anyhow!("HTTP server error: {e}"))?;
        },
    }

    Ok(())
}
