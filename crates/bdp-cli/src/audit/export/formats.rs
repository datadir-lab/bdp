//! Export format definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported export formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// FDA 21 CFR Part 11 compliance report (JSON)
    Fda,
    /// NIH Data Management & Sharing report (Markdown)
    Nih,
    /// EMA ALCOA++ compliance report (YAML)
    Ema,
    /// Data Availability Statement (Markdown)
    Das,
    /// Raw JSON export
    Json,
}

impl ExportFormat {
    /// Get file extension for this format
    pub fn extension(&self) -> &str {
        match self {
            ExportFormat::Fda => "json",
            ExportFormat::Nih => "md",
            ExportFormat::Ema => "yaml",
            ExportFormat::Das => "md",
            ExportFormat::Json => "json",
        }
    }

    /// Get default filename for this format
    pub fn default_filename(&self) -> String {
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        match self {
            ExportFormat::Fda => format!("audit-fda-{}.json", timestamp),
            ExportFormat::Nih => format!("audit-nih-{}.md", timestamp),
            ExportFormat::Ema => format!("audit-ema-{}.yaml", timestamp),
            ExportFormat::Das => "data-availability.md".to_string(),
            ExportFormat::Json => format!("audit-{}.json", timestamp),
        }
    }

    /// Get format name as string
    pub fn as_str(&self) -> &str {
        match self {
            ExportFormat::Fda => "fda",
            ExportFormat::Nih => "nih",
            ExportFormat::Ema => "ema",
            ExportFormat::Das => "das",
            ExportFormat::Json => "json",
        }
    }
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ExportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fda" => Ok(ExportFormat::Fda),
            "nih" => Ok(ExportFormat::Nih),
            "ema" => Ok(ExportFormat::Ema),
            "das" => Ok(ExportFormat::Das),
            "json" => Ok(ExportFormat::Json),
            _ => Err(format!(
                "Invalid export format: {}. Valid formats: fda, nih, ema, das, json",
                s
            )),
        }
    }
}

/// Export options
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Output file path
    pub output: PathBuf,

    /// Start date for export (optional)
    pub from: Option<DateTime<Utc>>,

    /// End date for export (optional)
    pub to: Option<DateTime<Utc>>,

    /// Project name (from manifest)
    pub project_name: Option<String>,

    /// Project version (from manifest)
    pub project_version: Option<String>,
}

impl ExportOptions {
    /// Create default export options with output path
    pub fn new(output: PathBuf) -> Self {
        Self {
            output,
            from: None,
            to: None,
            project_name: None,
            project_version: None,
        }
    }

    /// Set date range
    pub fn with_range(mut self, from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        self.from = Some(from);
        self.to = Some(to);
        self
    }

    /// Set project metadata
    pub fn with_project(mut self, name: String, version: String) -> Self {
        self.project_name = Some(name);
        self.project_version = Some(version);
        self
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_from_str() {
        assert_eq!("fda".parse::<ExportFormat>().unwrap(), ExportFormat::Fda);
        assert_eq!("FDA".parse::<ExportFormat>().unwrap(), ExportFormat::Fda);
        assert_eq!("nih".parse::<ExportFormat>().unwrap(), ExportFormat::Nih);
        assert_eq!("ema".parse::<ExportFormat>().unwrap(), ExportFormat::Ema);
        assert_eq!("das".parse::<ExportFormat>().unwrap(), ExportFormat::Das);
        assert_eq!("json".parse::<ExportFormat>().unwrap(), ExportFormat::Json);

        assert!("invalid".parse::<ExportFormat>().is_err());
    }

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::Fda.extension(), "json");
        assert_eq!(ExportFormat::Nih.extension(), "md");
        assert_eq!(ExportFormat::Ema.extension(), "yaml");
        assert_eq!(ExportFormat::Das.extension(), "md");
        assert_eq!(ExportFormat::Json.extension(), "json");
    }

    #[test]
    fn test_default_filename() {
        let fda_filename = ExportFormat::Fda.default_filename();
        assert!(fda_filename.starts_with("audit-fda-"));
        assert!(fda_filename.ends_with(".json"));

        let das_filename = ExportFormat::Das.default_filename();
        assert_eq!(das_filename, "data-availability.md");
    }
}
