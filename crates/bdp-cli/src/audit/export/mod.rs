//! Audit trail export functionality
//!
//! Provides export formats for regulatory compliance and research documentation.

pub mod das;
pub mod ema;
pub mod fda;
pub mod formats;
pub mod nih;
pub mod snapshot;

pub use das::DasExporter;
pub use ema::EmaExporter;
pub use fda::FdaExporter;
pub use formats::{ExportFormat, ExportOptions};
pub use nih::NihExporter;
pub use snapshot::SnapshotManager;

use crate::audit::logger::AuditLogger;
use crate::error::Result;
use std::path::PathBuf;
use std::sync::Arc;

/// Main export interface
pub struct AuditExporter {
    audit: Arc<dyn AuditLogger>,
    snapshot_manager: SnapshotManager,
}

impl AuditExporter {
    /// Create a new audit exporter
    pub fn new(audit: Arc<dyn AuditLogger>) -> Self {
        Self {
            snapshot_manager: SnapshotManager::new(audit.clone()),
            audit,
        }
    }

    /// Export audit trail to specified format
    pub async fn export(&self, format: ExportFormat, options: ExportOptions) -> Result<PathBuf> {
        // Create snapshot
        let snapshot_id = self.snapshot_manager.create_snapshot(&format).await?;

        // Export based on format
        let output = match format {
            ExportFormat::Fda => {
                let exporter = FdaExporter::new(self.audit.clone());
                exporter.export(&options).await?
            },
            ExportFormat::Nih => {
                let exporter = NihExporter::new(self.audit.clone());
                exporter.export(&options).await?
            },
            ExportFormat::Ema => {
                let exporter = EmaExporter::new(self.audit.clone());
                exporter.export(&options).await?
            },
            ExportFormat::Das => {
                let exporter = DasExporter::new(self.audit.clone());
                exporter.export(&options).await?
            },
            ExportFormat::Json => {
                // Raw JSON export
                let exporter = FdaExporter::new(self.audit.clone());
                exporter.export_raw_json(&options).await?
            },
        };

        // Update snapshot with output path
        self.snapshot_manager
            .update_snapshot_output(&snapshot_id, &output)
            .await?;

        Ok(output)
    }
}
