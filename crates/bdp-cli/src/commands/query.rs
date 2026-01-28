//! `bdp query` command implementation
//!
//! Advanced SQL-like querying interface for BDP data sources and metadata.

use crate::api::client::ApiClient;
use crate::error::{CliError, Result};
use colored::Colorize;
use sqlparser::ast::Statement;
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use std::io::{self, IsTerminal};
use tracing::{debug, info, warn};

/// Run the query command
#[allow(clippy::too_many_arguments)]
pub async fn run(
    entity: Option<String>,
    select: Option<String>,
    where_clause: Vec<String>,
    order_by: Option<String>,
    limit: i64,
    offset: Option<i64>,
    group_by: Option<String>,
    aggregate: Option<String>,
    having: Option<String>,
    join: Option<String>,
    on: Option<String>,
    sql: Option<String>,
    format: Option<String>,
    output: Option<String>,
    no_header: bool,
    explain: bool,
    dry_run: bool,
    server_url: String,
) -> Result<()> {
    info!("Running query command");

    // Determine output format (smart default based on TTY)
    let output_format = determine_output_format(format.as_deref());

    // Build or use SQL query
    let query_sql = if let Some(raw_sql) = sql {
        // Use raw SQL directly
        debug!("Using raw SQL query");
        raw_sql
    } else {
        // Build SQL from flags
        debug!("Building SQL from flags");
        build_sql_from_flags(
            entity,
            select,
            where_clause,
            order_by,
            limit,
            offset,
            group_by,
            aggregate,
            having,
            join,
            on,
        )?
    };

    // Dry run: show SQL and exit
    if dry_run {
        println!("{}", "Generated SQL:".bold());
        println!("{}", query_sql);
        return Ok(());
    }

    // Explain: show query plan
    if explain {
        let explain_sql = format!("EXPLAIN {}", query_sql);
        println!("{}", "Query Execution Plan:".bold());
        println!("{}", explain_sql);
        // TODO: Execute EXPLAIN query and show results
        return Ok(());
    }

    // Validate SQL syntax
    validate_sql(&query_sql)?;

    // Execute query
    debug!(sql = %query_sql, "Executing query");
    let results = execute_query(&server_url, &query_sql).await?;

    // Format and output results
    output_results(&results, &output_format, output.as_deref(), no_header)?;

    Ok(())
}

/// Validate SQL syntax using sqlparser
fn validate_sql(sql: &str) -> Result<()> {
    let dialect = PostgreSqlDialect {};

    match Parser::parse_sql(&dialect, sql) {
        Ok(statements) => {
            if statements.is_empty() {
                return Err(CliError::config("Empty SQL query"));
            }

            // Check for dangerous statements
            for statement in &statements {
                match statement {
                    Statement::Drop { .. } => {
                        return Err(CliError::config("DROP statements are not allowed"));
                    }
                    Statement::Delete { .. } => {
                        return Err(CliError::config("DELETE statements are not allowed. Use the CLI commands for data management."));
                    }
                    Statement::Update { .. } => {
                        return Err(CliError::config("UPDATE statements are not allowed. Use the CLI commands for data management."));
                    }
                    Statement::Insert { .. } => {
                        return Err(CliError::config("INSERT statements are not allowed. Use the CLI commands for data management."));
                    }
                    Statement::Truncate { .. } => {
                        return Err(CliError::config("TRUNCATE statements are not allowed"));
                    }
                    Statement::AlterTable { .. } | Statement::CreateTable { .. } => {
                        return Err(CliError::config("DDL statements are not allowed"));
                    }
                    Statement::Query(_) | Statement::Explain { .. } => {
                        // These are safe
                    }
                    _ => {
                        warn!("Unknown statement type: {:?}", statement);
                    }
                }
            }

            Ok(())
        }
        Err(e) => {
            Err(CliError::config(&format!(
                "Invalid SQL syntax: {}\n\nSQL:\n{}",
                e, sql
            )))
        }
    }
}

