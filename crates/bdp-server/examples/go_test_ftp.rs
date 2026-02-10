// Simple FTP download test for Gene Ontology

use anyhow::Result;
use suppaftp::{FtpStream, Mode};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Testing FTP download from EBI");

    // Test FTP connection in blocking thread
    let result = tokio::task::spawn_blocking(move || -> Result<usize> {
        info!("Connecting to ftp.ebi.ac.uk...");
        let mut ftp_stream = FtpStream::connect("ftp.ebi.ac.uk:21")?;

        info!("Logging in...");
        ftp_stream.login("anonymous", "anonymous@")?;

        info!("Enabling passive mode...");
        ftp_stream.set_mode(Mode::Passive);

        info!("Downloading test file: /pub/databases/GO/goa/current_release_numbers.txt");
        let cursor =
            ftp_stream.retr_as_buffer("/pub/databases/GO/goa/current_release_numbers.txt")?;

        let data = cursor.into_inner();
        let size = data.len();

        info!("Downloaded {} bytes", size);
        info!("Content:\n{}", String::from_utf8_lossy(&data));

        ftp_stream.quit()?;

        Ok(size)
    })
    .await??;

    info!("✓ FTP download successful: {} bytes", result);

    // Now test downloading from UNIPROT directory
    info!("\nTesting UNIPROT directory listing...");

    let file_list = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
        let mut ftp_stream = FtpStream::connect("ftp.ebi.ac.uk:21")?;
        ftp_stream.login("anonymous", "anonymous@")?;
        ftp_stream.set_mode(Mode::Passive);

        info!("Listing /pub/databases/GO/goa/UNIPROT/");
        let files = ftp_stream.nlst(Some("/pub/databases/GO/goa/UNIPROT/"))?;

        ftp_stream.quit()?;

        Ok(files)
    })
    .await??;

    info!("Found {} files in UNIPROT directory:", file_list.len());
    for (i, file) in file_list.iter().take(10).enumerate() {
        info!("  {}: {}", i + 1, file);
    }

    if file_list.len() > 10 {
        info!("  ... and {} more files", file_list.len() - 10);
    }

    // Check for goa_uniprot_all.gaf.gz
    if file_list
        .iter()
        .any(|f| f.contains("goa_uniprot_all.gaf.gz"))
    {
        info!("\n✓ Found goa_uniprot_all.gaf.gz");
    } else {
        info!("\n✗ goa_uniprot_all.gaf.gz not found");
    }

    info!("\n=== FTP Test Complete ===");

    Ok(())
}
