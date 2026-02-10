//! CLI command implementations
//!
//! Each subcommand has its own module with a `run` function that returns
//! a result struct implementing [`output::Render`]. The caller (main.rs)
//! invokes `.render()` to display output, keeping business logic separate
//! from presentation.

pub mod output;

pub mod audit;
pub mod cache_cmd;
pub mod clean;
pub mod config;
pub mod generate;
pub mod init;
pub mod pull;
pub mod query;
pub mod search;
pub mod source;
pub mod status;
pub mod uninstall;