/// Build SQL query from command flags
#[allow(clippy::too_many_arguments)]
fn build_sql_from_flags(
    entity: Option<String>,
    select: Option<String>,
    where_clause: Vec<String>,
    order_by: Option<String>,
    limit: i64,
    offset: Option<i64>,
    group_by: Option<String>,
    aggregate: Option<String>,
    having: Option<String>,
    join: Option<String>,
    on: Option<String>,
) -> Result<String> {
    // Validate entity is provided
    let entity = entity.ok_or_else(|| {
        CliError::config("Entity or --sql is required. Try 'bdp query protein' or 'bdp query --sql \"SELECT ...\"'")
    })?;

    // Resolve entity alias to table name
    let (table_name, auto_joins) = resolve_entity_alias(&entity)?;

    // Build SELECT clause
    let select_clause = if let Some(fields) = select {
        fields
    } else {
        "*".to_string()
    };

    // Build WHERE clause
    let where_clause_sql = build_where_clause(&where_clause)?;

    // Build SQL
    let mut sql = format!("SELECT {} FROM {}", select_clause, table_name);

    // Add auto-joins for metadata
    for join_sql in auto_joins {
        sql.push_str(&format!(" {}", join_sql));
    }

    // Add manual JOIN if specified
    if let Some(join_table) = join {
        if let Some(join_condition) = on {
            sql.push_str(&format!(" JOIN {} ON {}", join_table, join_condition));
        } else {
            return Err(CliError::config("--join requires --on condition"));
        }
    }

    // Add WHERE
    if !where_clause_sql.is_empty() {
        sql.push_str(&format!(" WHERE {}", where_clause_sql));
    }

    // Add GROUP BY
    if let Some(group_field) = group_by {
        sql.push_str(&format!(" GROUP BY {}", group_field));

        // Add aggregate if specified
        if let Some(agg_expr) = aggregate {
            // Replace SELECT clause with aggregation
            sql = sql.replace(&format!("SELECT {}", select_clause), &format!("SELECT {}, {}", group_field, agg_expr));
        }

        // Add HAVING
        if let Some(having_expr) = having {
            sql.push_str(&format!(" HAVING {}", having_expr));
        }
    }

    // Add ORDER BY
    if let Some(order) = order_by {
        let (field, direction) = parse_order_by(&order)?;
        sql.push_str(&format!(" ORDER BY {} {}", field, direction));
    }

    // Add LIMIT
    sql.push_str(&format!(" LIMIT {}", limit));

    // Add OFFSET
    if let Some(offset_val) = offset {
        sql.push_str(&format!(" OFFSET {}", offset_val));
    }

    Ok(sql)
}

/// Resolve entity alias to table name and auto-joins
fn resolve_entity_alias(entity: &str) -> Result<(String, Vec<String>)> {
    match entity.to_lowercase().as_str() {
        // Entity aliases with metadata auto-joins
        "protein" => Ok((
            "data_sources".to_string(),
            vec![
                "LEFT JOIN protein_metadata pm ON data_sources.metadata_id = pm.id WHERE data_sources.type = 'protein'".to_string(),
            ],
        )),
        "gene" => Ok((
            "data_sources".to_string(),
            vec![
                "LEFT JOIN gene_metadata gm ON data_sources.metadata_id = gm.id WHERE data_sources.type = 'gene'".to_string(),
            ],
        )),
        "genome" => Ok((
            "data_sources WHERE type = 'genome'".to_string(),
            vec![],
        )),
        "transcriptome" => Ok((
            "data_sources WHERE type = 'transcriptome'".to_string(),
            vec![],
        )),
        "proteome" => Ok((
            "data_sources WHERE type = 'proteome'".to_string(),
            vec![],
        )),

        // Direct table access
        "tools" => Ok(("tools".to_string(), vec![])),
        "orgs" | "organizations" => Ok(("organizations".to_string(), vec![])),
        "protein_metadata" => Ok(("protein_metadata".to_string(), vec![])),
        "gene_metadata" => Ok(("gene_metadata".to_string(), vec![])),
        "organism_taxonomy" => Ok(("organism_taxonomy".to_string(), vec![])),
        "publication_refs" => Ok(("publication_refs".to_string(), vec![])),

        // Unknown entity
        _ => Err(CliError::config(&format!(
            "Unknown entity: '{}'\n\nAvailable entities:\n  {}",
            entity,
            "protein, gene, genome, transcriptome, proteome, tools, orgs,\n  protein_metadata, gene_metadata, organism_taxonomy, publication_refs"
        ))),
    }
}

