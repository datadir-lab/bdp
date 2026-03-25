// crates/bdp-mcp/src/tools/compounds.rs

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
pub struct GetCompoundParams {
    /// CHEBI ID (e.g. "CHEBI:15422") or compound name (e.g. "ATP")
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCompoundRolesParams {
    /// CHEBI ID or compound name
    pub id: String,
    /// Pagination cursor (omit for first page)
    pub cursor: Option<String>,
    /// Results per page (default 50, max 200)
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCompoundTargetsParams {
    /// CHEBI ID or compound name
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCompoundTrialsParams {
    /// CHEBI ID or compound name
    pub id: String,
}

/// Resolve input to CHEBI ID string.
/// Returns Err with a helpful message if not found or ambiguous.
async fn resolve_to_chebi_id(pool: &sqlx::PgPool, input: &str) -> Result<String, McpError> {
    match resolve::detect_id_type(input) {
        Some(resolve::CanonicalId::Chebi(id)) => Ok(id.to_string()),
        _ => {
            let matches = resolve::compounds_by_name(pool, input)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            match matches.len() {
                0 => Err(McpError::invalid_params(
                    format!("No compound matching '{input}'. Try a CHEBI ID like 'CHEBI:15422'."),
                    None,
                )),
                1 => {
                    let c = queries::get_compound_by_id(pool, matches[0].id)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?
                        .ok_or_else(|| McpError::internal_error("resolve mismatch", None))?;
                    Ok(c.chebi_id)
                },
                _ => {
                    let candidates: Vec<_> =
                        matches.iter().map(|m| json!({"name": m.name})).collect();
                    Err(McpError::invalid_params(
                        format!(
                            "Ambiguous: '{input}' matches {} compounds. Use a CHEBI ID.",
                            matches.len()
                        ),
                        Some(json!({"candidates": candidates})),
                    ))
                },
            }
        },
    }
}

pub async fn get_compound(
    pool: &sqlx::PgPool,
    params: GetCompoundParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.id);
    let chebi_id = resolve_to_chebi_id(pool, input).await?;

    let compound = queries::get_compound(pool, &chebi_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| {
            McpError::invalid_params(format!("Compound '{chebi_id}' not found"), None)
        })?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_compound",
            query_params: json!({"id": params.id}),
            dataset_versions: json!({"chebi": "latest"}),
            result_count: Some(1),
            duration_ms: Some(duration_ms),
        },
    )
    .await;

    let text = format!(
        "Compound: {} ({})\nDefinition: {}\nFormula: {}\nInChIKey: {}\nSMILES: {}\nMonoisotopic mass: {}\nCharge: {}",
        compound.name,
        compound.chebi_id,
        compound.definition.as_deref().unwrap_or("N/A"),
        compound.formula.as_deref().unwrap_or("N/A"),
        compound.inchikey.as_deref().unwrap_or("N/A"),
        compound.smiles.as_deref().unwrap_or("N/A"),
        compound.mass_mono.map(|m| m.to_string()).as_deref().unwrap_or("N/A"),
        compound.charge.map(|c| c.to_string()).as_deref().unwrap_or("N/A"),
    );

    let structured = json!({
        "chebi_id": compound.chebi_id,
        "name": compound.name,
        "definition": compound.definition,
        "formula": compound.formula,
        "inchikey": compound.inchikey,
        "smiles": compound.smiles,
        "mass_mono": compound.mass_mono,
        "charge": compound.charge,
        "_meta": {
            "datasets_used": [{"name": "chebi"}],
            "duration_ms": duration_ms
        }
    });

    let mut result = CallToolResult::success(vec![Content::text(text)]);
    result.structured_content = Some(structured);
    Ok(result)
}

pub async fn get_compound_roles(
    pool: &sqlx::PgPool,
    params: GetCompoundRolesParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.id);
    let chebi_id = resolve_to_chebi_id(pool, input).await?;
    let offset = common::decode_cursor(params.cursor.as_deref());
    let limit = common::clamp_limit(params.limit);

    let rows = queries::get_compound_roles(pool, &chebi_id, offset, limit)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_compound_roles",
            query_params: json!({"id": params.id, "offset": offset, "limit": limit}),
            dataset_versions: json!({"chebi": "latest"}),
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

    let text_lines: Vec<String> = rows
        .iter()
        .map(|r| format!("{} — {} ({})", r.chebi_id, r.name, r.relationship_type))
        .collect();

    let text = if text_lines.is_empty() {
        format!("No roles found for {chebi_id}")
    } else {
        format!("Roles for {} ({} found):\n{}", chebi_id, rows.len(), text_lines.join("\n"))
    };

    let structured = json!({
        "chebi_id": chebi_id,
        "roles": rows.iter().map(|r| json!({
            "chebi_id": r.chebi_id,
            "name": r.name,
            "relationship_type": r.relationship_type,
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

pub async fn get_compound_targets(
    pool: &sqlx::PgPool,
    params: GetCompoundTargetsParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.id);
    let chebi_id = resolve_to_chebi_id(pool, input).await?;

    let compound_uuid = queries::compound_uuid_by_chebi_id(pool, &chebi_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| {
            McpError::invalid_params(format!("Compound '{chebi_id}' not found"), None)
        })?;

    let offset = 0i64;
    let limit = 50i64;
    let rows = queries::get_compound_targets(pool, compound_uuid, limit, offset)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_compound_targets",
            query_params: json!({"id": params.id}),
            dataset_versions: json!({"chembl": "latest"}),
            result_count: Some(rows.len() as i32),
            duration_ms: Some(duration_ms),
        },
    )
    .await;

    let count = rows.len();
    let text = if rows.is_empty() {
        format!("No drug targets found for {chebi_id}")
    } else {
        format!("Drug targets for {} ({} found)", chebi_id, count)
    };

    let structured = json!({
        "chebi_id": chebi_id,
        "targets": rows,
        "pagination": {
            "offset": offset,
            "limit": limit,
            "count": count,
        },
        "_meta": {"duration_ms": duration_ms}
    });

    let mut result = CallToolResult::success(vec![Content::text(text)]);
    result.structured_content = Some(structured);
    Ok(result)
}

pub fn get_compound_trials_stub() -> CallToolResult {
    common::stub_result("get_compound_trials", "Requires ClinicalTrials.gov pipeline.", "BDP-83")
}
