//! NIH Data Management & Sharing (DMS) export

use crate::audit::export::formats::ExportOptions;
use crate::audit::logger::AuditLogger;
use crate::audit::types::AuditEvent;
use crate::error::{CliError, Result};
use chrono::Utc;
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// NIH DMS exporter
pub struct NihExporter {
    audit: Arc<dyn AuditLogger>,
}

impl NihExporter {
    /// Create a new NIH exporter
    pub fn new(audit: Arc<dyn AuditLogger>) -> Self {
        Self { audit }
    }

    /// Export to NIH DMS format (markdown)
    pub async fn export(&self, options: &ExportOptions) -> Result<PathBuf> {
        let events = self.load_events().await?;
        let markdown = self.generate_markdown(&events, options)?;

        fs::write(&options.output, markdown)?;

        Ok(options.output.clone())
    }

    /// Generate NIH DMS markdown report
    fn generate_markdown(&self, events: &[AuditEvent], options: &ExportOptions) -> Result<String> {
        let mut md = String::new();

        // Header
        md.push_str("# NIH Data Management & Sharing (DMS) Compliance Report\n\n");
        md.push_str(&format!("**Generated**: {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        if let (Some(name), Some(version)) = (&options.project_name, &options.project_version) {
            md.push_str(&format!("**Project**: {} v{}\n\n", name, version));
        }

        md.push_str("---\n\n");

        // Data sources section
        md.push_str("## Data Sources\n\n");

        let sources = self.extract_data_sources(events);
        if sources.is_empty() {
            md.push_str("*No data sources recorded yet.*\n\n");
        } else {
            for (source_spec, info) in sources {
                md.push_str(&format!("### {}\n\n", source_spec));
                if let Some(downloaded_at) = info.get("downloaded_at") {
                    md.push_str(&format!("- **Downloaded**: {}\n", downloaded_at));
                }
                if let Some(checksum) = info.get("checksum") {
                    md.push_str(&format!("- **Checksum**: `{}`\n", checksum));
                }
                if let Some(size) = info.get("size_bytes") {
                    md.push_str(&format!("- **Size**: {} bytes\n", size));
                }
                md.push_str("\n");
            }
        }

        // Data Management Plan section
        md.push_str("## Data Management Plan\n\n");
        md.push_str("This project uses the BDP (Bioinformatics Data Package Manager) for systematic data management:\n\n");
        md.push_str("- **Version Control**: All data sources are version-pinned in `bdp.yml`\n");
        md.push_str("- **Reproducibility**: Lockfile (`bdl.lock`) ensures exact same data across environments\n");
        md.push_str("- **Integrity**: Cryptographic checksums verify data authenticity\n");
        md.push_str("- **Audit Trail**: Complete audit log tracks all data operations\n\n");

        // Compliance section
        md.push_str("## NIH DMS Policy Compliance\n\n");
        md.push_str("### Data Sharing\n\n");
        md.push_str("All data sources used in this project are publicly available and can be reproduced using:\n\n");
        md.push_str("```bash\n");
        md.push_str("git clone <repository-url>\n");
        md.push_str("cd <project-directory>\n");
        md.push_str("bdp pull\n");
        md.push_str("```\n\n");

        md.push_str("### Data Preservation\n\n");
        md.push_str("- **Manifest File**: `bdp.yml` (committed to version control)\n");
        md.push_str("- **Lockfile**: `bdl.lock` (committed to version control)\n");
        md.push_str("- **Audit Trail**: `.bdp/bdp.db` (local audit log)\n\n");

        // Audit summary
        md.push_str("## Audit Trail Summary\n\n");
        md.push_str(&format!("- **Total Events**: {}\n", events.len()));
        md.push_str(&format!("- **Machine ID**: {}\n", self.audit.machine_id()));

        let chain_verified = "Chain verification status available via `bdp audit verify`";
        md.push_str(&format!("- **Integrity**: {}\n\n", chain_verified));

        // Footer
        md.push_str("---\n\n");
        md.push_str("*This report was generated automatically by BDP CLI.*\n");
        md.push_str("*For more information, visit: https://github.com/datadir-lab/bdp*\n");

        Ok(md)
    }

    /// Extract data sources from audit events
    fn extract_data_sources(&self, events: &[AuditEvent]) -> HashMap<String, HashMap<String, String>> {
        let mut sources: HashMap<String, HashMap<String, String>> = HashMap::new();

        for event in events {
            if let Some(source_spec) = &event.source_spec {
                let entry = sources.entry(source_spec.clone()).or_insert_with(HashMap::new);

                // Extract information from event details
                if let Some(downloaded_at) = event.details.get("downloaded_at") {
                    entry.insert(
                        "downloaded_at".to_string(),
                        downloaded_at.as_str().unwrap_or("").to_string(),
                    );
                }
                if let Some(checksum) = event.details.get("checksum").or_else(|| event.details.get("sha256")) {
                    entry.insert(
                        "checksum".to_string(),
                        checksum.as_str().unwrap_or("").to_string(),
                    );
                }
                if let Some(size) = event.details.get("size_bytes") {
                    entry.insert(
                        "size_bytes".to_string(),
                        size.to_string(),
                    );
                }

                // Add timestamp from event
                entry.insert(
                    "downloaded_at".to_string(),
                    event.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                );
            }
        }

        sources
    }

    /// Load events from database
    async fn load_events(&self) -> Result<Vec<AuditEvent>> {
        let db_path = std::path::PathBuf::from(".bdp/bdp.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| CliError::audit(format!("Failed to open audit database: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT id, timestamp, event_type, source_spec, details, machine_id, event_hash, previous_hash FROM audit_events ORDER BY id ASC")
            .map_err(|e| CliError::audit(format!("Failed to prepare query: {}", e)))?;

        let events = stmt
            .query_map([], |row: &rusqlite::Row| {
                Ok(AuditEvent {
                    id: Some(row.get::<_, i64>(0)?),
                    timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    event_type: serde_json::from_str(&format!("\"{}\"", row.get::<_, String>(2)?))
                        .unwrap(),
                    source_spec: row.get::<_, Option<String>>(3)?,
                    details: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
                    machine_id: row.get::<_, String>(5)?,
                    event_hash: row.get::<_, Option<String>>(6)?,
                    previous_hash: row.get::<_, Option<String>>(7)?,
                    notes: None,
                    archived: false,
                })
            })
            .map_err(|e| CliError::audit(format!("Failed to query events: {}", e)))?;

        let events: Vec<AuditEvent> = events.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| CliError::audit(format!("Failed to collect events: {}", e)))?;

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::logger::LocalAuditLogger;
    use crate::audit::types::EventType;
    use serde_json::json;

    #[test]
    fn test_extract_data_sources() {
        let audit = Arc::new(
            LocalAuditLogger::new_in_memory("test-machine".to_string()).unwrap(),
        ) as Arc<dyn AuditLogger>;

        let exporter = NihExporter::new(audit);

        let events = vec![
            AuditEvent {
                id: Some(1),
                source_spec: Some("uniprot:P01308-fasta@1.0".to_string()),
                details: json!({
                    "checksum": "sha256-abc123",
                    "size_bytes": 4096
                }),
                ..Default::default()
            },
            AuditEvent {
                id: Some(2),
                source_spec: Some("ncbi:GRCh38-fasta@2.0".to_string()),
                details: json!({
                    "sha256": "sha256-def456"
                }),
                ..Default::default()
            },
        ];

        let sources = exporter.extract_data_sources(&events);
        assert_eq!(sources.len(), 2);
        assert!(sources.contains_key("uniprot:P01308-fasta@1.0"));
        assert!(sources.contains_key("ncbi:GRCh38-fasta@2.0"));
    }
}
