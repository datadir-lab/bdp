//! Common utilities shared across ingestion pipelines
//!
//! This module provides reusable components for data ingestion:
//!
//! - **ftp**: Shared FTP download utilities with retry logic
//! - **decompression**: Common decompression helpers for gzip, tar, zip
//! - **version_discovery**: Generic version discovery trait and utilities

pub mod decompression;
pub mod ftp;
pub mod version_discovery;
