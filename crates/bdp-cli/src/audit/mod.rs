//! Audit trail system for BDP CLI
//!
//! Provides local audit logging for regulatory compliance and research documentation.
//!
//! **IMPORTANT**: The audit trail is stored locally in SQLite and is EDITABLE.
//! It is intended for research documentation and report generation, NOT legal evidence.

pub mod logger;
pub mod machine_id;
pub mod middleware;
pub mod schema;
pub mod types;

pub use logger::{AuditLogger, LocalAuditLogger};
pub use machine_id::get_machine_id;
pub use middleware::execute_with_audit;
pub use types::{AuditEvent, EventType};
