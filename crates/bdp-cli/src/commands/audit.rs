//! `bdp audit` command implementation
//!
//! Manages audit trail for regulatory compliance and research documentation.

use crate::audit::{
    get_machine_id, AuditExporter, AuditLogger, ExportFormat, ExportOptions, LocalAuditLogger,
};
use crate::error::{CliError, Result};
use crate::AuditCommand;
use chrono::{DateTime, Utc};
use colored::Colorize;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;

/// Execute audit command
pub async fn run(command: &AuditCommand) -> Result<()> {
    match command {
        AuditCommand::List { limit, source } => list(*limit, source.as_deref()).await,
        AuditCommand::Verify => verify().await,
        AuditCommand::Export {
            format,
            output,
            from,
            to,
            project_name,
            project_version,
        } => {
            export(
                format,
                output.as_deref(),
                from.as_deref(),
                to.as_deref(),
                project_name.as_deref(),
                project_version.as_deref(),
            )
            .await
        },
    }
}

/// List audit events
async fn list(limit: usize, source_filter: Option<&str>) -> Result<()> {
    let db_path = PathBuf::from(".bdp/bdp.db");

    if !db_path.exists() {
        println!("{} No audit trail found. Run 'bdp init' first.", "→".cyan());
        return Ok(());
    }

    let conn = Connection::open(&db_path).map_err(|e| {
        CliError::audit(format!(
            "Failed to open audit database at '{}': {}. The database file may be corrupted.",
            db_path.display(),
            e
        ))
    })?;

    let mut query =
        "SELECT id, timestamp, event_type, source_spec, details FROM audit_events".to_string();

    if source_filter.is_some() {
        query.push_str(" WHERE source_spec = ?1");
    }

    query.push_str(" ORDER BY id DESC LIMIT ?");
    let param_idx = if source_filter.is_some() { "?2" } else { "?1" };
    query = query.replace("LIMIT ?", &format!("LIMIT {}", param_idx));

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| CliError::audit(format!("Failed to prepare query: {}", e)))?;

    let mut rows = if let Some(source) = source_filter {
        let params: Vec<&dyn rusqlite::ToSql> = vec![&source, &limit];
        stmt.query(params.as_slice())
            .map_err(|e| CliError::audit(format!("Failed to query events: {}", e)))?
    } else {
        let params: Vec<&dyn rusqlite::ToSql> = vec![&limit];
        stmt.query(params.as_slice())
            .map_err(|e| CliError::audit(format!("Failed to query events: {}", e)))?
    };

    let mut events = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| CliError::audit(format!("Failed to fetch row: {}", e)))?
    {
        events.push((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?,
        ));
    }

    if events.is_empty() {
        println!("{} No audit events found", "→".cyan());
        return Ok(());
    }

    println!("{} Showing {} most recent events:", "→".cyan(), events.len());
    println!();

    for (id, timestamp, event_type, source_spec, details) in events.iter().rev() {
        let ts = DateTime::parse_from_rfc3339(timestamp)
            .ok()
            .map(|dt| {
                dt.with_timezone(&Utc)
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string()
            })
            .unwrap_or_else(|| timestamp.clone());

        println!("{} {} {}", format!("#{}", id).bright_black(), event_type.bold(), ts.dimmed());

        if let Some(spec) = source_spec {
            println!("  {} {}", "Source:".cyan(), spec);
        }

        // Parse and display relevant details
        if let Ok(details_json) = serde_json::from_str::<serde_json::Value>(details) {
            if let Some(obj) = details_json.as_object() {
                for (key, value) in obj {
                    // Skip internal fields
                    if key.starts_with('_') || key == "timestamp" {
                        continue;
                    }

                    let value_str = match value {
                        serde_json::Value::String(s) => s.clone(),
                        _ => value.to_string(),
                    };

                    if value_str.len() < 100 {
                        println!("  {} {}", format!("{}:", key).dimmed(), value_str);
                    }
                }
            }
        }

        println!();
    }

    Ok(())
}

/// Verify audit trail integrity
async fn verify() -> Result<()> {
    let db_path = PathBuf::from(".bdp/bdp.db");

    if !db_path.exists() {
        return Err(CliError::audit(
            "No audit trail found at '.bdp/bdp.db'. This directory must be initialized with 'bdp init' first.".to_string(),
        ));
    }

    println!("{} Verifying audit trail integrity...", "→".cyan());

    let machine_id = get_machine_id()?;
    let audit = Arc::new(LocalAuditLogger::new(db_path, machine_id)?);

    let verified = audit.verify_integrity().await?;

    if verified {
        println!("{} Audit trail verified successfully", "✓".green().bold());
        println!("  {} Hash chain is intact", "→".cyan());
        println!("  {} No tampering detected", "→".cyan());
    } else {
        println!("{} Audit trail verification FAILED", "✗".red().bold());
        println!("  {} Hash chain is broken", "→".yellow());
        println!("  {} Possible tampering or data corruption", "→".yellow());
    }

    Ok(())
}

