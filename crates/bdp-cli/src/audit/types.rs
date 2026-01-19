//! Audit event types and structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Audit event types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Command started
    InitStart,
    /// Command completed successfully
    InitSuccess,
    /// Command failed
    InitFailure,
    /// Source added
    SourceAdd,
    /// Source removed
    SourceRemove,
    /// Download started
    DownloadStart,
    /// Download completed
    DownloadSuccess,
    /// Download failed
    DownloadFailure,
    /// Checksum verification
    VerifyChecksum,
    /// Post-pull hook execution
    PostPullHook,
    /// Configuration change
    ConfigChange,
    /// Cache operation
    CacheOperation,
}

impl EventType {
    /// Convert to string representation
    pub fn as_str(&self) -> &str {
        match self {
            EventType::InitStart => "init_start",
            EventType::InitSuccess => "init_success",
            EventType::InitFailure => "init_failure",
            EventType::SourceAdd => "source_add",
            EventType::SourceRemove => "source_remove",
            EventType::DownloadStart => "download_start",
            EventType::DownloadSuccess => "download_success",
            EventType::DownloadFailure => "download_failure",
            EventType::VerifyChecksum => "verify_checksum",
            EventType::PostPullHook => "post_pull_hook",
            EventType::ConfigChange => "config_change",
            EventType::CacheOperation => "cache_operation",
        }
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Audit event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event ID (assigned by database)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Event type
    pub event_type: EventType,

    /// Source specification (e.g., "uniprot:P01308-fasta@1.0")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_spec: Option<String>,

    /// Event details (JSON)
    pub details: JsonValue,

    /// Machine ID
    pub machine_id: String,

    /// Event hash (computed on save)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_hash: Option<String>,

    /// Previous event hash (for chain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_hash: Option<String>,

    /// User notes/annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    /// Archived flag
    #[serde(default)]
    pub archived: bool,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(
        event_type: EventType,
        source_spec: Option<String>,
        details: JsonValue,
        machine_id: String,
    ) -> Self {
        Self {
            id: None,
            timestamp: Utc::now(),
            event_type,
            source_spec,
            details,
            machine_id,
            event_hash: None,
            previous_hash: None,
            notes: None,
            archived: false,
        }
    }

    /// Compute hash of this event
    pub fn compute_hash(&self) -> String {
        use sha2::{Digest, Sha256};

        let data = format!(
            "{}|{}|{}|{}|{}",
            self.id.unwrap_or(0),
            self.timestamp.to_rfc3339(),
            self.event_type.as_str(),
            self.source_spec.as_ref().unwrap_or(&String::new()),
            self.previous_hash.as_ref().unwrap_or(&String::new())
        );

        let hash = Sha256::digest(data.as_bytes());
        hex::encode(hash)
    }
}

impl Default for AuditEvent {
    fn default() -> Self {
        Self {
            id: None,
            timestamp: Utc::now(),
            event_type: EventType::InitStart,
            source_spec: None,
            details: serde_json::json!({}),
            machine_id: String::new(),
            event_hash: None,
            previous_hash: None,
            notes: None,
            archived: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_type_as_str() {
        assert_eq!(EventType::InitStart.as_str(), "init_start");
        assert_eq!(EventType::DownloadSuccess.as_str(), "download_success");
    }

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(
            EventType::InitStart,
            None,
            json!({"path": "/test"}),
            "machine-123".to_string(),
        );

        assert_eq!(event.event_type, EventType::InitStart);
        assert_eq!(event.machine_id, "machine-123");
        assert_eq!(event.archived, false);
    }

    #[test]
    fn test_compute_hash() {
        let mut event = AuditEvent::new(
            EventType::InitStart,
            None,
            json!({"test": true}),
            "machine-123".to_string(),
        );

        event.id = Some(1);
        let hash1 = event.compute_hash();

        event.id = Some(2);
        let hash2 = event.compute_hash();

        // Different IDs should produce different hashes
        assert_ne!(hash1, hash2);

        // Same event should produce same hash
        event.id = Some(1);
        let hash3 = event.compute_hash();
        assert_eq!(hash1, hash3);
    }
}
