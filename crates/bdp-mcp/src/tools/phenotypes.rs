// crates/bdp-mcp/src/tools/phenotypes.rs

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
pub struct GetPhenotypeParams {
    /// HPO ID (e.g. "HP:0001250") or phenotype name (e.g. "Seizure")
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetPhenotypeDiseasesParams {
    /// HPO ID or phenotype name
    pub id: String,
    /// Pagination cursor (omit for first page)
    pub cursor: Option<String>,
    /// Results per page (default 50, max 200)
    pub limit: Option<i64>,
}

/// Resolve input to HPO ID string.
/// Returns Err with a helpful message if not found or ambiguous.
async fn resolve_to_hpo_id(pool: &sqlx::PgPool, input: &str) -> Result<String, McpError> {
    match resolve::detect_id_type(input) {
        Some(resolve::CanonicalId::Hpo(id)) => Ok(id.to_string()),
        _ => {
            let matches = resolve::phenotypes_by_name(pool, input)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            match matches.len() {
                0 => Err(McpError::invalid_params(
                    format!("No phenotype matching '{input}'. Try an HPO ID like 'HP:0001250'."),
                    None,
                )),
                1 => {
                    let p = queries::get_phenotype_by_id(pool, matches[0].id)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?
                        .ok_or_else(|| McpError::internal_error("resolve mismatch", None))?;
                    Ok(p.hpo_id)
                },
                _ => {
                    let candidates: Vec<_> =
                        matches.iter().map(|m| json!({"name": m.name})).collect();
                    Err(McpError::invalid_params(
                        format!(
                            "Ambiguous: '{input}' matches {} phenotypes. Use an HPO ID.",
                            matches.len()
                        ),
                        Some(json!({"candidates": candidates})),
                    ))
                },
            }
        },
    }
}

pub async fn get_phenotype(
    pool: &sqlx::PgPool,
    params: GetPhenotypeParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.id);
    let hpo_id = resolve_to_hpo_id(pool, input).await?;

    let phenotype = queries::get_phenotype(pool, &hpo_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| McpError::invalid_params(format!("Phenotype '{hpo_id}' not found"), None))?;

    let synonyms: Vec<String> = phenotype
        .synonyms_json
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let alt_ids: Vec<String> = phenotype
        .alt_ids_json
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_phenotype",
            query_params: json!({"id": params.id}),
            dataset_versions: json!({}),
            result_count: Some(1),
            duration_ms: Some(duration_ms),
        },
    )
    .await;

    let synonym_list = synonyms.join(", ");
    let text = format!(
        "Phenotype: {} ({})\nDefinition: {}\nSynonyms: {}\nAlt IDs: {}",
        phenotype.name,
        phenotype.hpo_id,
        phenotype.definition.as_deref().unwrap_or("N/A"),
        if synonym_list.is_empty() {
            "none".to_string()
        } else {
            synonym_list
        },
        if alt_ids.is_empty() {
            "none".to_string()
        } else {
            alt_ids.join(", ")
        },
    );

    let structured = json!({
        "hpo_id": phenotype.hpo_id,
        "name": phenotype.name,
        "definition": phenotype.definition,
        "synonyms": synonyms,
        "alt_ids": alt_ids,
        "_meta": {
            "datasets_used": [{"name": "hpo"}],
            "duration_ms": duration_ms
        }
    });

    let mut result = CallToolResult::success(vec![Content::text(text)]);
    result.structured_content = Some(structured);
    Ok(result)
}

pub async fn get_phenotype_diseases(
    pool: &sqlx::PgPool,
    params: GetPhenotypeDiseasesParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.id);
    let hpo_id = resolve_to_hpo_id(pool, input).await?;
    let offset = common::decode_cursor(params.cursor.as_deref());
    let limit = common::clamp_limit(params.limit);

    let rows = queries::get_phenotype_diseases(pool, &hpo_id, offset, limit)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_phenotype_diseases",
            query_params: json!({"id": params.id, "offset": offset, "limit": limit}),
            dataset_versions: json!({}),
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
        .map(|r| format!("{} — {}", r.mondo_id, r.name,))
        .collect();

    let text = if text_lines.is_empty() {
        format!("No diseases found for {hpo_id}")
    } else {
        format!("Diseases for {} ({} found):\n{}", hpo_id, rows.len(), text_lines.join("\n"))
    };

    let structured = json!({
        "hpo_id": hpo_id,
        "diseases": rows.iter().map(|r| json!({
            "mondo_id": r.mondo_id,
            "name": r.name,
            "definition": r.definition,
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
