//! API request and response types
//!
//! Matches the backend API structure.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request to resolve manifest dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveRequest {
    pub sources: Vec<String>,
    pub tools: Vec<String>,
}

/// Response from resolve endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedManifest {
    pub sources: HashMap<String, ResolvedSource>,
    pub tools: HashMap<String, ResolvedTool>,
}

/// A resolved source entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedSource {
    /// Resolved specification (org:name@version)
    pub resolved: String,

    /// Format (fasta, gtf, etc.)
    pub format: String,

    /// SHA-256 checksum
    pub checksum: String,

    /// File size in bytes
    pub size: i64,

    /// External version string
    pub external_version: String,

    /// Download URL (presigned if from S3)
    pub download_url: String,

    /// Number of dependencies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_count: Option<i32>,
}

/// A resolved tool entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedTool {
    /// Resolved specification (org:name@version)
    pub resolved: String,

    /// Tool version
    pub version: String,

    /// Download URL
    pub url: String,

    /// SHA-256 checksum
    pub checksum: String,

    /// Size in bytes
    pub size: i64,
}

/// Data source details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub id: String,
    pub organization: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub format: String,
    pub file_size: i64,
    pub checksum: String,
    pub external_version: Option<String>,
    pub is_aggregate: bool,
    pub dependency_count: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

/// Organization details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub logo_url: Option<String>,
    pub is_system: bool,
}

/// Search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
}

/// A single search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub organization: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub format: String,
    pub entry_type: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_serialization() {
        let response = ApiResponse {
            success: true,
            data: "test data".to_string(),
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":\"test data\""));
    }

    #[test]
    fn test_resolve_request() {
        let request = ResolveRequest {
            sources: vec!["uniprot:P01308-fasta@1.0".to_string()],
            tools: vec!["ncbi:blast@2.14.0".to_string()],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("uniprot:P01308-fasta@1.0"));
        assert!(json.contains("ncbi:blast@2.14.0"));
    }
}
