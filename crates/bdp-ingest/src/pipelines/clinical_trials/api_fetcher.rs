use anyhow::{Context, Result};
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;
use tracing::info;

#[derive(Deserialize)]
struct ApiPage {
    studies: Vec<serde_json::Value>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
    #[serde(rename = "totalCount")]
    total_count: Option<u64>,
}

pub async fn fetch_updated_studies(
    client: &Client,
    base_url: &str,
    from_date: NaiveDate,
    page_size: u32,
    max_retries: u32,
) -> Result<Vec<serde_json::Value>> {
    let date_str = from_date.format("%Y-%m-%d").to_string();
    let filter = format!("AREA[LastUpdatePostDate]RANGE[{date_str},MAX]");
    let mut all_studies = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url = format!(
            "{base_url}/studies?query.term={filter}&pageSize={page_size}&format=json"
        );
        if let Some(ref token) = page_token {
            url.push_str(&format!("&pageToken={token}"));
        }

        let mut last_err = anyhow::anyhow!("no attempts");
        let page: ApiPage = 'retry: {
            for attempt in 0..=max_retries {
                match client.get(&url).send().await {
                    Ok(r) => {
                        let text = r.text().await.context("reading CT API response")?;
                        match serde_json::from_str(&text) {
                            Ok(p) => break 'retry p,
                            Err(e) => last_err = e.into(),
                        }
                    }
                    Err(e) => last_err = e.into(),
                }
                if attempt < max_retries {
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt))).await;
                }
            }
            return Err(last_err);
        };

        let count = page.studies.len();
        all_studies.extend(page.studies);
        info!(
            fetched = all_studies.len(),
            total = page.total_count,
            page_count = count,
            "CT API page"
        );

        match page.next_page_token {
            Some(t) if !t.is_empty() => page_token = Some(t),
            _ => break,
        }
    }

    Ok(all_studies)
}
