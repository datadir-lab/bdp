// crates/bdp-mcp/src/tools/genes.rs

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
pub struct GetGeneParams {
    /// UniProt accession (e.g. "P38398") or gene symbol (e.g. "BRCA1")
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetGenePathwaysParams {
    /// UniProt accession (e.g. "P38398") or gene symbol (e.g. "BRCA1")
    pub id: String,
    /// Pagination cursor (omit for first page)
    pub cursor: Option<String>,
    /// Results per page (default 50, max 200)
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetGeneDiseasesParams {
    /// UniProt accession or gene symbol
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetGeneLiteratureParams {
    /// UniProt accession or gene symbol
    pub id: String,
}

/// Resolve input to a UniProt accession string.
/// Returns Err with a helpful message if not found or ambiguous.
async fn resolve_to_uniprot_acc(pool: &sqlx::PgPool, input: &str) -> Result<String, McpError> {
    match resolve::detect_id_type(input) {
        Some(resolve::CanonicalId::UniProt(acc)) => Ok(acc.to_string()),
        _ => {
            let matches = resolve::genes_by_symbol(pool, input)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            match matches.len() {
                0 => Err(McpError::invalid_params(
                    format!("No gene matching '{input}'. Try a UniProt accession like 'P38398'."),
                    None,
                )),
                1 => Ok(matches[0].accession.clone()),
                _ => {
                    let candidates: Vec<_> = matches
                        .iter()
                        .map(|m| {
                            json!({
                                "accession": m.accession,
                                "gene_name": m.gene_name
                            })
                        })
                        .collect();
                    Err(McpError::invalid_params(
                        format!(
                            "Ambiguous: '{input}' matches {} genes. Use a UniProt accession.",
                            matches.len()
                        ),
                        Some(json!({"candidates": candidates})),
                    ))
                },
            }
        },
    }
}

pub async fn get_gene(
    pool: &sqlx::PgPool,
    params: GetGeneParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.id);
    let uniprot_acc = resolve_to_uniprot_acc(pool, input).await?;

    let gene = queries::get_gene_by_uniprot(pool, &uniprot_acc)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| McpError::invalid_params(format!("Gene '{uniprot_acc}' not found"), None))?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_gene",
            query_params: json!({"id": params.id}),
            dataset_versions: json!({"uniprot": "latest"}),
            result_count: Some(1),
            duration_ms: Some(duration_ms),
        },
    )
    .await;

    let text = format!(
        "Gene: {} ({})\nEntry: {}\nOrganism: {} (taxon {})\nSequence length: {} aa",
        gene.gene_name.as_deref().unwrap_or("N/A"),
        gene.uniprot_acc,
        gene.entry_name.as_deref().unwrap_or("N/A"),
        gene.organism.as_deref().unwrap_or("N/A"),
        gene.ncbi_taxon_id
            .map(|t| t.to_string())
            .as_deref()
            .unwrap_or("N/A"),
        gene.sequence_length
            .map(|l| l.to_string())
            .as_deref()
            .unwrap_or("N/A"),
    );

    let structured = json!({
        "uniprot_acc": gene.uniprot_acc,
        "gene_name": gene.gene_name,
        "entry_name": gene.entry_name,
        "organism": gene.organism,
        "ncbi_taxon_id": gene.ncbi_taxon_id,
        "sequence_length": gene.sequence_length,
        "_meta": {
            "duration_ms": duration_ms
        }
    });

    let mut result = CallToolResult::success(vec![Content::text(text)]);
    result.structured_content = Some(structured);
    Ok(result)
}

