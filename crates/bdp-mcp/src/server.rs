use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::tools::diseases::{
    GetDiseaseGenesParams, GetDiseaseParams, GetDiseasePhenotypesParams, GetDiseaseTrialsParams,
};

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

    /// Fetch a disease record by MONDO ID or name.
    #[tool(
        description = "Fetch a disease record by MONDO ID (e.g. 'MONDO:0004975') or free-text name. Returns definition, cross-references (OMIM, Orphanet), and synonyms."
    )]
    async fn get_disease(
        &self,
        Parameters(params): Parameters<GetDiseaseParams>,
    ) -> Result<CallToolResult, McpError> {
        crate::tools::diseases::get_disease(&self.pool, params).await
    }

    /// Fetch HPO phenotype annotations for a disease.
    #[tool(
        description = "Fetch HPO phenotype annotations for a disease by MONDO ID or name. Returns a paginated list of associated phenotypes with frequency, onset, and evidence."
    )]
    async fn get_disease_phenotypes(
        &self,
        Parameters(params): Parameters<GetDiseasePhenotypesParams>,
    ) -> Result<CallToolResult, McpError> {
        crate::tools::diseases::get_disease_phenotypes(&self.pool, params).await
    }

    /// [PLANNED] Fetch gene-disease associations (requires DisGeNET pipeline).
    #[tool(
        description = "Fetch gene-disease associations for a MONDO disease. NOTE: Not yet available — requires DisGeNET ingestion pipeline (tracked: BDP-81)."
    )]
    async fn get_disease_genes(
        &self,
        Parameters(_params): Parameters<GetDiseaseGenesParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(crate::tools::diseases::get_disease_genes_stub())
    }

    /// [PLANNED] Fetch active clinical trials for a disease.
    #[tool(
        description = "Fetch active clinical trials for a disease. NOTE: Not yet available — requires ClinicalTrials.gov pipeline (tracked: BDP-83)."
    )]
    async fn get_disease_trials(
        &self,
        Parameters(_params): Parameters<GetDiseaseTrialsParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(crate::tools::diseases::get_disease_trials_stub())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for BdpMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("bdp-mcp", env!("CARGO_PKG_VERSION")))
    }
}
