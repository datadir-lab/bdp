// crates/bdp-mcp/src/tools/diseases.rs

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
pub struct GetDiseaseParams {
    /// MONDO ID (e.g. "MONDO:0004975") or disease name (e.g. "Alzheimer disease")
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetDiseasePhenotypesParams {
    /// MONDO ID or disease name
    pub id: String,
    /// Pagination cursor (omit for first page)
    pub cursor: Option<String>,
    /// Results per page (default 50, max 200)
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetDiseaseGenesParams {
    /// MONDO ID or disease name
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetDiseaseTrialsParams {
    /// MONDO ID or disease name
    pub id: String,
}

/// Resolve input to MONDO ID string.
/// Returns Err with a helpful message if not found or ambiguous.
async fn resolve_to_mondo_id(pool: &sqlx::PgPool, input: &str) -> Result<String, McpError> {
    match resolve::detect_id_type(input) {
        Some(resolve::CanonicalId::Mondo(id)) => Ok(id.to_string()),
        _ => {
            let matches = resolve::diseases_by_name(pool, input)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            match matches.len() {
                0 => Err(McpError::invalid_params(
                    format!("No disease matching '{input}'. Try a MONDO ID like 'MONDO:0004975'."),
                    None,
                )),
                1 => {
                    let d = queries::get_disease_by_id(pool, matches[0].id)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?
                        .ok_or_else(|| McpError::internal_error("resolve mismatch", None))?;
                    Ok(d.mondo_id)
                },
                _ => {
                    let candidates: Vec<_> =
                        matches.iter().map(|m| json!({"name": m.name})).collect();
                    Err(McpError::invalid_params(
                        format!(
                            "Ambiguous: '{input}' matches {} diseases. Use a MONDO ID.",
                            matches.len()
                        ),
                        Some(json!({"candidates": candidates})),
                    ))
                },
            }
        },
    }
}

pub async fn get_disease(
    pool: &sqlx::PgPool,
    params: GetDiseaseParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.id);
    let mondo_id = resolve_to_mondo_id(pool, input).await?;

    let disease = queries::get_disease(pool, &mondo_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| McpError::invalid_params(format!("Disease '{mondo_id}' not found"), None))?;

    let synonyms = queries::get_disease_synonyms(pool, disease.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let xrefs = queries::get_disease_xrefs(pool, disease.id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_disease",
            query_params: json!({"id": params.id}),
            dataset_versions: json!({"mondo": disease.mondo_release}),
            result_count: Some(1),
            duration_ms: Some(duration_ms),
        },
    )
    .await;

    let synonym_list = synonyms
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let text = format!(
        "Disease: {} ({})\nDefinition: {}\nOMIM: {}\nOrphanet: {}\nSynonyms: {}",
        disease.name,
        disease.mondo_id,
        disease.definition.as_deref().unwrap_or("N/A"),
        disease.omim_id.as_deref().unwrap_or("N/A"),
        disease.orphanet_id.as_deref().unwrap_or("N/A"),
        if synonym_list.is_empty() {
            "none".to_string()
        } else {
            synonym_list
        },
    );

    let structured = json!({
        "mondo_id": disease.mondo_id,
        "name": disease.name,
        "definition": disease.definition,
        "omim_id": disease.omim_id,
        "orphanet_id": disease.orphanet_id,
        "synonyms": synonyms.iter().map(|s| json!({"scope": s.scope, "text": s.text})).collect::<Vec<_>>(),
        "xrefs": xrefs.iter().map(|x| json!({"source_db": x.source_db, "source_id": x.source_id})).collect::<Vec<_>>(),
        "_meta": {
            "datasets_used": [{"name": "mondo", "release": disease.mondo_release}],
            "duration_ms": duration_ms
        }
    });

    let mut result = CallToolResult::success(vec![Content::text(text)]);
    result.structured_content = Some(structured);
    Ok(result)
}

pub async fn get_disease_phenotypes(
    pool: &sqlx::PgPool,
    params: GetDiseasePhenotypesParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.id);
    let mondo_id = resolve_to_mondo_id(pool, input).await?;
    let offset = common::decode_cursor(params.cursor.as_deref());
    let limit = common::clamp_limit(params.limit);

    let rows = queries::get_disease_phenotypes(pool, &mondo_id, offset, limit)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_disease_phenotypes",
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
        .map(|r| {
            format!(
                "{} — {} (freq: {}, evidence: {})",
                r.hpo_id,
                r.hpo_name,
                r.frequency.as_deref().unwrap_or("N/A"),
                r.evidence.as_deref().unwrap_or("N/A"),
            )
        })
        .collect();

    let text = if text_lines.is_empty() {
        format!("No phenotypes found for {mondo_id}")
    } else {
        format!("Phenotypes for {} ({} found):\n{}", mondo_id, rows.len(), text_lines.join("\n"))
    };

    let structured = json!({
        "mondo_id": mondo_id,
        "phenotypes": rows.iter().map(|r| json!({
            "hpo_id": r.hpo_id,
            "name": r.hpo_name,
            "frequency": r.frequency,
            "onset": r.onset,
            "evidence": r.evidence,
            "reference": r.reference,
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

/// Stub: gene-disease associations (requires DisGeNET pipeline).
pub fn get_disease_genes_stub() -> CallToolResult {
    common::stub_result(
        "get_disease_genes",
        "Requires DisGeNET pipeline (BDP-81). Will return gene-disease associations.",
        "BDP-81",
    )
}

pub async fn get_disease_trials(
    pool: &sqlx::PgPool,
    params: GetDiseaseTrialsParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.id);
    let mondo_id = resolve_to_mondo_id(pool, input).await?;

    let disease = queries::get_disease(pool, &mondo_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| McpError::invalid_params(format!("Disease '{mondo_id}' not found"), None))?;

    let offset = 0i64;
    let limit = 50i64;
    let rows = queries::get_disease_trials(pool, disease.id, limit, offset)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_disease_trials",
            query_params: json!({"id": params.id}),
            dataset_versions: json!({"clinicaltrials": "latest"}),
            result_count: Some(rows.len() as i32),
            duration_ms: Some(duration_ms),
        },
    )
    .await;

    let count = rows.len();
    let text = if rows.is_empty() {
        format!("No clinical trials found for {mondo_id}")
    } else {
        format!("Clinical trials for {} ({} found)", mondo_id, count)
    };

    let structured = json!({
        "mondo_id": mondo_id,
        "trials": rows,
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
