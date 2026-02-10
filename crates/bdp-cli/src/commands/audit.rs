//! `bdp audit` command implementation
//!
//! Manages audit trail for regulatory compliance and research documentation.

use std::{path::PathBuf, sync::Arc};

use chrono::{DateTime, Utc};
use colored::Colorize;
use rusqlite::Connection;

use crate::{
    audit::{
        get_machine_id, AuditExporter, AuditLogger, ExportFormat, ExportOptions, LocalAuditLogger,
    },
    commands::output::Render,
    error::{CliError, Result},
    AuditCommand,
};

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Combined output for the `bdp audit` command tree.
pub enum AuditOutput {
    List(AuditListOutput),
    Verify(AuditVerifyOutput),
    Export(AuditExportOutput),
}

impl Render for AuditOutput {
    fn render(&self) {
        match self {
            AuditOutput::List(o) => o.render(),
            AuditOutput::Verify(o) => o.render(),
            AuditOutput::Export(o) => o.render(),
        }
    }
}

/// Output of `bdp audit list`.
pub struct AuditListOutput {
    /// False when the `.bdp/bdp.db` file does not exist.
    pub db_exists: bool,
    pub events: Vec<AuditEventInfo>,
}

/// A single audit event for display purposes.
pub struct AuditEventInfo {
    pub id: i64,
    pub timestamp: String,
    pub event_type: String,
    pub source_spec: Option<String>,
    pub details: String,
}

impl Render for AuditListOutput {
    fn render(&self) {
        use colored::Colorize;

        if !self.db_exists {
            println!("{} No audit trail found. Run 'bdp init' first.", "→".cyan());
            return;
        }

        if self.events.is_empty() {
            println!("{} No audit events found", "→".cyan());
            return;
        }

        println!("{} Showing {} most recent events:", "→".cyan(), self.events.len());
        println!();

        // Events are stored newest-first (ORDER BY id DESC); display oldest-first.
        for event in self.events.iter().rev() {
            render_event(event);
        }
    }
}

/// Render a single audit event line to the terminal.
fn render_event(event: &AuditEventInfo) {
    use colored::Colorize;

    let ts = DateTime::parse_from_rfc3339(&event.timestamp)
        .ok()
        .map(|dt| {
            dt.with_timezone(&Utc)
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string()
        })
        .unwrap_or_else(|| event.timestamp.clone());

    println!(
        "{} {} {}",
        format!("#{}", event.id).bright_black(),
        event.event_type.bold(),
        ts.dimmed()
    );

    if let Some(ref spec) = event.source_spec {
        println!("  {} {}", "Source:".cyan(), spec);
    }

    render_event_details(&event.details);
    println!();
}

/// Print the JSON details of an audit event, skipping internal fields
/// and values that are too long.
fn render_event_details(details: &str) {
    use colored::Colorize;

    let Ok(details_json) = serde_json::from_str::<serde_json::Value>(details) else {
        return;
    };
    let Some(obj) = details_json.as_object() else {
        return;
    };

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

/// Output of `bdp audit verify`.
pub struct AuditVerifyOutput {
    pub verified: bool,
}

impl Render for AuditVerifyOutput {
    fn render(&self) {
        use colored::Colorize;

        if self.verified {
            println!("{} Audit trail verified successfully", "✓".green().bold());
            println!("  {} Hash chain is intact", "→".cyan());
            println!("  {} No tampering detected", "→".cyan());
        } else {
            println!("{} Audit trail verification FAILED", "✗".red().bold());
            println!("  {} Hash chain is broken", "→".yellow());
            println!("  {} Possible tampering or data corruption", "→".yellow());
        }
    }
}

/// Output of `bdp audit export`.
pub struct AuditExportOutput {
    pub format: String,
    pub output_path: PathBuf,
}

impl Render for AuditExportOutput {
    fn render(&self) {
        use colored::Colorize;

        println!("{} Export completed successfully", "✓".green().bold());
        println!("  {} {}", "File:".cyan(), self.output_path.display());
    }
}

// ---------------------------------------------------------------------------
// Command implementations
// ---------------------------------------------------------------------------

/// Execute audit command
pub async fn run(command: &AuditCommand) -> Result<AuditOutput> {
    match command {
        AuditCommand::List { limit, source } => {
            list(*limit, source.as_deref()).await.map(AuditOutput::List)
        },
        AuditCommand::Verify => verify().await.map(AuditOutput::Verify),
        AuditCommand::Export {
            format,
            output,
            from,
            to,
            project_name,
            project_version,
        } => export(
            format,
            output.as_deref(),
            from.as_deref(),
            to.as_deref(),
            project_name.as_deref(),
            project_version.as_deref(),
        )
        .await
        .map(AuditOutput::Export),
    }
}

/// List audit events
async fn list(limit: usize, source_filter: Option<&str>) -> Result<AuditListOutput> {
    let db_path = PathBuf::from(".bdp/bdp.db");

    if !db_path.exists() {
        return Ok(AuditListOutput {
            db_exists: false,
            events: Vec::new(),
        });
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
        events.push(AuditEventInfo {
            id: row.get::<_, i64>(0)?,
            timestamp: row.get::<_, String>(1)?,
            event_type: row.get::<_, String>(2)?,
            source_spec: row.get::<_, Option<String>>(3)?,
            details: row.get::<_, String>(4)?,
        });
    }

    Ok(AuditListOutput {
        db_exists: true,
        events,
    })
}

/// Verify audit trail integrity
async fn verify() -> Result<AuditVerifyOutput> {
    let db_path = PathBuf::from(".bdp/bdp.db");

    if !db_path.exists() {
        return Err(CliError::audit(
            "No audit trail found at '.bdp/bdp.db'. This directory must be initialized with 'bdp \
             init' first."
                .to_string(),
        ));
    }

    println!("{} Verifying audit trail integrity...", "→".cyan());

    let machine_id = get_machine_id(None)?;
    let audit = Arc::new(LocalAuditLogger::new(db_path, machine_id)?);

    let verified = audit.verify_integrity().await?;

    Ok(AuditVerifyOutput { verified })
}

/// Export audit trail to regulatory format
async fn export(
    format: &str,
    output: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
    project_name: Option<&str>,
    project_version: Option<&str>,
) -> Result<AuditExportOutput> {
    let db_path = PathBuf::from(".bdp/bdp.db");

    if !db_path.exists() {
        return Err(CliError::audit(
            "No audit trail found at '.bdp/bdp.db'. Initialize this directory with 'bdp init' \
             first."
                .to_string(),
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
            )));
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
    let machine_id = get_machine_id(None)?;
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

    Ok(AuditExportOutput {
        format: format.to_uppercase(),
        output_path: result_path,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;
    use crate::audit::{AuditEvent, AuditLogger, EventType};

    #[tokio::test]
    #[serial]
    async fn test_verify_empty_trail() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().ok();
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

        // Restore original directory
        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(dir);
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_list_no_database() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().ok();
        std::env::set_current_dir(&temp_dir).unwrap();

        // List should handle missing database gracefully
        let result = list(10, None).await;
        assert!(result.is_ok());

        // Restore original directory
        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(dir);
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_export_formats() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().ok();
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
            if let Err(e) = &result {
                eprintln!("Export error for format {}: {:?}", format, e);
            }
            assert!(result.is_ok(), "Export failed for format: {} - {:?}", format, result.err());
            assert!(output_path.exists(), "Output file not created for format: {}", format);
        }

        // Restore original directory
        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(dir);
        }
    }
}
