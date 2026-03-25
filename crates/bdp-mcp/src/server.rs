use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::tools::compounds::{
    GetCompoundParams, GetCompoundRolesParams, GetCompoundTargetsParams, GetCompoundTrialsParams,
};
use crate::tools::diseases::{
    GetDiseaseGenesParams, GetDiseaseParams, GetDiseasePhenotypesParams, GetDiseaseTrialsParams,
};
use crate::tools::genes::{
    GetGeneDiseasesParams, GetGeneLiteratureParams, GetGeneParams, GetGenePathwaysParams,
};
use crate::tools::pathways::{GetPathwayParams, GetPathwayProteinsParams};
use crate::tools::phenotypes::{GetPhenotypeDiseasesParams, GetPhenotypeParams};

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

    /// Fetch an HPO phenotype record by HPO ID or name.
    #[tool(
        description = "Fetch an HPO phenotype record by HPO ID (e.g. 'HP:0001250') or free-text name. Returns definition, synonyms, and alternate IDs."
    )]
    async fn get_phenotype(
        &self,
        Parameters(params): Parameters<GetPhenotypeParams>,
    ) -> Result<CallToolResult, McpError> {
        crate::tools::phenotypes::get_phenotype(&self.pool, params).await
    }

    /// Fetch diseases associated with a phenotype.
    #[tool(
        description = "Fetch diseases annotated with a given HPO phenotype by HPO ID or name. Returns a paginated list of associated diseases."
    )]
    async fn get_phenotype_diseases(
        &self,
        Parameters(params): Parameters<GetPhenotypeDiseasesParams>,
    ) -> Result<CallToolResult, McpError> {
        crate::tools::phenotypes::get_phenotype_diseases(&self.pool, params).await
    }

    /// Fetch a compound record by CHEBI ID or name.
    #[tool(
        description = "Fetch a compound record by CHEBI ID (e.g. 'CHEBI:15422') or free-text name. Returns definition, formula, InChIKey, SMILES, and mass."
    )]
    async fn get_compound(
        &self,
        Parameters(params): Parameters<GetCompoundParams>,
    ) -> Result<CallToolResult, McpError> {
        crate::tools::compounds::get_compound(&self.pool, params).await
    }

    /// Fetch biological roles for a compound.
    #[tool(
        description = "Fetch biological roles for a compound by CHEBI ID or name. Returns a paginated list of has_role relationships."
    )]
    async fn get_compound_roles(
        &self,
        Parameters(params): Parameters<GetCompoundRolesParams>,
    ) -> Result<CallToolResult, McpError> {
        crate::tools::compounds::get_compound_roles(&self.pool, params).await
    }

    /// [PLANNED] Fetch drug-target bioactivity data for a compound.
    #[tool(
        description = "Fetch drug-target bioactivity data for a compound. NOTE: Not yet available — requires ChEMBL pipeline (tracked: BDP-80)."
    )]
    async fn get_compound_targets(
        &self,
        Parameters(_params): Parameters<GetCompoundTargetsParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(crate::tools::compounds::get_compound_targets_stub())
    }

    /// [PLANNED] Fetch clinical trials for a compound.
    #[tool(
        description = "Fetch clinical trials for a compound. NOTE: Not yet available — requires ClinicalTrials.gov pipeline (tracked: BDP-83)."
    )]
    async fn get_compound_trials(
        &self,
        Parameters(_params): Parameters<GetCompoundTrialsParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(crate::tools::compounds::get_compound_trials_stub())
    }

    /// Fetch a gene/protein record by UniProt accession or gene symbol.
    #[tool(
        description = "Fetch a gene/protein record by UniProt accession (e.g. 'P38398') or gene symbol (e.g. 'BRCA1'). Returns entry name, organism, taxon ID, and sequence length."
    )]
    async fn get_gene(
        &self,
        Parameters(params): Parameters<GetGeneParams>,
    ) -> Result<CallToolResult, McpError> {
        crate::tools::genes::get_gene(&self.pool, params).await
    }

    /// Fetch Reactome pathways for a gene/protein.
    #[tool(
        description = "Fetch Reactome pathways associated with a gene/protein by UniProt accession or gene symbol. Returns a paginated list of pathway names and IDs."
    )]
    async fn get_gene_pathways(
        &self,
        Parameters(params): Parameters<GetGenePathwaysParams>,
    ) -> Result<CallToolResult, McpError> {
        crate::tools::genes::get_gene_pathways(&self.pool, params).await
    }

    /// [PLANNED] Fetch gene-disease associations.
    #[tool(
        description = "Fetch gene-disease associations for a gene/protein. NOTE: Not yet available — requires DisGeNET ingestion pipeline (tracked: BDP-81)."
    )]
    async fn get_gene_diseases(
        &self,
        Parameters(_params): Parameters<GetGeneDiseasesParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(crate::tools::genes::get_gene_diseases_stub())
    }

    /// [PLANNED] Fetch literature for a gene/protein.
    #[tool(
        description = "Fetch literature for a gene/protein. NOTE: Not yet available — requires PubMed ingestion pipeline (tracked: BDP-75)."
    )]
    async fn get_gene_literature(
        &self,
        Parameters(_params): Parameters<GetGeneLiteratureParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(crate::tools::genes::get_gene_literature_stub())
    }

    /// Fetch a Reactome pathway record by ID or name.
    #[tool(
        description = "Fetch a Reactome pathway record by Reactome ID (e.g. 'R-HSA-109581') or free-text name. Returns species, top-level flag, and dataset version."
    )]
    async fn get_pathway(
        &self,
        Parameters(params): Parameters<GetPathwayParams>,
    ) -> Result<CallToolResult, McpError> {
        crate::tools::pathways::get_pathway(&self.pool, params).await
    }

    /// Fetch proteins in a Reactome pathway.
    #[tool(
        description = "Fetch proteins (UniProt accessions) in a Reactome pathway by Reactome ID or name. Returns a paginated list of protein accessions and evidence types."
    )]
    async fn get_pathway_proteins(
        &self,
        Parameters(params): Parameters<GetPathwayProteinsParams>,
    ) -> Result<CallToolResult, McpError> {
        crate::tools::pathways::get_pathway_proteins(&self.pool, params).await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for BdpMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("bdp-mcp", env!("CARGO_PKG_VERSION")))
    }
}
