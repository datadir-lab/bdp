use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};
use sqlx::PgPool;
use sqlx::Row;
use tracing::info;
use uuid::Uuid;

/// List all .xml.gz filenames from the PubMed FTP directory HTML.
pub async fn list_pubmed_files(client: &Client, base_url: &str) -> Result<Vec<String>> {
    let html = client.get(base_url).send().await?.text().await?;
    let doc = Html::parse_document(&html);
    let sel = Selector::parse("a[href]").map_err(|e| anyhow::anyhow!("selector: {:?}", e))?;
    let files: Vec<String> = doc
        .select(&sel)
        .filter_map(|el| el.value().attr("href"))
        .filter(|href| href.ends_with(".xml.gz"))
        .map(|href| {
            if href.starts_with("http") {
                href.to_string()
            } else {
                format!("{}{}", base_url.trim_end_matches('/'), href)
            }
        })
        .collect();
    info!(count = files.len(), "listed PubMed files");
    Ok(files)
}

/// Register pending files in pubmed_ingest_files table (skips already-registered files).
pub async fn register_pending_files(
    pool: &PgPool,
    filenames: &[String],
    _org_id: Uuid, // pubmed_ingest_files is not org-scoped; org_id reserved for future use
) -> Result<usize> {
    let mut registered = 0usize;
    for filename in filenames {
        let result = sqlx::query(
            "INSERT INTO pubmed_ingest_files (filename, status) VALUES ($1, 'pending') ON CONFLICT (filename) DO NOTHING",
        )
        .bind(filename)
        .execute(pool)
        .await;
        match result {
            Ok(r) => registered += r.rows_affected() as usize,
            Err(e) => tracing::warn!("failed to register {}: {}", filename, e),
        }
    }
    Ok(registered)
}

/// Fetch (id, filename) tuples with status='pending' for processing.
pub async fn get_pending_files(pool: &PgPool) -> Result<Vec<(i64, String)>> {
    let rows = sqlx::query(
        "SELECT id, filename FROM pubmed_ingest_files WHERE status = 'pending' ORDER BY id",
    )
    .fetch_all(pool)
    .await?;
    let mut results = Vec::with_capacity(rows.len());
    for row in &rows {
        let id: i64 = row.try_get("id")?;
        let filename: String = row.try_get("filename")?;
        results.push((id, filename));
    }
    Ok(results)
}
