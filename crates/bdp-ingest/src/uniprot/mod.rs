//! UniProt data ingestion

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use tracing::{info, warn};

const UNIPROT_RELEASE_URL: &str = "https://ftp.uniprot.org/pub/databases/uniprot/current_release";

/// Ingest UniProt data
pub async fn ingest(output_dir: &str, version: Option<&str>) -> Result<()> {
    let output_path = Path::new(output_dir);
    std::fs::create_dir_all(output_path)?;

    info!("Output directory: {}", output_path.display());

    let version = match version {
        Some(v) => v.to_string(),
        None => {
            info!("Fetching latest UniProt version...");
            fetch_latest_version().await?
        },
    };

    info!("UniProt version: {}", version);

    // Download release notes
    download_release_notes(&version, output_path).await?;

    // Download datasets
    download_datasets(&version, output_path).await?;

    Ok(())
}

/// Fetch the latest UniProt version
async fn fetch_latest_version() -> Result<String> {
    let url = format!("{}/RELEASE.metalink", UNIPROT_RELEASE_URL);
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch latest version: {}", response.status());
    }

    // Parse version from response
    // TODO: Implement actual parsing
    warn!("Version detection not fully implemented, using placeholder");
    Ok("2024_01".to_string())
}

/// Download release notes
async fn download_release_notes(version: &str, output_dir: &Path) -> Result<()> {
    info!("Downloading release notes for version {}", version);

    let url = format!("{}/relnotes.txt", UNIPROT_RELEASE_URL);
    let output_file = output_dir.join(format!("relnotes_{}.txt", version));

    download_file(&url, &output_file).await?;

    info!("Release notes saved to {}", output_file.display());
    Ok(())
}

/// Download UniProt datasets
async fn download_datasets(version: &str, output_dir: &Path) -> Result<()> {
    info!("Downloading UniProt datasets for version {}", version);

    let datasets = vec![
        "knowledgebase/complete/uniprot_sprot.xml.gz",
        "knowledgebase/complete/uniprot_trembl.xml.gz",
    ];

    for dataset in datasets {
        let url = format!("{}/{}", UNIPROT_RELEASE_URL, dataset);
        let filename = dataset.split('/').next_back().unwrap();
        let output_file = output_dir.join(filename);

        info!("Downloading {} ...", filename);
        download_file(&url, &output_file).await?;
        info!("Downloaded {}", output_file.display());
    }

    Ok(())
}

/// Download a file with progress bar
async fn download_file(url: &str, output_path: &Path) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download {}: {}", url, response.status());
    }

    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("#>-"),
    );
    pb.set_message(format!("Downloading {}", output_path.file_name().unwrap().to_string_lossy()));

    let mut file = std::fs::File::create(output_path)?;
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();

    use futures::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        std::io::Write::write_all(&mut file, &chunk)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message(format!(
        "Downloaded {}",
        output_path.file_name().unwrap().to_string_lossy()
    ));

    Ok(())
}
