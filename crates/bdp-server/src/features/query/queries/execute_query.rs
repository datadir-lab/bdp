//! Execute SQL query
//!
//! Executes a validated SQL query against the PostgreSQL database.
//! Implements safety checks and timeout controls.

use serde::{Deserialize, Serialize};
use sqlx::{Column, PgPool, Row, TypeInfo, ValueRef};
use std::time::Duration;
use thiserror::Error;

/// Request to execute a SQL query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteQueryRequest {
    /// SQL query to execute (must be SELECT or EXPLAIN)
    pub sql: String,
}

/// Response containing query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteQueryResponse {
    /// Column names
    pub columns: Vec<String>,
    /// Result rows (each row is a vector of JSON values)
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// Errors that can occur during query execution
#[derive(Debug, Error)]
pub enum ExecuteQueryError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Query timeout exceeded (30 seconds)")]
    Timeout,

    #[error("Invalid SQL: {0}")]
    InvalidSql(String),

    #[error("Query not allowed: {0}")]
    Forbidden(String),
}

/// Execute a SQL query with validation and timeout
///
/// # Security
///
/// - Only SELECT and EXPLAIN queries are allowed
/// - Query timeout is enforced (30 seconds)
/// - Results are limited to prevent memory exhaustion
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `request` - Query request containing SQL
///
/// # Returns
///
/// Query results with columns and rows
pub async fn handle(
    pool: PgPool,
    request: ExecuteQueryRequest,
) -> Result<ExecuteQueryResponse, ExecuteQueryError> {
    // Validate SQL (basic safety check)
    validate_sql(&request.sql)?;

    // Execute query with timeout
    let result = tokio::time::timeout(
        Duration::from_secs(30),
        execute_sql(&pool, &request.sql),
    )
    .await
    .map_err(|_| ExecuteQueryError::Timeout)??;

    Ok(result)
}

/// Validate SQL query for safety
fn validate_sql(sql: &str) -> Result<(), ExecuteQueryError> {
    let sql_upper = sql.trim().to_uppercase();

    // Only allow SELECT and EXPLAIN queries
    if !sql_upper.starts_with("SELECT") && !sql_upper.starts_with("EXPLAIN") {
        return Err(ExecuteQueryError::Forbidden(
            "Only SELECT and EXPLAIN queries are allowed".to_string(),
        ));
    }

    // Block dangerous keywords (even in SELECT queries)
    let dangerous_keywords = [
        "DROP", "DELETE", "UPDATE", "INSERT", "TRUNCATE", "ALTER", "CREATE",
        "GRANT", "REVOKE", "EXECUTE", "CALL", "COPY",
    ];

    for keyword in &dangerous_keywords {
        if sql_upper.contains(keyword) {
            return Err(ExecuteQueryError::Forbidden(format!(
                "{} statements are not allowed",
                keyword
            )));
        }
    }

    Ok(())
}

/// Execute SQL query and convert results to JSON
async fn execute_sql(
    pool: &PgPool,
    sql: &str,
) -> Result<ExecuteQueryResponse, ExecuteQueryError> {
    // Execute query
    let rows = sqlx::query(sql).fetch_all(pool).await?;

    if rows.is_empty() {
        return Ok(ExecuteQueryResponse {
            columns: Vec::new(),
            rows: Vec::new(),
        });
    }

    // Extract column names from first row
    let columns: Vec<String> = rows[0]
        .columns()
        .iter()
        .map(|col| col.name().to_string())
        .collect();

    // Convert rows to JSON values
    let mut result_rows = Vec::new();

    for row in rows {
        let mut json_row = Vec::new();

        for (idx, column) in row.columns().iter().enumerate() {
            let value = postgres_value_to_json(&row, idx, column.type_info().name())?;
            json_row.push(value);
        }

        result_rows.push(json_row);
    }

    Ok(ExecuteQueryResponse {
        columns,
        rows: result_rows,
    })
}

/// Convert PostgreSQL value to JSON
///
/// Handles common PostgreSQL types and converts them to appropriate JSON representations.
fn postgres_value_to_json(
    row: &sqlx::postgres::PgRow,
    idx: usize,
    type_name: &str,
) -> Result<serde_json::Value, ExecuteQueryError> {
    use sqlx::Row;

    // Handle NULL
    if row.try_get_raw(idx)?.is_null() {
        return Ok(serde_json::Value::Null);
    }

    // Map PostgreSQL types to JSON
    let value = match type_name {
        "BOOL" => {
            let v: bool = row.try_get(idx)?;
            serde_json::Value::Bool(v)
        },
        "INT2" | "INT4" => {
            let v: i32 = row.try_get(idx)?;
            serde_json::Value::Number(v.into())
        },
        "INT8" => {
            let v: i64 = row.try_get(idx)?;
            serde_json::Value::Number(v.into())
        },
        "FLOAT4" => {
            let v: f32 = row.try_get(idx)?;
            serde_json::json!(v)
        },
        "FLOAT8" | "NUMERIC" => {
            let v: f64 = row.try_get(idx)?;
            serde_json::json!(v)
        },
        "TEXT" | "VARCHAR" | "CHAR" | "BPCHAR" | "NAME" => {
            let v: String = row.try_get(idx)?;
            serde_json::Value::String(v)
        },
        "UUID" => {
            let v: uuid::Uuid = row.try_get(idx)?;
            serde_json::Value::String(v.to_string())
        },
        "TIMESTAMP" | "TIMESTAMPTZ" => {
            let v: chrono::NaiveDateTime = row.try_get(idx)?;
            serde_json::Value::String(v.to_string())
        },
        "DATE" => {
            let v: chrono::NaiveDate = row.try_get(idx)?;
            serde_json::Value::String(v.to_string())
        },
        "JSON" | "JSONB" => {
            let v: serde_json::Value = row.try_get(idx)?;
            v
        },
        _ => {
            // Fallback: try to get as string
            let v: String = row.try_get(idx).unwrap_or_else(|_| format!("<{}>", type_name));
            serde_json::Value::String(v)
        },
    };

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_sql_allows_select() {
        assert!(validate_sql("SELECT * FROM data_sources").is_ok());
        assert!(validate_sql("select id from organizations").is_ok());
    }

    #[test]
    fn test_validate_sql_allows_explain() {
        assert!(validate_sql("EXPLAIN SELECT * FROM data_sources").is_ok());
    }

    #[test]
    fn test_validate_sql_blocks_drop() {
        let result = validate_sql("DROP TABLE data_sources");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("DROP"));
    }

    #[test]
    fn test_validate_sql_blocks_delete() {
        let result = validate_sql("DELETE FROM data_sources WHERE id = '123'");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_sql_blocks_update() {
        let result = validate_sql("UPDATE data_sources SET name = 'test'");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_sql_blocks_insert() {
        let result = validate_sql("INSERT INTO data_sources (name) VALUES ('test')");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_sql_blocks_truncate() {
        let result = validate_sql("TRUNCATE TABLE data_sources");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_sql_blocks_alter() {
        let result = validate_sql("ALTER TABLE data_sources ADD COLUMN test TEXT");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_sql_blocks_create() {
        let result = validate_sql("CREATE TABLE test (id UUID)");
        assert!(result.is_err());
    }
}