/// Export audit trail to regulatory format
async fn export(
    format: &str,
    output: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
    project_name: Option<&str>,
    project_version: Option<&str>,
) -> Result<()> {
    let db_path = PathBuf::from(".bdp/bdp.db");

    if !db_path.exists() {
        return Err(CliError::audit(
            "No audit trail found at '.bdp/bdp.db'. Initialize this directory with 'bdp init' first.".to_string(),
        ));
    }

    // Parse export format
    let export_format = match format.to_lowercase().as_str() {
        "fda" => ExportFormat::Fda,
        "nih" => ExportFormat::Nih,
        "ema" => ExportFormat::Ema,
        "das" => ExportFormat::Das,
        "json" => ExportFormat::Json,
        _ => {
            return Err(CliError::audit(format!(
                "Unknown export format: {}. Valid formats: fda, nih, ema, das, json",
                format
            )))
        },
    };

    // Parse date range
    let from_dt = if let Some(from_str) = from {
        Some(
            DateTime::parse_from_rfc3339(from_str)
                .map_err(|e| CliError::audit(format!("Invalid 'from' date: {}", e)))?
                .with_timezone(&Utc),
        )
    } else {
        None
    };

    let to_dt = if let Some(to_str) = to {
        Some(
            DateTime::parse_from_rfc3339(to_str)
                .map_err(|e| CliError::audit(format!("Invalid 'to' date: {}", e)))?
                .with_timezone(&Utc),
        )
    } else {
        None
    };

    // Determine output path
    let output_path = if let Some(path) = output {
        PathBuf::from(path)
    } else {
        PathBuf::from(export_format.default_filename())
    };

    println!("{} Exporting audit trail to {} format...", "→".cyan(), format.to_uppercase());

    // Create exporter
    let machine_id = get_machine_id()?;
    let audit = Arc::new(LocalAuditLogger::new(db_path, machine_id)?);
    let exporter = AuditExporter::new(audit);

    // Build export options
    let mut options = ExportOptions::new(output_path.clone());
    if let Some(from) = from_dt {
        options = options.with_range(from, to_dt.unwrap_or_else(Utc::now));
    }
    if let (Some(name), Some(version)) = (project_name, project_version) {
        options = options.with_project(name.to_string(), version.to_string());
    }

    // Export
    let result_path = exporter.export(export_format, options).await?;

    println!("{} Export completed successfully", "✓".green().bold());
    println!("  {} {}", "File:".cyan(), result_path.display());

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::audit::{AuditEvent, AuditLogger, EventType};
    use serde_json::json;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_verify_empty_trail() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Init creates the database
        let bdp_dir = temp_dir.path().join(".bdp");
        std::fs::create_dir_all(&bdp_dir).unwrap();

        let db_path = bdp_dir.join("bdp.db");
        let machine_id = "test-machine".to_string();
        let _audit = Arc::new(LocalAuditLogger::new(db_path, machine_id).unwrap());

        // Verify should succeed with empty trail
        let result = verify().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_no_database() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // List should handle missing database gracefully
        let result = list(10, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_export_formats() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let bdp_dir = temp_dir.path().join(".bdp");
        std::fs::create_dir_all(&bdp_dir).unwrap();

        let db_path = bdp_dir.join("bdp.db");
        let machine_id = "test-machine".to_string();
        let audit = Arc::new(LocalAuditLogger::new(db_path, machine_id).unwrap());

        // Log some events
        for i in 0..3 {
            let event = AuditEvent::new(
                EventType::InitStart,
                None,
                json!({"test": i}),
                "test-machine".to_string(),
            );
            audit.log_event(event).await.unwrap();
        }

        // Test each format
        let formats = vec!["fda", "nih", "ema", "das", "json"];
        for format in formats {
            let output_path = temp_dir.path().join(format!("test-{}.out", format));
            let result = export(
                format,
                Some(output_path.to_str().unwrap()),
                None,
                None,
                Some("test-project"),
                Some("1.0.0"),
            )
            .await;
            assert!(result.is_ok(), "Export failed for format: {}", format);
            assert!(output_path.exists(), "Output file not created for format: {}", format);
        }
    }
}
