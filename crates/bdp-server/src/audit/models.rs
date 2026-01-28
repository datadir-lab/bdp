//! Audit data models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

// ============================================================================
// Audit Query Constants
// ============================================================================

/// Default number of audit entries returned per query
pub const DEFAULT_AUDIT_QUERY_LIMIT: i64 = 100;

/// Maximum number of audit entries that can be returned in a single query.
/// This prevents excessive memory usage and query timeouts.
pub const MAX_AUDIT_QUERY_LIMIT: i64 = 1000;

/// Audit log entry from the database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditEntry {
    /// Unique identifier for the audit entry
    pub id: Uuid,
    /// User ID who performed the action (nullable for anonymous actions)
    pub user_id: Option<Uuid>,
    /// Action performed
    pub action: String,
    /// Type of resource affected
    pub resource_type: String,
    /// ID of the affected resource
    pub resource_id: Option<Uuid>,
    /// Before/after state or creation data
    pub changes: Option<JsonValue>,
    /// Client IP address (IPv4 or IPv6)
    pub ip_address: Option<String>,
    /// Client user agent string
    pub user_agent: Option<String>,
    /// Timestamp when the action occurred
    pub timestamp: DateTime<Utc>,
    /// Additional contextual metadata
    pub metadata: Option<JsonValue>,
}

/// Audit action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditAction {
    Create,
    Update,
    Delete,
    Read,
    Login,
    Logout,
    Register,
    Publish,
    Unpublish,
    Archive,
    Upload,
    Download,
    Grant,
    Revoke,
    Ingest,
    Other,
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Read => "read",
            Self::Login => "login",
            Self::Logout => "logout",
            Self::Register => "register",
            Self::Publish => "publish",
            Self::Unpublish => "unpublish",
            Self::Archive => "archive",
            Self::Upload => "upload",
            Self::Download => "download",
            Self::Grant => "grant",
            Self::Revoke => "revoke",
            Self::Ingest => "ingest",
            Self::Other => "other",
        }
    }
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Resource types that can be audited
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Organization,
    DataSource,
    Version,
    Tool,
    RegistryEntry,
    VersionFile,
    Dependency,
    Organism,
    ProteinMetadata,
    Citation,
    Tag,
    Download,
    VersionMapping,
    User,
    Session,
    ApiKey,
    IngestionJob,
    Other,
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Organization => "organization",
            Self::DataSource => "data_source",
            Self::Version => "version",
            Self::Tool => "tool",
            Self::RegistryEntry => "registry_entry",
            Self::VersionFile => "version_file",
            Self::Dependency => "dependency",
            Self::Organism => "organism",
            Self::ProteinMetadata => "protein_metadata",
            Self::Citation => "citation",
            Self::Tag => "tag",
            Self::Download => "download",
            Self::VersionMapping => "version_mapping",
            Self::User => "user",
            Self::Session => "session",
            Self::ApiKey => "api_key",
            Self::IngestionJob => "ingestion_job",
            Self::Other => "other",
        }
    }
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Query parameters for audit logs
#[derive(Debug, Clone, Deserialize)]
pub struct AuditQuery {
    /// Filter by user ID
    pub user_id: Option<Uuid>,
    /// Filter by action
    pub action: Option<AuditAction>,
    /// Filter by resource type
    pub resource_type: Option<ResourceType>,
    /// Filter by resource ID
    pub resource_id: Option<Uuid>,
    /// Start timestamp for range query
    pub start_time: Option<DateTime<Utc>>,
    /// End timestamp for range query
    pub end_time: Option<DateTime<Utc>>,
    /// Maximum number of results to return
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Offset for pagination
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    DEFAULT_AUDIT_QUERY_LIMIT
}

impl Default for AuditQuery {
    fn default() -> Self {
        Self {
            user_id: None,
            action: None,
            resource_type: None,
            resource_id: None,
            start_time: None,
            end_time: None,
            limit: default_limit(),
            offset: 0,
        }
    }
}

