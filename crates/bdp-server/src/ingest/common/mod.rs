//! Common utilities shared across ingestion pipelines
//!
//! This module provides reusable components for data ingestion:
//!
//! - **ftp**: Shared FTP download utilities with retry logic
//! - **decompression**: Common decompression helpers for gzip, tar, zip

pub mod ftp;
pub mod decompression;
