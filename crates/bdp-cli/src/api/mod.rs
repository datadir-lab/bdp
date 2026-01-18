//! API client module
//!
//! HTTP client for interacting with the BDP backend server.

pub mod client;
pub mod endpoints;
pub mod types;

pub use client::ApiClient;
pub use types::*;
