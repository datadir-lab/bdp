//! Audit logger trait and implementations

use crate::audit::schema;
use crate::audit::types::AuditEvent;
use crate::error::{CliError, Result};
use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Trait for audit logging (dependency injection)
#[async_trait]
pub trait AuditLogger: Send + Sync {
    /// Log an audit event
    async fn log_event(&self, event: AuditEvent) -> Result<i64>;

    /// Verify audit chain integrity
    async fn verify_integrity(&self) -> Result<bool>;

    /// Get machine ID
    fn machine_id(&self) -> &str;
}

/// Local SQLite audit logger (MVP implementation)
pub struct LocalAuditLogger {
    db: Arc<Mutex<Connection>>,
    machine_id: String,
}

impl LocalAuditLogger {
    /// Create a new local audit logger
    pub fn new(db_path: PathBuf, machine_id: String) -> Result<Self> {
        // Create parent directory if needed
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open database connection
        let conn = Connection::open(&db_path)
            .map_err(|e| CliError::Audit(format!("Failed to open audit database: {}", e)))?;

        // Initialize schema
        schema::init_schema(&conn)?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            machine_id,
        })
    }

    /// Create an in-memory audit logger (for testing)
    #[cfg(test)]
    pub fn new_in_memory(machine_id: String) -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| CliError::Audit(format!("Failed to create in-memory database: {}", e)))?;

        schema::init_schema(&conn)?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            machine_id,
        })
    }

    /// Get the last event hash for chain linking
    fn get_last_event_hash(&self, conn: &Connection) -> Result<Option<String>> {
        let mut stmt = conn
            .prepare("SELECT event_hash FROM audit_events ORDER BY id DESC LIMIT 1")
            .map_err(|e| CliError::Audit(format!("Failed to query last event: {}", e)))?;

        let result = stmt
            .query_row([], |row| row.get::<_, Option<String>>(0))
            .optional()
            .map_err(|e| CliError::Audit(format!("Failed to get last event hash: {}", e)))?;

        Ok(result.flatten())
    }
}

#[async_trait]
impl AuditLogger for LocalAuditLogger {
    async fn log_event(&self, mut event: AuditEvent) -> Result<i64> {
        let conn = self
            .db
            .lock()
            .map_err(|e| CliError::Audit(format!("Failed to acquire database lock: {}", e)))?;

        // Get previous hash for chain linking
        event.previous_hash = self.get_last_event_hash(&conn)?;

        // Convert details to JSON string
        let details_json = serde_json::to_string(&event.details)
            .map_err(|e| CliError::Audit(format!("Failed to serialize details: {}", e)))?;

        // Insert event
        conn.execute(
            r#"
            INSERT INTO audit_events (
                timestamp, event_type, source_spec, details,
                machine_id, previous_hash, notes, archived
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                event.timestamp.to_rfc3339(),
                event.event_type.as_str(),
                event.source_spec,
                details_json,
                event.machine_id,
                event.previous_hash,
                event.notes,
                event.archived,
            ],
        )
        .map_err(|e| CliError::Audit(format!("Failed to insert audit event: {}", e)))?;

        let event_id = conn.last_insert_rowid();

        // Compute and update hash
        event.id = Some(event_id);
        let event_hash = event.compute_hash();

        conn.execute(
            "UPDATE audit_events SET event_hash = ?1 WHERE id = ?2",
            params![event_hash, event_id],
        )
        .map_err(|e| CliError::Audit(format!("Failed to update event hash: {}", e)))?;

        Ok(event_id)
    }

    async fn verify_integrity(&self) -> Result<bool> {
        let conn = self
            .db
            .lock()
            .map_err(|e| CliError::Audit(format!("Failed to acquire database lock: {}", e)))?;

        // Load all events
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, timestamp, event_type, source_spec, details,
                       machine_id, event_hash, previous_hash
                FROM audit_events
                ORDER BY id ASC
                "#,
            )
            .map_err(|e| CliError::Audit(format!("Failed to prepare query: {}", e)))?;

        let events = stmt
            .query_map([], |row: &rusqlite::Row| {
                let timestamp_str = row.get::<_, String>(1)?;
                let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&chrono::Utc);

                let event_type_str = row.get::<_, String>(2)?;
                let event_type =
                    serde_json::from_str(&format!("\"{}\"", event_type_str)).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let details_str = row.get::<_, String>(4)?;
                let details = serde_json::from_str(&details_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                Ok(AuditEvent {
                    id: Some(row.get::<_, i64>(0)?),
                    timestamp,
                    event_type,
                    source_spec: row.get::<_, Option<String>>(3)?,
                    details,
                    machine_id: row.get::<_, String>(5)?,
                    event_hash: row.get::<_, Option<String>>(6)?,
                    previous_hash: row.get::<_, Option<String>>(7)?,
                    notes: None,
                    archived: false,
                })
            })
            .map_err(|e| CliError::Audit(format!("Failed to query events: {}", e)))?;

        let events: Vec<AuditEvent> = events
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| CliError::Audit(format!("Failed to collect events: {}", e)))?;

        // Verify chain
        for i in 1..events.len() {
            // If previous event has no hash, chain is broken
            let Some(prev_hash) = events[i - 1].event_hash.as_ref() else {
                return Ok(false);
            };
            let current_prev_hash = events[i].previous_hash.as_ref();

            if current_prev_hash != Some(prev_hash) {
                return Ok(false); // Chain broken
            }

            // Verify current event's hash
            let computed_hash = events[i].compute_hash();
            if events[i].event_hash.as_ref() != Some(&computed_hash) {
                return Ok(false); // Hash mismatch
            }
        }

        Ok(true)
    }

    fn machine_id(&self) -> &str {
        &self.machine_id
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::audit::types::EventType;
    use serde_json::json;

    #[tokio::test]
    async fn test_local_audit_logger_creation() {
        let logger = LocalAuditLogger::new_in_memory("test-machine".to_string()).unwrap();
        assert_eq!(logger.machine_id(), "test-machine");
    }

    #[tokio::test]
    async fn test_log_event() {
        let logger = LocalAuditLogger::new_in_memory("test-machine".to_string()).unwrap();

        let event = AuditEvent::new(
            EventType::InitStart,
            None,
            json!({"path": "/test"}),
            "test-machine".to_string(),
        );

        let event_id = logger.log_event(event).await.unwrap();
        assert_eq!(event_id, 1);
    }

    #[tokio::test]
    async fn test_event_chain_linking() {
        let logger = LocalAuditLogger::new_in_memory("test-machine".to_string()).unwrap();

        // Log first event
        let event1 = AuditEvent::new(
            EventType::InitStart,
            None,
            json!({"test": 1}),
            "test-machine".to_string(),
        );
        logger.log_event(event1).await.unwrap();

        // Log second event
        let event2 = AuditEvent::new(
            EventType::InitSuccess,
            None,
            json!({"test": 2}),
            "test-machine".to_string(),
        );
        logger.log_event(event2).await.unwrap();

        // Verify chain integrity
        let is_valid = logger.verify_integrity().await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_verify_integrity_valid_chain() {
        let logger = LocalAuditLogger::new_in_memory("test-machine".to_string()).unwrap();

        // Log multiple events
        for i in 0..5 {
            let event = AuditEvent::new(
                EventType::InitStart,
                None,
                json!({"index": i}),
                "test-machine".to_string(),
            );
            logger.log_event(event).await.unwrap();
        }

        let is_valid = logger.verify_integrity().await.unwrap();
        assert!(is_valid);
    }
}
