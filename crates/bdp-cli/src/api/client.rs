//! HTTP API client for BDP server
//!
//! Provides methods to interact with the BDP backend API.

use crate::api::{endpoints, types::*};
use crate::error::{CliError, Result};
use crate::manifest::Manifest;
use reqwest::Client;
use std::time::Duration;

// ============================================================================
// API Client Constants
// ============================================================================

/// Default timeout for API requests in seconds.
/// Can be overridden via BDP_API_TIMEOUT_SECS environment variable.
/// Set to 5 minutes to accommodate large file downloads.
pub const DEFAULT_API_TIMEOUT_SECS: u64 = 300;

/// Default BDP server URL when not specified via environment variable.
pub const DEFAULT_SERVER_URL: &str = "http://localhost:8000";

/// API client for BDP server
pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(base_url: String) -> Result<Self> {
        let timeout_secs = std::env::var("BDP_API_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_API_TIMEOUT_SECS);

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        Ok(Self { client, base_url })
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self> {
        let base_url =
            std::env::var("BDP_SERVER_URL").unwrap_or_else(|_| DEFAULT_SERVER_URL.to_string());

        Self::new(base_url)
    }

    /// Check server health
    pub async fn health_check(&self) -> Result<bool> {
        let url = endpoints::health_url(&self.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Resolve manifest dependencies
    pub async fn resolve_manifest(&self, manifest: &Manifest) -> Result<ResolvedManifest> {
        let url = endpoints::resolve_url(&self.base_url);

        let request = ResolveRequest {
            sources: manifest.sources.clone(),
            tools: manifest.tools.clone(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        let api_response: ApiResponse<ResolvedManifest> = response.json().await?;

        if !api_response.success {
            return Err(CliError::api(
                api_response.error.unwrap_or_else(|| {
                    "Failed to resolve manifest dependencies. Check that all source specifications are valid and available.".to_string()
                }),
            ));
        }

        Ok(api_response.data)
    }

    /// Download a file from the server
    ///
    /// Returns the file bytes
    pub async fn download_file(
        &self,
        org: &str,
        name: &str,
        version: &str,
        format: &str,
    ) -> Result<Vec<u8>> {
        let url = endpoints::data_source_download_url(&self.base_url, org, name, version, format);

        let response = self.client.get(&url).send().await?.error_for_status()?;

        let bytes = response.bytes().await?.to_vec();

        Ok(bytes)
    }

    /// Get data source details
    pub async fn get_data_source(
        &self,
        org: &str,
        name: &str,
        version: &str,
    ) -> Result<DataSource> {
        let url = endpoints::data_source_details_url(&self.base_url, org, name, version);

        let response = self.client.get(&url).send().await?.error_for_status()?;

        let api_response: ApiResponse<DataSource> = response.json().await?;

        if !api_response.success {
            return Err(CliError::api(
                api_response.error.unwrap_or_else(|| {
                    format!(
                        "Data source '{}/{}@{}' not found or unavailable. Run 'bdp search {}' to find available sources.",
                        org, name, version, name
                    )
                }),
            ));
        }

        Ok(api_response.data)
    }

    /// Search for data sources with filters
    pub async fn search(
        &self,
        query: &str,
        page: Option<i32>,
        page_size: Option<i32>,
    ) -> Result<SearchResponse> {
        self.search_with_filters(query, None, None, None, None, page, page_size)
            .await
    }

    /// Search for data sources with full filter support
    #[allow(clippy::too_many_arguments)]
    pub async fn search_with_filters(
        &self,
        query: &str,
        type_filter: Option<Vec<String>>,
        source_type_filter: Option<Vec<String>>,
        organism: Option<String>,
        format: Option<String>,
        page: Option<i32>,
        page_size: Option<i32>,
    ) -> Result<SearchResponse> {
        let url = endpoints::search_url_with_filters(
            &self.base_url,
            query,
            type_filter.as_deref(),
            source_type_filter.as_deref(),
            organism.as_deref(),
            format.as_deref(),
            page,
            page_size,
        );

        let response = self.client.get(&url).send().await?.error_for_status()?;

        let api_response: ApiResponse<SearchResponse> = response.json().await?;

        if !api_response.success {
            return Err(CliError::api(
                api_response.error.unwrap_or_else(|| {
                    format!(
                        "Search for '{}' failed. Try a different search term or check your server connection.",
                        query
                    )
                }),
            ));
        }

        Ok(api_response.data)
    }

    /// List all organizations
    pub async fn list_organizations(&self) -> Result<Vec<Organization>> {
        let url = endpoints::organizations_url(&self.base_url);

        let response = self.client.get(&url).send().await?.error_for_status()?;

        let api_response: ApiResponse<Vec<Organization>> = response.json().await?;

        if !api_response.success {
            return Err(CliError::api(api_response.error.unwrap_or_else(|| {
                "Failed to list organizations. Check your server connection.".to_string()
            })));
        }

        Ok(api_response.data)
    }

    /// Get organization details
    pub async fn get_organization(&self, name: &str) -> Result<Organization> {
        let url = endpoints::organization_details_url(&self.base_url, name);

        let response = self.client.get(&url).send().await?.error_for_status()?;

        let api_response: ApiResponse<Organization> = response.json().await?;

        if !api_response.success {
            return Err(CliError::api(
                api_response.error.unwrap_or_else(|| {
                    format!(
                        "Organization '{}' not found. Run 'bdp org list' to see available organizations.",
                        name
                    )
                }),
            ));
        }

        Ok(api_response.data)
    }

    /// Execute a SQL query
    pub async fn execute_query(&self, sql: String) -> Result<QueryResults> {
        let url = format!("{}/api/v1/query", self.base_url);

        let request = QueryRequest { sql };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        let api_response: ApiResponse<QueryResults> = response.json().await?;

        if !api_response.success {
            return Err(CliError::api(api_response.error.unwrap_or_else(|| {
                "Query execution failed. Check your SQL syntax and try again.".to_string()
            })));
        }

        Ok(api_response.data)
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_creation() {
        let client = ApiClient::new("http://localhost:8000".to_string()).unwrap();
        assert_eq!(client.base_url(), "http://localhost:8000");
    }

    #[test]
    fn test_api_client_from_env() {
        std::env::set_var("BDP_SERVER_URL", "http://test.example.com");
        let client = ApiClient::from_env().unwrap();
        assert_eq!(client.base_url(), "http://test.example.com");
        std::env::remove_var("BDP_SERVER_URL");
    }

    #[tokio::test]
    async fn test_health_check_unreachable() {
        let client = ApiClient::new("http://localhost:9999".to_string()).unwrap();
        let result = client.health_check().await.unwrap();
        assert!(!result);
    }
}
