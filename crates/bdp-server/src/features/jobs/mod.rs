//! Jobs feature module
//!
//! Provides public read-only access to job status and sync progress.
//! NO authentication required, NO job triggers allowed.

pub mod queries;
pub mod routes;

#[cfg(test)]
mod routes_test;

pub use routes::jobs_routes;
