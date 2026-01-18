//! Progress bar utilities for CLI operations
//!
//! Provides progress indicators for downloads and long-running operations.

use indicatif::{ProgressBar, ProgressStyle};

/// Create a progress bar for file downloads
pub fn create_download_progress(size: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .expect("Invalid progress bar template")
            .progress_chars("#>-"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Create a spinner for indeterminate operations
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Invalid spinner template"),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Create a simple progress bar with custom message
pub fn create_progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")
            .expect("Invalid progress bar template")
            .progress_chars("#>-"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Format bytes into human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size as u64, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
        assert_eq!(format_bytes(1099511627776), "1.00 TB");
    }

    #[test]
    fn test_create_download_progress() {
        let pb = create_download_progress(1024, "Downloading file");
        assert_eq!(pb.length(), Some(1024));
    }

    #[test]
    fn test_create_spinner() {
        let pb = create_spinner("Processing...");
        assert!(pb.is_finished() == false);
        pb.finish();
    }

    #[test]
    fn test_create_progress_bar() {
        let pb = create_progress_bar(100, "Processing items");
        assert_eq!(pb.length(), Some(100));
    }
}
