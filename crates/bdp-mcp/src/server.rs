use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use serde::Deserialize;
use sqlx::PgPool;

/// The BDP MCP server. Holds a database connection pool and implements the MCP
/// `ServerHandler` trait via the rmcp macros.
#[derive(Clone)]
pub struct BdpMcpServer {
    pool: Arc<PgPool>,
    tool_router: ToolRouter<Self>,
}

impl BdpMcpServer {
    /// Create a new server from an existing pool.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
            tool_router: Self::tool_router(),
        }
    }

    /// Return a reference to the underlying pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// Parameters for the `ping` tool (none required).
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PingParams {}

#[tool_router(router = tool_router)]
impl BdpMcpServer {
    /// Verify the server is alive and reachable.
    #[tool(description = "Ping the BDP MCP server to verify it is alive.")]
    async fn ping(&self, Parameters(_params): Parameters<PingParams>) -> String {
        "pong".to_string()
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for BdpMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("bdp-mcp", env!("CARGO_PKG_VERSION")))
    }
}