pub async fn get_gene_pathways(
    pool: &sqlx::PgPool,
    params: GetGenePathwaysParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.id);
    let uniprot_acc = resolve_to_uniprot_acc(pool, input).await?;
    let offset = common::decode_cursor(params.cursor.as_deref());
    let limit = common::clamp_limit(params.limit);

    let rows = queries::get_gene_pathways(pool, &uniprot_acc, offset, limit)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_gene_pathways",
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

    let text_lines: Vec<String> = rows
        .iter()
        .map(|r| {
            format!(
                "{} — {} ({}){}",
                r.reactome_id,
                r.name,
                r.species_name,
                if r.is_top_level { " [top-level]" } else { "" },
            )
        })
        .collect();

    let text = if text_lines.is_empty() {
        format!("No Reactome pathways found for {uniprot_acc}")
    } else {
        format!(
            "Reactome pathways for {} ({} found):\n{}",
            uniprot_acc,
            rows.len(),
            text_lines.join("\n")
        )
    };

    let structured = json!({
        "uniprot_acc": uniprot_acc,
        "pathways": rows.iter().map(|r| json!({
            "reactome_id": r.reactome_id,
            "name": r.name,
            "species_name": r.species_name,
            "is_top_level": r.is_top_level,
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

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetGeneInteractionsParams {
    /// UniProt accession (e.g. "P04637")
    pub gene: String,
    /// Minimum combined STRING score (0-1000, default 400)
    pub min_score: Option<i16>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

pub async fn get_gene_diseases(
    pool: &sqlx::PgPool,
    params: GetGeneDiseasesParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.id);
    let uniprot_acc = resolve_to_uniprot_acc(pool, input).await?;

    let gene_uuid = queries::resolve_gene_uuid(pool, &uniprot_acc)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| {
            McpError::invalid_params(
                format!("Gene '{uniprot_acc}' not found in data_sources"),
                None,
            )
        })?;

    let offset = 0i64;
    let limit = 50i64;
    let rows = queries::get_gene_diseases(pool, gene_uuid, limit, offset)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_gene_diseases",
            query_params: json!({"id": params.id}),
            dataset_versions: json!({"disgenet": "latest"}),
            result_count: Some(rows.len() as i32),
            duration_ms: Some(duration_ms),
        },
    )
    .await;

    let count = rows.len();
    let text = if rows.is_empty() {
        format!("No disease associations found for {uniprot_acc}")
    } else {
        format!("Disease associations for {} ({} found)", uniprot_acc, count)
    };

    let structured = json!({
        "uniprot_acc": uniprot_acc,
        "diseases": rows,
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

pub async fn get_gene_interactions(
    pool: &sqlx::PgPool,
    params: GetGeneInteractionsParams,
) -> Result<CallToolResult, McpError> {
    let start = Instant::now();
    let input = resolve::cap_input(&params.gene);
    let uniprot_acc = resolve_to_uniprot_acc(pool, input).await?;

    let gene_uuid = queries::resolve_gene_uuid(pool, &uniprot_acc)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| {
            McpError::invalid_params(
                format!("Gene '{uniprot_acc}' not found in data_sources"),
                None,
            )
        })?;

    let offset = common::decode_cursor(params.cursor.as_deref());
    let limit = common::clamp_limit(params.limit);
    let min_score = params.min_score.unwrap_or(400);

    let rows = queries::get_gene_interactions(pool, gene_uuid, min_score, limit, offset)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let duration_ms = start.elapsed().as_millis() as i32;

    audit::log_tool_call(
        pool,
        audit::AuditEntry {
            agent_id: None,
            tool_name: "get_gene_interactions",
            query_params: json!({"gene": params.gene, "min_score": min_score, "offset": offset, "limit": limit}),
            dataset_versions: json!({"string": "latest"}),
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
        format!("No protein interactions found for {uniprot_acc} with min_score >= {min_score}")
    } else {
        format!(
            "Protein interactions for {} ({} found, min_score >= {}):",
            uniprot_acc, count, min_score
        )
    };

    let structured = json!({
        "uniprot_acc": uniprot_acc,
        "interactions": rows,
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

/// Stub: gene literature (requires PubMed pipeline).
pub fn get_gene_literature_stub() -> CallToolResult {
    common::stub_result("get_gene_literature", "Requires PubMed pipeline (BDP-75).", "BDP-75")
}
