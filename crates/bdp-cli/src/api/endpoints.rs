//! API endpoint URL builders
//!
//! Helper functions to construct API endpoint URLs.

/// Build resolve endpoint URL
pub fn resolve_url(base_url: &str) -> String {
    format!("{}/api/v1/resolve", base_url)
}

/// Build data source download URL
pub fn data_source_download_url(
    base_url: &str,
    org: &str,
    name: &str,
    version: &str,
    format: &str,
) -> String {
    format!(
        "{}/api/v1/data-sources/{}/{}/{}/download?format={}",
        base_url, org, name, version, format
    )
}

/// Build data source details URL
pub fn data_source_details_url(base_url: &str, org: &str, name: &str, version: &str) -> String {
    format!("{}/api/v1/data-sources/{}/{}/{}", base_url, org, name, version)
}

/// Build search URL
pub fn search_url(
    base_url: &str,
    query: &str,
    page: Option<i32>,
    page_size: Option<i32>,
) -> String {
    let mut url = format!("{}/api/v1/search?q={}", base_url, query);

    if let Some(p) = page {
        url.push_str(&format!("&page={}", p));
    }

    if let Some(ps) = page_size {
        url.push_str(&format!("&page_size={}", ps));
    }

    url
}

/// Build organization list URL
pub fn organizations_url(base_url: &str) -> String {
    format!("{}/api/v1/organizations", base_url)
}

/// Build organization details URL
pub fn organization_details_url(base_url: &str, name: &str) -> String {
    format!("{}/api/v1/organizations/{}", base_url, name)
}

/// Build health check URL
pub fn health_url(base_url: &str) -> String {
    format!("{}/health", base_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_url() {
        let url = resolve_url("http://localhost:8000");
        assert_eq!(url, "http://localhost:8000/api/v1/resolve");
    }

    #[test]
    fn test_data_source_download_url() {
        let url =
            data_source_download_url("http://localhost:8000", "uniprot", "P01308", "1.0", "fasta");
        assert_eq!(
            url,
            "http://localhost:8000/api/v1/data-sources/uniprot/P01308/1.0/download?format=fasta"
        );
    }

    #[test]
    fn test_data_source_details_url() {
        let url = data_source_details_url("http://localhost:8000", "uniprot", "P01308", "1.0");
        assert_eq!(url, "http://localhost:8000/api/v1/data-sources/uniprot/P01308/1.0");
    }

    #[test]
    fn test_search_url() {
        let url = search_url("http://localhost:8000", "insulin", None, None);
        assert_eq!(url, "http://localhost:8000/api/v1/search?q=insulin");

        let url_with_pagination = search_url("http://localhost:8000", "insulin", Some(2), Some(20));
        assert_eq!(
            url_with_pagination,
            "http://localhost:8000/api/v1/search?q=insulin&page=2&page_size=20"
        );
    }

    #[test]
    fn test_organizations_url() {
        let url = organizations_url("http://localhost:8000");
        assert_eq!(url, "http://localhost:8000/api/v1/organizations");
    }

    #[test]
    fn test_health_url() {
        let url = health_url("http://localhost:8000");
        assert_eq!(url, "http://localhost:8000/health");
    }
}
