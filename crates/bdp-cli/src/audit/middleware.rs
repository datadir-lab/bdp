//! CQRS audit middleware pattern
//!
//! Wraps commands with automatic audit logging.

use crate::audit::logger::AuditLogger;
use crate::audit::types::{AuditEvent, EventType};
use crate::error::Result;
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;

/// Execute a command with audit logging
///
/// This middleware pattern automatically logs:
/// - Command start event
/// - Command success/failure event
/// - Any errors that occur
pub async fn execute_with_audit<F, T, Fut>(
    audit: Arc<dyn AuditLogger>,
    start_event_type: EventType,
    success_event_type: EventType,
    failure_event_type: EventType,
    source_spec: Option<String>,
    start_details: JsonValue,
    command: F,
) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    // Log start event
    let start_event = AuditEvent::new(
        start_event_type,
        source_spec.clone(),
        start_details,
        audit.machine_id().to_string(),
    );

    audit.log_event(start_event).await?;

    // Execute command
    let result = command().await;

    // Log completion event
    match &result {
        Ok(_) => {
            let success_event = AuditEvent::new(
                success_event_type,
                source_spec,
                json!({"status": "success"}),
                audit.machine_id().to_string(),
            );
            audit.log_event(success_event).await?;
        }
        Err(e) => {
            let failure_event = AuditEvent::new(
                failure_event_type,
                source_spec,
                json!({
                    "status": "failure",
                    "error": e.to_string()
                }),
                audit.machine_id().to_string(),
            );
            audit.log_event(failure_event).await?;
        }
    }

    result
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::audit::logger::LocalAuditLogger;
    use crate::error::CliError;

    #[tokio::test]
    async fn test_execute_with_audit_success() {
        let logger = Arc::new(LocalAuditLogger::new_in_memory("test-machine".to_string()).unwrap())
            as Arc<dyn AuditLogger>;

        let result = execute_with_audit(
            logger.clone(),
            EventType::InitStart,
            EventType::InitSuccess,
            EventType::InitFailure,
            None,
            json!({"path": "/test"}),
            || async { Ok(42) },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        // Verify events were logged
        let is_valid = logger.verify_integrity().await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_execute_with_audit_failure() {
        let logger = Arc::new(LocalAuditLogger::new_in_memory("test-machine".to_string()).unwrap())
            as Arc<dyn AuditLogger>;

        let result: Result<()> = execute_with_audit(
            logger.clone(),
            EventType::InitStart,
            EventType::InitSuccess,
            EventType::InitFailure,
            None,
            json!({"path": "/test"}),
            || async {
                Err(CliError::audit("Test error"))
            },
        )
        .await;

        assert!(result.is_err());

        // Verify events were logged (start + failure)
        let is_valid = logger.verify_integrity().await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_execute_with_audit_source_spec() {
        let logger = Arc::new(LocalAuditLogger::new_in_memory("test-machine".to_string()).unwrap())
            as Arc<dyn AuditLogger>;

        let result = execute_with_audit(
            logger.clone(),
            EventType::DownloadStart,
            EventType::DownloadSuccess,
            EventType::DownloadFailure,
            Some("uniprot:P01308-fasta@1.0".to_string()),
            json!({"url": "https://example.com"}),
            || async { Ok(()) },
        )
        .await;

        assert!(result.is_ok());
    }
}
