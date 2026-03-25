// Lists and downloads Parquet files from an Open Targets directory listing.
// Open Targets FTP directories return HTML with <a href="*.parquet"> links.

use anyhow::{Context, Result};
use bytes::Bytes;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, info};

/// Return all `.parquet` hrefs found in an HTML directory listing.
pub async fn list_parquet_files(client: &Client, url: &str) -> Result<Vec<String>> {
    let html = client
        .get(url)
        .send()
        .await
        .context("listing Open Targets directory")?
        .text()
        .await?;

    let doc = Html::parse_document(&html);
    let sel =
        Selector::parse("a[href]").map_err(|e| anyhow::anyhow!("invalid selector: {:?}", e))?;
    let files: Vec<String> = doc
        .select(&sel)
        .filter_map(|el| el.value().attr("href"))
        .filter(|href| href.ends_with(".parquet"))
        .map(|href| {
            if href.starts_with("http") {
                href.to_string()
            } else {
                format!("{}{}", url.trim_end_matches('/'), href)
            }
        })
        .collect();

    info!(count = files.len(), %url, "found parquet files");
    Ok(files)
}

/// Download a single Parquet file into memory.
pub async fn download_parquet(client: &Client, url: &str, max_retries: u32) -> Result<Bytes> {
    let mut last_err = anyhow::anyhow!("no attempts");
    for attempt in 0..=max_retries {
        match client.get(url).send().await {
            Ok(resp) => {
                let resp = resp.error_for_status()?;
                let bytes = resp.bytes().await.context("reading parquet bytes")?;
                debug!(url, bytes = bytes.len(), "downloaded parquet");
                return Ok(bytes);
            },
            Err(e) => {
                last_err = anyhow::anyhow!("{}", e);
                if attempt < max_retries {
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt))).await;
                }
            },
        }
    }
    Err(last_err).context(format!("downloading {url}"))
}
