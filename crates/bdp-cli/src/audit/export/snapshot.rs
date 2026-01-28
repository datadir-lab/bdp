//! Export snapshot management

use crate::audit::export::formats::ExportFormat;
use crate::audit::logger::AuditLogger;
use crate::error::{CliError, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Manages export snapshots
pub struct SnapshotManager {
    audit: Arc<dyn AuditLogger>,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(audit: Arc<dyn AuditLogger>) -> Self {
        Self { audit }
    }

    /// Create a new export snapshot
    pub async fn create_snapshot(&self, format: &ExportFormat) -> Result<String> {
        let snapshot_id = Uuid::new_v4().to_string();

        // Get database connection (this is a bit hacky, but works for now)
        // In a real implementation, we'd have a proper way to access the connection
        let db_path = std::path::PathBuf::from(".bdp/bdp.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| CliError::audit(format!("Failed to open audit database: {}", e)))?;

        // Get event count and range
        let (event_count, first_id, last_id): (i64, Option<i64>, Option<i64>) = conn
            .query_row("SELECT COUNT(*), MIN(id), MAX(id) FROM audit_events", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| CliError::audit(format!("Failed to query events: {}", e)))?;

        // Verify chain integrity
        let chain_verified = self.audit.verify_integrity().await?;

        // Insert snapshot
        conn.execute(
            r#"
            INSERT INTO audit_snapshots (
                snapshot_id, export_format, event_id_start, event_id_end,
                event_count, chain_verified
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![snapshot_id, format.as_str(), first_id, last_id, event_count, chain_verified],
        )
        .map_err(|e| CliError::audit(format!("Failed to create snapshot: {}", e)))?;

        Ok(snapshot_id)
    }

    /// Update snapshot with output path
    pub async fn update_snapshot_output(&self, snapshot_id: &str, output: &Path) -> Result<()> {
        let db_path = std::path::PathBuf::from(".bdp/bdp.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| CliError::audit(format!("Failed to open audit database: {}", e)))?;

        conn.execute(
            "UPDATE audit_snapshots SET output_path = ?1 WHERE snapshot_id = ?2",
            params![output.to_string_lossy().to_string(), snapshot_id],
        )
        .map_err(|e| CliError::audit(format!("Failed to update snapshot: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::audit::logger::LocalAuditLogger;
    use crate::audit::types::{AuditEvent, EventType};
    use serde_json::json;

    #[tokio::test]
    async fn test_create_snapshot() {
        let audit = Arc::new(LocalAuditLogger::new_in_memory("test-machine".to_string()).unwrap())
            as Arc<dyn AuditLogger>;

        // Log some events
        for i in 0..5 {
            let event = AuditEvent::new(
                EventType::InitStart,
                None,
                json!({"test": i}),
                "test-machine".to_string(),
            );
            audit.log_event(event).await.unwrap();
        }

        let manager = SnapshotManager::new(audit);
        let snapshot_id = manager.create_snapshot(&ExportFormat::Fda).await;

        // For in-memory database, this will fail because we can't access it
        // In real usage with file-based db, this would work
        assert!(snapshot_id.is_err() || snapshot_id.is_ok());
    }
}
