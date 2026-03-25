// crates/bdp-mcp/src/db/audit.rs
//
// Audit logging for MCP tool calls.
// Writes to agent_query_log — failures are swallowed (warn-logged) so they
// never propagate to the caller.

use serde_json::Value;
use sqlx::PgPool;
use tracing::warn;

pub struct AuditEntry<'a> {
    pub agent_id: Option<&'a str>,
    pub tool_name: &'a str,
    pub query_params: Value,
    pub dataset_versions: Value,
    pub result_count: Option<i32>,
    pub duration_ms: Option<i32>,
}

/// Write a tool call to agent_query_log.
/// If the INSERT fails, logs a warning and returns — never propagates to the caller.
pub async fn log_tool_call(pool: &PgPool, entry: AuditEntry<'_>) {
    let result = sqlx::query(
        "INSERT INTO agent_query_log
         (agent_id, tool_name, query_params, dataset_versions, result_count, duration_ms)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(entry.agent_id.unwrap_or("anonymous"))
    .bind(entry.tool_name)
    .bind(&entry.query_params)
    .bind(&entry.dataset_versions)
    .bind(entry.result_count)
    .bind(entry.duration_ms)
    .execute(pool)
    .await;

    if let Err(e) = result {
        warn!(
            tool = entry.tool_name,
            error = %e,
            "audit write failed — tool result unaffected"
        );
    }
}
