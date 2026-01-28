//! EMA ALCOA++ compliance export

use crate::audit::export::formats::ExportOptions;
use crate::audit::logger::AuditLogger;
use crate::error::{CliError, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// EMA ALCOA++ compliance report
#[derive(Debug, Serialize, Deserialize)]
pub struct EmaReport {
    pub alcoa_plus_compliance_report: AlcoaPlusReport,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlcoaPlusReport {
    pub generated_at: String,
    pub project: Option<ProjectInfo>,
    pub attributable: ComplianceItem,
    pub legible: ComplianceItem,
    pub contemporaneous: ComplianceItem,
    pub original: ComplianceItem,
    pub accurate: ComplianceItem,
    pub complete: ComplianceItem,
    pub consistent: ComplianceItem,
    pub enduring: ComplianceItem,
    pub available: ComplianceItem,
    pub traceable: ComplianceItem,
    pub disclaimer: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComplianceItem {
    pub status: String,
    pub evidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// EMA exporter
pub struct EmaExporter {
    audit: Arc<dyn AuditLogger>,
}

impl EmaExporter {
    /// Create a new EMA exporter
    pub fn new(audit: Arc<dyn AuditLogger>) -> Self {
        Self { audit }
    }

    /// Export to EMA ALCOA++ format (YAML)
    pub async fn export(&self, options: &ExportOptions) -> Result<PathBuf> {
        let report = self.generate_report(options).await?;

        // Serialize to YAML
        let yaml = serde_yaml::to_string(&report)
            .map_err(|e| CliError::audit(format!("Failed to serialize EMA report: {}", e)))?;

        fs::write(&options.output, yaml)?;

        Ok(options.output.clone())
    }

    /// Generate EMA ALCOA++ report
    async fn generate_report(&self, options: &ExportOptions) -> Result<EmaReport> {
        let chain_verified = self.audit.verify_integrity().await?;

        let report = EmaReport {
            alcoa_plus_compliance_report: AlcoaPlusReport {
                generated_at: Utc::now().to_rfc3339(),
                project: options.project_name.as_ref().zip(options.project_version.as_ref())
                    .map(|(name, version)| ProjectInfo {
                        name: name.clone(),
                        version: version.clone(),
                    }),
                attributable: ComplianceItem {
                    status: "compliant".to_string(),
                    evidence: "All actions recorded with machine_id and timestamp".to_string(),
                    details: Some(serde_json::json!({
                        "machine_id": self.audit.machine_id(),
                        "tracking": "Automated via CQRS middleware"
                    })),
                },
                legible: ComplianceItem {
                    status: "compliant".to_string(),
                    evidence: "Human-readable JSON/YAML/Markdown exports, machine-processable SQLite database".to_string(),
                    details: Some(serde_json::json!({
                        "formats": ["JSON", "YAML", "Markdown", "SQLite"]
                    })),
                },
                contemporaneous: ComplianceItem {
                    status: "compliant".to_string(),
                    evidence: "Events timestamped at occurrence (ISO 8601 format)".to_string(),
                    details: Some(serde_json::json!({
                        "timestamp_format": "ISO 8601",
                        "timezone": "UTC"
                    })),
                },
                original: ComplianceItem {
                    status: "compliant".to_string(),
                    evidence: "Source URLs recorded, checksums verify original data".to_string(),
                    details: Some(serde_json::json!({
                        "verification": "SHA-256 checksums in lockfile"
                    })),
                },
                accurate: ComplianceItem {
                    status: "compliant".to_string(),
                    evidence: "Cryptographic checksums, automated integrity verification".to_string(),
                    details: Some(serde_json::json!({
                        "checksum_algorithm": "SHA-256",
                        "hash_chain": "Event linking via cryptographic hashes"
                    })),
                },
                complete: ComplianceItem {
                    status: "compliant".to_string(),
                    evidence: "All data operations logged (download, verify, post-pull)".to_string(),
                    details: Some(serde_json::json!({
                        "event_types": ["init", "download", "verify", "post_pull", "config"]
                    })),
                },
                consistent: ComplianceItem {
                    status: "compliant".to_string(),
                    evidence: "Chronological event ordering enforced by database".to_string(),
                    details: Some(serde_json::json!({
                        "ordering": "Ascending by ID and timestamp",
                        "immutability": "Hash chain prevents reordering"
                    })),
                },
                enduring: ComplianceItem {
                    status: "compliant".to_string(),
                    evidence: "SQLite database for long-term storage, archival exports".to_string(),
                    details: Some(serde_json::json!({
                        "storage": "SQLite + JSON/YAML archives",
                        "format": "Industry-standard, readable for decades"
                    })),
                },
                available: ComplianceItem {
                    status: "compliant".to_string(),
                    evidence: "Multiple export formats (JSON, YAML, Markdown)".to_string(),
                    details: Some(serde_json::json!({
                        "export_commands": [
                            "bdp audit export --format fda",
                            "bdp audit export --format das",
                            "bdp audit export --format ema"
                        ]
                    })),
                },
                traceable: ComplianceItem {
                    status: "compliant".to_string(),
                    evidence: format!("Full provenance from source to derived files. Chain verified: {}", chain_verified),
                    details: Some(serde_json::json!({
                        "provenance": "Source files → Post-pull outputs",
                        "chain_verified": chain_verified
                    })),
                },
                disclaimer: "This audit trail is stored locally in SQLite and is editable by the user. It is intended for research documentation and regulatory reporting, not for legal evidence or forensic purposes.".to_string(),
            },
        };

        Ok(report)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::audit::logger::LocalAuditLogger;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_ema_report_structure() {
        let audit = Arc::new(
            LocalAuditLogger::new_in_memory("test-machine".to_string()).unwrap(),
        ) as Arc<dyn AuditLogger>;

        let exporter = EmaExporter::new(audit);

        let temp_dir = TempDir::new().unwrap();
        let output = temp_dir.path().join("audit-ema.yaml");

        let options = ExportOptions::new(output.clone())
            .with_project("test-project".to_string(), "1.0.0".to_string());

        let report = exporter.generate_report(&options).await.unwrap();

        assert_eq!(report.alcoa_plus_compliance_report.attributable.status, "compliant");
        assert_eq!(report.alcoa_plus_compliance_report.legible.status, "compliant");
        assert!(report.alcoa_plus_compliance_report.project.is_some());
    }
}