/// Build WHERE clause from multiple conditions
fn build_where_clause(conditions: &[String]) -> Result<String> {
    if conditions.is_empty() {
        return Ok(String::new());
    }

    let mut where_parts = Vec::new();

    for condition in conditions {
        // Check if it's a simple key=value or complex expression
        if condition.contains('=') && !condition.contains(' ') {
            // Simple key=value
            let parts: Vec<&str> = condition.splitn(2, '=').collect();
            if parts.len() == 2 {
                where_parts.push(format!("{} = '{}'", parts[0].trim(), parts[1].trim()));
            }
        } else {
            // Complex expression - use as-is
            where_parts.push(condition.clone());
        }
    }

    Ok(where_parts.join(" AND "))
}

/// Parse order-by field and direction
fn parse_order_by(order_by: &str) -> Result<(String, String)> {
    if let Some((field, dir)) = order_by.split_once(':') {
        let direction = match dir.to_lowercase().as_str() {
            "asc" => "ASC",
            "desc" => "DESC",
            _ => return Err(CliError::config(&format!("Invalid order direction: '{}'. Use 'asc' or 'desc'", dir))),
        };
        Ok((field.to_string(), direction.to_string()))
    } else {
        // Default to ASC
        Ok((order_by.to_string(), "ASC".to_string()))
    }
}

/// Determine output format based on TTY and user preference
fn determine_output_format(format: Option<&str>) -> String {
    if let Some(fmt) = format {
        return fmt.to_string();
    }

    // Smart default: table for TTY, tsv for pipes
    if io::stdout().is_terminal() {
        "table".to_string()
    } else {
        "tsv".to_string()
    }
}

/// Execute query against the backend
async fn execute_query(server_url: &str, sql: &str) -> Result<QueryResults> {
    let client = ApiClient::new(server_url.to_string())?;
    debug!("Executing query against backend");
    client.execute_query(sql.to_string()).await
}

/// Output results in specified format
fn output_results(
    results: &QueryResults,
    format: &str,
    output_file: Option<&str>,
    no_header: bool,
) -> Result<()> {
    let formatted = match format {
        "table" => format_as_table(results),
        "json" => format_as_json(results)?,
        "csv" => format_as_csv(results, no_header),
        "tsv" => format_as_tsv(results, no_header),
        "compact" => format_as_compact(results),
        _ => return Err(CliError::config(&format!("Unknown format: '{}'. Use table, json, csv, tsv, or compact", format))),
    };

    // Write to file or stdout
    if let Some(file_path) = output_file {
        std::fs::write(file_path, formatted)?;
        println!("{} Output written to: {}", "✓".green(), file_path.cyan());
    } else {
        print!("{}", formatted);
    }

    Ok(())
}

/// Format results as table
fn format_as_table(results: &QueryResults) -> String {
    use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Table};

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(&results.columns);

    for row in &results.rows {
        let row_strings: Vec<String> = row.iter().map(|v| value_to_string(v)).collect();
        table.add_row(row_strings);
    }

    format!("{}\n", table)
}

/// Convert JSON value to string for display
fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            format!("[{}]", arr.iter().map(value_to_string).collect::<Vec<_>>().join(", "))
        }
        serde_json::Value::Object(_) => value.to_string(),
    }
}

/// Format results as JSON
fn format_as_json(results: &QueryResults) -> Result<String> {
    let rows: Vec<serde_json::Map<String, serde_json::Value>> = results
        .rows
        .iter()
        .map(|row| {
            let mut map = serde_json::Map::new();
            for (i, col_name) in results.columns.iter().enumerate() {
                if let Some(value) = row.get(i) {
                    map.insert(col_name.clone(), value.clone());
                }
            }
            map
        })
        .collect();

    Ok(serde_json::to_string_pretty(&rows)?)
}

/// Format results as CSV
fn format_as_csv(results: &QueryResults, no_header: bool) -> String {
    let mut output = String::new();

    // Header
    if !no_header {
        output.push_str(&results.columns.join(","));
        output.push('\n');
    }

    // Rows
    for row in &results.rows {
        let row_strings: Vec<String> = row.iter().map(|v| csv_escape(&value_to_string(v))).collect();
        output.push_str(&row_strings.join(","));
        output.push('\n');
    }

    output
}

/// Escape CSV value
fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Format results as TSV
fn format_as_tsv(results: &QueryResults, no_header: bool) -> String {
    let mut output = String::new();

    // Header
    if !no_header {
        output.push_str(&results.columns.join("\t"));
        output.push('\n');
    }

    // Rows
    for row in &results.rows {
        let row_strings: Vec<String> = row.iter().map(value_to_string).collect();
        output.push_str(&row_strings.join("\t"));
        output.push('\n');
    }

    output
}

