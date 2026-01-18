//! Audit middleware re-exports
//!
//! This module re-exports the audit middleware from the audit module.
//! The actual implementation is in `crate::audit::middleware`.
//!
//! # Usage
//!
//! ```no_run
//! use bdp_server::middleware::audit::AuditLayer;
//! use sqlx::PgPool;
//!
//! # async fn example(pool: PgPool) {
//! let audit_layer = AuditLayer::new(pool);
//! # }
//! ```

pub use crate::audit::AuditLayer;
