//! Data Availability Statement generator

use crate::audit::export::nih::NihExporter;
use crate::audit::export::formats::ExportOptions;
use crate::audit::logger::AuditLogger;
use crate::error::Result;
use std::path::PathBuf;
use std::sync::Arc;

/// Data Availability Statement exporter
///
/// Generates publication-ready data availability statements
pub struct DasExporter {
    nih_exporter: NihExporter,
}

impl DasExporter {
    /// Create a new DAS exporter
    pub fn new(audit: Arc<dyn AuditLogger>) -> Self {
        Self {
            nih_exporter: NihExporter::new(audit),
        }
    }

    /// Export Data Availability Statement
    ///
    /// This is essentially a simplified version of the NIH export,
    /// formatted for inclusion in research papers
    pub async fn export(&self, options: &ExportOptions) -> Result<PathBuf> {
        // For now, use the NIH exporter which generates a suitable format
        // In the future, this could be customized further
        self.nih_exporter.export(options).await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::audit::logger::LocalAuditLogger;

    #[test]
    fn test_das_exporter_creation() {
        let audit = Arc::new(
            LocalAuditLogger::new_in_memory("test-machine".to_string()).unwrap(),
        ) as Arc<dyn AuditLogger>;

        let _exporter = DasExporter::new(audit);
        // Just test creation for now
    }
}
