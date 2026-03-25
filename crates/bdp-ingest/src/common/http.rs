// crates/bdp-ingest/src/common/http.rs

use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, info};

/// Download a URL to a String, with retry.
///
/// Tries up to `max_retries` times with exponential backoff.
pub async fn download_text(url: &str, max_retries: u32) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(300))
        .user_agent("bdp-ingest/0.1 (https://github.com/datadir-lab/bdp)")
        .build()
        .context("failed to build HTTP client")?;

    let mut last_error = None;
    for attempt in 0..=max_retries {
        if attempt > 0 {
            let backoff = Duration::from_secs(2u64.pow(attempt));
            info!("Retry {}/{} for {} (backoff: {:?})", attempt, max_retries, url, backoff);
            tokio::time::sleep(backoff).await;
        }

        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    let text = resp.text().await.context("failed to read response body")?;
                    debug!("Downloaded {} bytes from {}", text.len(), url);
                    return Ok(text);
                }
                last_error = Some(anyhow::anyhow!("HTTP {}: {}", status, url));
            }
            Err(e) => {
                last_error = Some(anyhow::anyhow!("request failed: {}: {}", url, e));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("download failed: {}", url)))
}

/// Download a URL to bytes, with retry.
pub async fn download_bytes(url: &str, max_retries: u32) -> Result<bytes::Bytes> {
    let client = Client::builder()
        .timeout(Duration::from_secs(600))
        .user_agent("bdp-ingest/0.1 (https://github.com/datadir-lab/bdp)")
        .build()
        .context("failed to build HTTP client")?;

    let mut last_error = None;
    for attempt in 0..=max_retries {
        if attempt > 0 {
            let backoff = Duration::from_secs(2u64.pow(attempt));
            tokio::time::sleep(backoff).await;
        }

        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                return resp.bytes().await.context("failed to read response body");
            }
            Ok(resp) => {
                last_error = Some(anyhow::anyhow!("HTTP {}: {}", resp.status(), url));
            }
            Err(e) => {
                last_error = Some(anyhow::anyhow!("{}", e));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("download failed: {}", url)))
}
