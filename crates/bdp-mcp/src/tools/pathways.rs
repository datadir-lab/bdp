// crates/bdp-mcp/src/tools/pathways.rs

use rmcp::{
    model::{CallToolResult, Content},
    schemars, ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
use std::time::Instant;

use crate::db::{audit, queries, resolve};
use crate::tools::common;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetPathwayParams {
    /// Reactome ID (e.g. "R-HSA-109581") or pathway name
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetPathwayProteinsParams {
    /// Reactome ID (e.g. "R-HSA-109581") or pathway name
    pub id: String,
    /// Pagination cursor (omit for first page)
    pub cursor: Option<String>,
    /// Results per page (default 50, max 200)
    pub limit: Option<i64>,
}

/// Resolve input to a Reactome ID string.
/// Returns Err with a helpful message if not found or ambiguous.
async fn resolve_to_reactome_id(pool: &sqlx::PgPool, input: &str) -> Result<String, McpError> {
    match resolve::detect_id_type(input) {
        Some(resolve::CanonicalId::Reactome(id)) => Ok(id.to_string()),
        _ => {
            let matches = resolve::pathways_by_name(pool, input)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            match matches.len() {
                0 => Err(McpError::invalid_params(
                    format!(
                        "No pathway matching '{input}'. Try a Reactome ID like 'R-HSA-109581'."
                    ),
                    None,
                )),
                1 => {
                    let pathway = queries::get_pathway_by_id(pool, matches[0].id)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?
                        .ok_or_else(|| McpError::internal_error("resolve mismatch", None))?;
                    Ok(pathway.reactome_id)
                },
                _ => {
                    let candidates: Vec<_> =
                        matches.iter().map(|m| json!({"name": m.name})).collect();
                    Err(McpError::invalid_params(
                        format!(
                            "Ambiguous: '{input}' matches {} pathways. Use a Reactome ID.",
                            matches.len()
                        ),
                        Some(json!({"candidates": candidates})),
                    ))
                },
            }
        },
    }
}

pub async fn get_pathway(
    pool: &sqlx::PgPool,
    params: GetPathwayParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.id);
    let reactome_id = resolve_to_reactome_id(pool, input).await?;

    let pathway = queries::get_pathway(pool, &reactome_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| {
            McpError::invalid_params(format!("Pathway '{reactome_id}' not found"), None)
        })?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_pathway",
            query_params: json!({"id": params.id}),
            dataset_versions: json!({"reactome": pathway.reactome_release}),
            result_count: Some(1),
            duration_ms: Some(duration_ms),
        },
    )
    .await;

    let text = format!(
        "Pathway: {} ({})\nSpecies: {}\nTop-level: {}",
        pathway.name, pathway.reactome_id, pathway.species_name, pathway.is_top_level,
    );

    let structured = json!({
        "reactome_id": pathway.reactome_id,
        "name": pathway.name,
        "species_name": pathway.species_name,
        "is_top_level": pathway.is_top_level,
        "_meta": {
            "datasets_used": [{"name": "reactome", "release": pathway.reactome_release}],
            "duration_ms": duration_ms
        }
    });

    let mut result = CallToolResult::success(vec![Content::text(text)]);
    result.structured_content = Some(structured);
    Ok(result)
}

pub async fn get_pathway_proteins(
    pool: &sqlx::PgPool,
    params: GetPathwayProteinsParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.id);
    let reactome_id = resolve_to_reactome_id(pool, input).await?;
    let offset = common::decode_cursor(params.cursor.as_deref());
    let limit = common::clamp_limit(params.limit);

    let rows = queries::get_pathway_proteins(pool, &reactome_id, offset, limit)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_pathway_proteins",
            query_params: json!({"id": params.id, "offset": offset, "limit": limit}),
            dataset_versions: json!({"reactome": "latest"}),
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

    let text_lines: Vec<String> =
        rows.iter()
            .map(|r| {
                format!(
                    "{} (evidence: {})",
                    r.uniprot_acc,
                    r.evidence_type.as_deref().unwrap_or("N/A"),
                )
            })
            .collect();

    let text = if text_lines.is_empty() {
        format!("No proteins found for pathway {reactome_id}")
    } else {
        format!("Proteins in {} ({} found):\n{}", reactome_id, rows.len(), text_lines.join("\n"))
    };

    let structured = json!({
        "reactome_id": reactome_id,
        "proteins": rows.iter().map(|r| json!({
            "uniprot_acc": r.uniprot_acc,
            "evidence_type": r.evidence_type,
        })).collect::<Vec<_>>(),
        "pagination": {
            "offset": offset,
            "limit": limit,
            "count": rows.len(),
            "next_cursor": next_cursor,
        },
        "_meta": {"duration_ms": duration_ms}
    });

    let mut result = CallToolResult::success(vec![Content::text(text)]);
    result.structured_content = Some(structured);
    Ok(result)
}
