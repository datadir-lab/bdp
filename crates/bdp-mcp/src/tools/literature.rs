// crates/bdp-mcp/src/tools/literature.rs

use rmcp::model::CallToolResult;
use rmcp::schemars;
use serde::Deserialize;

use crate::tools::common;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchLiteratureParams {
    /// Query string or entity (e.g. "BRCA1 breast cancer")
    pub query: String,
    /// Pagination cursor
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetPublicationParams {
    /// PubMed ID (e.g. "PMID:12345678") or DOI
    pub id: String,
}

pub fn search_literature_stub() -> CallToolResult {
    common::stub_result(
        "search_literature",
        "Requires PubMed pipeline (BDP-84). Literature ingestion planned for 2026-Q3.",
        "BDP-84",
    )
}

pub fn get_publication_stub() -> CallToolResult {
    common::stub_result("get_publication", "Requires PubMed pipeline (BDP-84).", "BDP-84")
}
