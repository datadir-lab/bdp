// crates/bdp-mcp/src/tools/traversal.rs

use rmcp::{model::CallToolResult, schemars, ErrorData as McpError};
use serde::Deserialize;

use crate::tools::common;
use crate::tools::compounds::GetCompoundRolesParams;
use crate::tools::diseases::GetDiseasePhenotypesParams;
use crate::tools::genes::GetGenePathwaysParams;
use crate::tools::phenotypes::GetPhenotypeDiseasesParams;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TraverseParams {
    /// Traversal path (e.g. "disease->phenotype", "gene->pathway", "phenotype->disease", "compound->role")
    pub path: String,
    /// Source entity ID or name
    pub from: String,
    /// Pagination cursor
    pub cursor: Option<String>,
    /// Results per page
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindConnectionParams {
    /// Starting entity
    pub from: String,
    /// Target entity
    pub to: String,
}

pub async fn traverse(
    pool: &sqlx::PgPool,
    params: TraverseParams,
) -> Result<CallToolResult, McpError> {
    match params.path.as_str() {
        "disease->phenotype" => {
            crate::tools::diseases::get_disease_phenotypes(
                pool,
                GetDiseasePhenotypesParams {
                    id: params.from,
                    cursor: params.cursor,
                    limit: params.limit,
                },
            )
            .await
        }
        "phenotype->disease" => {
            crate::tools::phenotypes::get_phenotype_diseases(
                pool,
                GetPhenotypeDiseasesParams {
                    id: params.from,
                    cursor: params.cursor,
                    limit: params.limit,
                },
            )
            .await
        }
        "gene->pathway" => {
            crate::tools::genes::get_gene_pathways(
                pool,
                GetGenePathwaysParams {
                    id: params.from,
                    cursor: params.cursor,
                    limit: params.limit,
                },
            )
            .await
        }
        "compound->role" => {
            crate::tools::compounds::get_compound_roles(
                pool,
                GetCompoundRolesParams {
                    id: params.from,
                    cursor: params.cursor,
                    limit: params.limit,
                },
            )
            .await
        }
        other => Ok(common::stub_result(
            "traverse",
            &format!(
                "Path '{other}' not yet supported. Available: disease->phenotype, phenotype->disease, gene->pathway, compound->role"
            ),
            "BDP-90",
        )),
    }
}

pub fn find_connection_stub() -> CallToolResult {
    common::stub_result(
        "find_connection",
        "Graph path-finding not yet implemented. Will find multi-hop connections between any two entities.",
        "BDP-90",
    )
}
