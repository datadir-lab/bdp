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
    search_url_with_filters(base_url, query, None, None, None, None, page, page_size)
}

/// Build search URL with full filter support
#[allow(clippy::too_many_arguments)]
pub fn search_url_with_filters(
    base_url: &str,
    query: &str,
    type_filter: Option<&[String]>,
    source_type_filter: Option<&[String]>,
    organism: Option<&str>,
    format: Option<&str>,
    page: Option<i32>,
    page_size: Option<i32>,
) -> String {
    use urlencoding::encode;

    let mut url = format!("{}/api/v1/search?query={}", base_url, encode(query));

    if let Some(types) = type_filter {
        for t in types {
            url.push_str(&format!("&type_filter={}", encode(t)));
        }
    }

    if let Some(source_types) = source_type_filter {
        for st in source_types {
            url.push_str(&format!("&source_type_filter={}", encode(st)));
        }
    }

    if let Some(org) = organism {
        url.push_str(&format!("&organism={}", encode(org)));
    }

    if let Some(fmt) = format {
        url.push_str(&format!("&format={}", encode(fmt)));
    }

    if let Some(p) = page {
        url.push_str(&format!("&page={}", p));
    }

    if let Some(ps) = page_size {
        url.push_str(&format!("&per_page={}", ps));
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
        assert_eq!(url, "http://localhost:8000/api/v1/search?query=insulin");

        let url_with_pagination = search_url("http://localhost:8000", "insulin", Some(2), Some(20));
        assert_eq!(
            url_with_pagination,
            "http://localhost:8000/api/v1/search?query=insulin&page=2&per_page=20"
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