/// Format results as compact (one per line)
fn format_as_compact(results: &QueryResults) -> String {
    let mut output = String::new();

    for row in &results.rows {
        let row_strings: Vec<String> = row.iter().map(value_to_string).collect();
        output.push_str(&row_strings.join(" "));
        output.push('\n');
    }

    output
}

// Re-export QueryResults from API types for convenience
pub use crate::api::types::QueryResults;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_entity_alias_protein() {
        let (table, joins) = resolve_entity_alias("protein").unwrap();
        assert!(table.contains("data_sources"));
        assert_eq!(joins.len(), 1);
        assert!(joins[0].contains("protein_metadata"));
    }

    #[test]
    fn test_resolve_entity_alias_gene() {
        let (table, joins) = resolve_entity_alias("gene").unwrap();
        assert!(table.contains("data_sources"));
        assert_eq!(joins.len(), 1);
        assert!(joins[0].contains("gene_metadata"));
    }

    #[test]
    fn test_resolve_entity_alias_tools() {
        let (table, joins) = resolve_entity_alias("tools").unwrap();
        assert_eq!(table, "tools");
        assert!(joins.is_empty());
    }

    #[test]
    fn test_resolve_entity_alias_unknown() {
        let result = resolve_entity_alias("unknown");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown entity"));
    }

    #[test]
    fn test_build_where_clause_simple() {
        let conditions = vec!["organism=human".to_string()];
        let result = build_where_clause(&conditions).unwrap();
        assert_eq!(result, "organism = 'human'");
    }

    #[test]
    fn test_build_where_clause_multiple() {
        let conditions = vec!["organism=human".to_string(), "format=fasta".to_string()];
        let result = build_where_clause(&conditions).unwrap();
        assert_eq!(result, "organism = 'human' AND format = 'fasta'");
    }

    #[test]
    fn test_build_where_clause_complex() {
        let conditions = vec!["organism='human' AND downloads>1000".to_string()];
        let result = build_where_clause(&conditions).unwrap();
        assert_eq!(result, "organism='human' AND downloads>1000");
    }

    #[test]
    fn test_parse_order_by_asc() {
        let (field, dir) = parse_order_by("downloads:asc").unwrap();
        assert_eq!(field, "downloads");
        assert_eq!(dir, "ASC");
    }

    #[test]
    fn test_parse_order_by_desc() {
        let (field, dir) = parse_order_by("downloads:desc").unwrap();
        assert_eq!(field, "downloads");
        assert_eq!(dir, "DESC");
    }

    #[test]
    fn test_parse_order_by_default() {
        let (field, dir) = parse_order_by("name").unwrap();
        assert_eq!(field, "name");
        assert_eq!(dir, "ASC");
    }

    #[test]
    fn test_determine_output_format_explicit() {
        assert_eq!(determine_output_format(Some("json")), "json");
        assert_eq!(determine_output_format(Some("csv")), "csv");
    }

    #[test]
    fn test_format_as_json() {
        let results = QueryResults {
            columns: vec!["name".to_string(), "version".to_string()],
            rows: vec![vec![
                serde_json::Value::String("test1".to_string()),
                serde_json::Value::String("1.0".to_string()),
            ]],
        };
        let json = format_as_json(&results).unwrap();
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"test1\""));
    }

    #[test]
    fn test_format_as_csv() {
        let results = QueryResults {
            columns: vec!["name".to_string(), "version".to_string()],
            rows: vec![vec![
                serde_json::Value::String("test1".to_string()),
                serde_json::Value::String("1.0".to_string()),
            ]],
        };
        let csv = format_as_csv(&results, false);
        assert!(csv.starts_with("name,version"));
        assert!(csv.contains("test1,1.0"));
    }

    #[test]
    fn test_format_as_tsv() {
        let results = QueryResults {
            columns: vec!["name".to_string(), "version".to_string()],
            rows: vec![vec![
                serde_json::Value::String("test1".to_string()),
                serde_json::Value::String("1.0".to_string()),
            ]],
        };
        let tsv = format_as_tsv(&results, false);
        assert!(tsv.starts_with("name\tversion"));
        assert!(tsv.contains("test1\t1.0"));
    }
}