/// Input for creating an audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuditEntry {
    /// User ID who performed the action (nullable for anonymous actions)
    pub user_id: Option<Uuid>,
    /// Action performed
    pub action: AuditAction,
    /// Type of resource affected
    pub resource_type: ResourceType,
    /// ID of the affected resource
    pub resource_id: Option<Uuid>,
    /// Before/after state or creation data
    pub changes: Option<JsonValue>,
    /// Additional contextual metadata
    pub metadata: Option<JsonValue>,
    /// Client IP address
    pub ip_address: Option<String>,
    /// Client user agent string
    pub user_agent: Option<String>,
}

impl CreateAuditEntry {
    /// Create a builder for constructing audit entries
    pub fn builder() -> AuditEntryBuilder {
        AuditEntryBuilder::default()
    }
}

/// Builder for creating audit entries
#[derive(Debug, Clone, Default)]
pub struct AuditEntryBuilder {
    user_id: Option<Uuid>,
    action: Option<AuditAction>,
    resource_type: Option<ResourceType>,
    resource_id: Option<Uuid>,
    changes: Option<JsonValue>,
    metadata: Option<JsonValue>,
    ip_address: Option<String>,
    user_agent: Option<String>,
}

impl AuditEntryBuilder {
    pub fn user_id(mut self, user_id: Option<Uuid>) -> Self {
        self.user_id = user_id;
        self
    }

    pub fn action(mut self, action: AuditAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn resource_type(mut self, resource_type: ResourceType) -> Self {
        self.resource_type = Some(resource_type);
        self
    }

    pub fn resource_id(mut self, resource_id: Option<Uuid>) -> Self {
        self.resource_id = resource_id;
        self
    }

    pub fn changes(mut self, changes: JsonValue) -> Self {
        self.changes = Some(changes);
        self
    }

    pub fn metadata(mut self, metadata: JsonValue) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn ip_address(mut self, ip_address: impl Into<String>) -> Self {
        self.ip_address = Some(ip_address.into());
        self
    }

    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Build the CreateAuditEntry
    ///
    /// # Panics
    /// Panics if action or resource_type are not set. Use `try_build()` for fallible construction.
    pub fn build(self) -> CreateAuditEntry {
        self.try_build()
            .expect("AuditEntryBuilder: action and resource_type are required")
    }

    /// Try to build the CreateAuditEntry, returning an error if required fields are missing
    pub fn try_build(self) -> Result<CreateAuditEntry, &'static str> {
        let action = self.action.ok_or("action is required")?;
        let resource_type = self.resource_type.ok_or("resource_type is required")?;

        Ok(CreateAuditEntry {
            user_id: self.user_id,
            action,
            resource_type,
            resource_id: self.resource_id,
            changes: self.changes,
            metadata: self.metadata,
            ip_address: self.ip_address,
            user_agent: self.user_agent,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_action_as_str() {
        assert_eq!(AuditAction::Create.as_str(), "create");
        assert_eq!(AuditAction::Update.as_str(), "update");
        assert_eq!(AuditAction::Delete.as_str(), "delete");
    }

    #[test]
    fn test_resource_type_as_str() {
        assert_eq!(ResourceType::Organization.as_str(), "organization");
        assert_eq!(ResourceType::DataSource.as_str(), "data_source");
        assert_eq!(ResourceType::Version.as_str(), "version");
    }

    #[test]
    fn test_audit_entry_builder() {
        let entry = CreateAuditEntry::builder()
            .action(AuditAction::Create)
            .resource_type(ResourceType::Organization)
            .user_id(Some(Uuid::new_v4()))
            .ip_address("192.168.1.1")
            .build();

        assert_eq!(entry.action, AuditAction::Create);
        assert_eq!(entry.resource_type, ResourceType::Organization);
    }

    #[test]
    fn test_action_serialization() {
        let json = serde_json::to_string(&AuditAction::Create).unwrap();
        assert_eq!(json, r#""create""#);

        let action: AuditAction = serde_json::from_str(r#""update""#).unwrap();
        assert_eq!(action, AuditAction::Update);
    }

    #[test]
    fn test_resource_type_serialization() {
        let json = serde_json::to_string(&ResourceType::DataSource).unwrap();
        assert_eq!(json, r#""data_source""#);

        let resource: ResourceType = serde_json::from_str(r#""organization""#).unwrap();
        assert_eq!(resource, ResourceType::Organization);
    }
}
