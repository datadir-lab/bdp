// crates/bdp-mcp/src/tools/literature.rs

use rmcp::{
    model::{CallToolResult, Content},
    schemars, ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::time::Instant;

use crate::db::{audit, queries};
use crate::tools::common;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchLiteratureParams {
    /// Query string or entity (e.g. "BRCA1 breast cancer")
    pub query: String,
    /// Pagination cursor
    pub cursor: Option<String>,
    /// Results per page (default 50, max 200)
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetPublicationParams {
    /// PubMed ID as integer (e.g. 12345678) or prefixed (e.g. "PMID:12345678")
    pub id: String,
}

pub async fn search_literature(
    pool: &sqlx::PgPool,
    params: SearchLiteratureParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let offset = common::decode_cursor(params.cursor.as_deref());
    let limit = common::clamp_limit(params.limit);

    let rows = queries::search_literature(pool, &params.query, limit, offset)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "search_literature",
            query_params: json!({"query": params.query, "offset": offset, "limit": limit}),
            dataset_versions: json!({"pubmed": "latest"}),
            result_count: Some(rows.len() as i32),
            duration_ms: Some(duration_ms),
        },
    )
    .await;

    let next_cursor = if rows.len() == limit as usize {
        Some(common::encode_cursor(offset + limit))
    } else {
        None
    };

    let count = rows.len();
    let text = if rows.is_empty() {
        format!("No publications found for query '{}'", params.query)
    } else {
        format!("Publications for '{}' ({} found)", params.query, count)
    };

    let structured = json!({
        "query": params.query,
        "publications": rows,
        "pagination": {
            "offset": offset,
            "limit": limit,
            "count": count,
            "next_cursor": next_cursor,
        },
        "_meta": {"duration_ms": duration_ms}
    });

    let mut result = CallToolResult::success(vec![Content::text(text)]);
    result.structured_content = Some(structured);
    Ok(result)
}

/// Parse a PMID from various input formats ("PMID:12345678", "12345678").
fn parse_pmid(id: &str) -> Option<i32> {
    let stripped = id.strip_prefix("PMID:").unwrap_or(id);
    stripped.trim().parse::<i32>().ok()
}

pub async fn get_publication(
    pool: &sqlx::PgPool,
    params: GetPublicationParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();

    let pmid = parse_pmid(&params.id).ok_or_else(|| {
        McpError::invalid_params(
            format!(
                "Invalid PubMed ID '{}'. Use an integer like '12345678' or 'PMID:12345678'.",
                params.id
            ),
            None,
        )
    })?;

    let pub_data = queries::get_publication(pool, pmid)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| McpError::invalid_params(format!("Publication PMID:{pmid} not found"), None))?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_publication",
            query_params: json!({"id": params.id}),
            dataset_versions: json!({"pubmed": "latest"}),
            result_count: Some(1),
            duration_ms: Some(duration_ms),
        },
    )
    .await;

    let title = pub_data["title"].as_str().unwrap_or("N/A");
    let journal = pub_data["journal"].as_str().unwrap_or("N/A");
    let pub_year = pub_data["pub_year"].as_i64().map(|y| y.to_string()).unwrap_or_else(|| "N/A".to_string());
    let text = format!(
        "Publication PMID:{pmid}\nTitle: {title}\nJournal: {journal}\nYear: {pub_year}"
    );

    let mut result = CallToolResult::success(vec![Content::text(text)]);
    result.structured_content = Some(pub_data);
    Ok(result)
}
