//! Audit logging module
//!
//! This module provides comprehensive audit trail functionality for tracking
//! commands (write operations) in the system. Queries (read operations) are
//! not audited to reduce noise and improve performance.
//!
//! # Architecture
//!
//! This module implements audit logging following CQRS principles:
//! - **Commands** (POST, PUT, PATCH, DELETE) are audited
//! - **Queries** (GET) are not audited
//! - Captures request body for commands
//! - Extracts user info from auth headers
//! - Logs after successful command execution
//!
//! # Usage
//!
//! ```no_run
//! use axum::Router;
//! use sqlx::PgPool;
//! use tower::ServiceBuilder;
//! use bdp_server::audit;
//!
//! # async fn example(pool: PgPool) {
//! let app = Router::new()
//!     .layer(ServiceBuilder::new().layer(audit::AuditLayer::new(pool.clone())));
//! # }
//! ```
//!
//! # Example: Manual Audit Logging
//!
//! ```no_run
//! use bdp_server::audit::{CreateAuditEntry, AuditAction, ResourceType, create_audit_entry};
//! use sqlx::PgPool;
//! use uuid::Uuid;
//!
//! # async fn example(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
//! let entry = CreateAuditEntry::builder()
//!     .action(AuditAction::Create)
//!     .resource_type(ResourceType::Organization)
//!     .resource_id(Some(Uuid::new_v4()))
//!     .user_id(Some(Uuid::new_v4()))
//!     .ip_address("192.168.1.1")
//!     .build();
//!
//! let audit_log = create_audit_entry(pool, entry).await?;
//! println!("Created audit log: {}", audit_log.id);
//! # Ok(())
//! # }
//! ```

mod middleware;
mod models;
mod queries;

#[cfg(test)]
mod middleware_tests;

pub use middleware::AuditLayer;
pub use models::{
    AuditAction, AuditEntry, AuditEntryBuilder, AuditQuery, CreateAuditEntry, ResourceType,
};
pub use queries::{create_audit_entry, get_audit_trail, get_user_audit_logs, query_audit_logs};
