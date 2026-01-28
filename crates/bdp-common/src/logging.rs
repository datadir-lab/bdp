//! Logging Configuration and Initialization
//!
//! This module provides a centralized logging system for all BDP components,
//! similar to Serilog in .NET. It supports:
//!
//! - Multiple output targets (console, file, both)
//! - Multiple log formats (text, JSON)
//! - Configurable log levels
//! - Log file rotation (daily)
//! - Environment-based configuration
//!
//! # Best Practices
//!
//! 1. **NEVER use `println!`, `eprintln!`, or `dbg!` macros**
//!    - Use structured logging macros instead: `trace!`, `debug!`, `info!`, `warn!`, `error!`
//!
//! 2. **Use appropriate log levels**:
//!    - `trace!`: Very detailed information for debugging specific issues
//!    - `debug!`: Detailed information useful during development
//!    - `info!`: General informational messages about application progress
//!    - `warn!`: Warning messages for potentially harmful situations
//!    - `error!`: Error messages for failures that need attention
//!
//! 3. **Use structured logging with fields**:
//!    ```rust
//!    use tracing::{info, error};
//!
//!    info!(user_id = %user.id, username = %user.name, "User logged in");
//!    error!(error = ?err, path = %file_path, "Failed to read file");
//!    ```
//!
//! 4. **Use spans for operations**:
//!    ```rust
//!    use tracing::{info_span, instrument};
//!
//!    #[instrument(skip(db))]
//!    async fn process_order(order_id: &str, db: &PgPool) -> Result<()> {
//!        info!("Processing order");
//!        // ... operation logic
//!        Ok(())
//!    }
//!    ```
//!
//! # Example
//!
//! ```no_run
//! use bdp_common::logging::{LogConfig, init_logging};
//! use tracing::info;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize logging from environment or use defaults
//!     let config = LogConfig::from_env()?;
//!     init_logging(&config)?;
//!
//!     info!("Application started");
//!     Ok(())
//! }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Log level for filtering messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Very detailed trace-level logging
    Trace,
    /// Debug-level logging for development
    Debug,
    /// Informational messages
    #[default]
    Info,
    /// Warning messages
    Warn,
    /// Error messages
    Error,
}

impl LogLevel {
    /// Convert to tracing Level
    pub fn to_tracing_level(self) -> Level {
        match self {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(anyhow::anyhow!("Invalid log level: {}", s)),
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

/// Output target for logs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogOutput {
    /// Output to console only
    #[default]
    Console,
    /// Output to file only
    File,
    /// Output to both console and file
    Both,
}

impl std::str::FromStr for LogOutput {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "console" | "stdout" => Ok(LogOutput::Console),
            "file" => Ok(LogOutput::File),
            "both" | "all" => Ok(LogOutput::Both),
            _ => Err(anyhow::anyhow!("Invalid log output: {}", s)),
        }
    }
}

impl std::fmt::Display for LogOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogOutput::Console => write!(f, "console"),
            LogOutput::File => write!(f, "file"),
            LogOutput::Both => write!(f, "both"),
        }
    }
}

/// Log format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Human-readable text format
    #[default]
    Text,
    /// JSON format for structured logging
    Json,
}

impl std::str::FromStr for LogFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" | "pretty" => Ok(LogFormat::Text),
            "json" => Ok(LogFormat::Json),
            _ => Err(anyhow::anyhow!("Invalid log format: {}", s)),
        }
    }
}

impl std::fmt::Display for LogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogFormat::Text => write!(f, "text"),
            LogFormat::Json => write!(f, "json"),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Minimum log level to display
    pub level: LogLevel,

    /// Output target (console, file, or both)
    pub output: LogOutput,

    /// Log format (text or JSON)
    pub format: LogFormat,

    /// Directory for log files (only used when output includes file)
    pub log_dir: PathBuf,

    /// Log file name prefix (e.g., "bdp-server" -> "bdp-server.2024-01-18.log")
    pub log_file_prefix: String,

    /// Additional filter directives (e.g., "sqlx=warn,tower_http=debug")
    /// This allows fine-tuning specific module log levels
    pub filter_directives: Option<String>,

    /// Whether to include file and line number in logs
    pub include_location: bool,

    /// Whether to include thread IDs in logs
    pub include_thread_ids: bool,

    /// Whether to include target module names in logs
    pub include_targets: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            output: LogOutput::Console,
            format: LogFormat::Text,
            log_dir: PathBuf::from("./logs"),
            log_file_prefix: "bdp".to_string(),
            filter_directives: None,
            include_location: false,
            include_thread_ids: false,
            include_targets: true,
        }
    }
}

impl LogConfig {
    /// Create a new LogConfig with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from environment variables
    ///
    /// Environment variables:
    /// - `LOG_LEVEL`: Log level (trace, debug, info, warn, error)
    /// - `LOG_OUTPUT`: Output target (console, file, both)
    /// - `LOG_FORMAT`: Log format (text, json)
    /// - `LOG_DIR`: Directory for log files
    /// - `LOG_FILE_PREFIX`: Prefix for log files
    /// - `LOG_FILTER`: Additional filter directives
    /// - `LOG_INCLUDE_LOCATION`: Include file/line in logs (true/false)
    /// - `LOG_INCLUDE_THREAD_IDS`: Include thread IDs (true/false)
    /// - `LOG_INCLUDE_TARGETS`: Include module targets (true/false)
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        if let Ok(level) = std::env::var("LOG_LEVEL") {
            config.level = level.parse()?;
        }

        if let Ok(output) = std::env::var("LOG_OUTPUT") {
            config.output = output.parse()?;
        }

        if let Ok(format) = std::env::var("LOG_FORMAT") {
            config.format = format.parse()?;
        }

        if let Ok(dir) = std::env::var("LOG_DIR") {
            config.log_dir = PathBuf::from(dir);
        }

        if let Ok(prefix) = std::env::var("LOG_FILE_PREFIX") {
            config.log_file_prefix = prefix;
        }

        if let Ok(filter) = std::env::var("LOG_FILTER") {
            config.filter_directives = Some(filter);
        }

        if let Ok(val) = std::env::var("LOG_INCLUDE_LOCATION") {
            config.include_location = val.parse().unwrap_or(false);
        }

        if let Ok(val) = std::env::var("LOG_INCLUDE_THREAD_IDS") {
            config.include_thread_ids = val.parse().unwrap_or(false);
        }

        if let Ok(val) = std::env::var("LOG_INCLUDE_TARGETS") {
            config.include_targets = val.parse().unwrap_or(true);
        }

        Ok(config)
    }

    /// Create a builder for fluent configuration
    pub fn builder() -> LogConfigBuilder {
        LogConfigBuilder::default()
    }
}

/// Builder for LogConfig
#[derive(Default)]
pub struct LogConfigBuilder {
    config: LogConfig,
}

impl LogConfigBuilder {
    pub fn level(mut self, level: LogLevel) -> Self {
        self.config.level = level;
        self
    }

    pub fn output(mut self, output: LogOutput) -> Self {
        self.config.output = output;
        self
    }

    pub fn format(mut self, format: LogFormat) -> Self {
        self.config.format = format;
        self
    }

    pub fn log_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.config.log_dir = dir.into();
        self
    }

    pub fn log_file_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.config.log_file_prefix = prefix.into();
        self
    }

    pub fn filter_directives(mut self, filter: impl Into<String>) -> Self {
        self.config.filter_directives = Some(filter.into());
        self
    }

    pub fn include_location(mut self, include: bool) -> Self {
        self.config.include_location = include;
        self
    }

    pub fn include_thread_ids(mut self, include: bool) -> Self {
        self.config.include_thread_ids = include;
        self
    }

    pub fn include_targets(mut self, include: bool) -> Self {
        self.config.include_targets = include;
        self
    }

    pub fn build(self) -> LogConfig {
        self.config
    }
}

/// Initialize logging with the given configuration
///
/// This sets up the global tracing subscriber. It should only be called once
/// at application startup.
///
/// # Example
///
/// ```no_run
/// use bdp_common::logging::{LogConfig, init_logging};
///
/// let config = LogConfig::from_env().unwrap();
/// init_logging(&config).unwrap();
/// ```
pub fn init_logging(config: &LogConfig) -> Result<()> {
    // Build the base filter
    let mut filter =
        EnvFilter::from_default_env().add_directive(config.level.to_tracing_level().into());

    // Add custom filter directives if provided
    if let Some(ref directives) = config.filter_directives {
        for directive in directives.split(',') {
            filter = filter.add_directive(
                directive
                    .parse()
                    .context("Failed to parse filter directive")?,
            );
        }
    }

    match config.output {
        LogOutput::Console => {
            // Console-only output
            init_console_logging(config, filter)?;
        },
        LogOutput::File => {
            // File-only output
            init_file_logging(config, filter)?;
        },
        LogOutput::Both => {
            // Both console and file output
            init_both_logging(config, filter)?;
        },
    }

    Ok(())
}

/// Initialize console-only logging
fn init_console_logging(config: &LogConfig, filter: EnvFilter) -> Result<()> {
    let fmt_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(config.include_targets)
        .with_thread_ids(config.include_thread_ids)
        .with_file(config.include_location)
        .with_line_number(config.include_location)
        .with_span_events(FmtSpan::CLOSE);

    match config.format {
        LogFormat::Text => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer)
                .try_init()?;
        },
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer.json())
                .try_init()?;
        },
    }

    Ok(())
}

/// Initialize file-only logging
fn init_file_logging(config: &LogConfig, filter: EnvFilter) -> Result<()> {
    // Ensure log directory exists
    std::fs::create_dir_all(&config.log_dir).context("Failed to create log directory")?;

    // Create daily rotating file appender
    let file_appender = tracing_appender::rolling::daily(&config.log_dir, &config.log_file_prefix);

    // Make it non-blocking for better performance
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Note: _guard must be kept alive for the duration of the program
    // We'll leak it to keep it alive for the application lifetime
    std::mem::forget(_guard);

    let fmt_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_target(config.include_targets)
        .with_thread_ids(config.include_thread_ids)
        .with_file(config.include_location)
        .with_line_number(config.include_location)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false); // No ANSI colors in files

    match config.format {
        LogFormat::Text => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer)
                .try_init()?;
        },
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer.json())
                .try_init()?;
        },
    }

    Ok(())
}

/// Initialize both console and file logging
fn init_both_logging(config: &LogConfig, filter: EnvFilter) -> Result<()> {
    // Ensure log directory exists
    std::fs::create_dir_all(&config.log_dir).context("Failed to create log directory")?;

    // Create daily rotating file appender
    let file_appender = tracing_appender::rolling::daily(&config.log_dir, &config.log_file_prefix);

    // Make it non-blocking for better performance
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Note: _guard must be kept alive for the duration of the program
    // We'll leak it to keep it alive for the application lifetime
    std::mem::forget(_guard);

    match config.format {
        LogFormat::Text => {
            // Console layer (text format)
            let console_layer = fmt::layer()
                .with_writer(std::io::stdout)
                .with_target(config.include_targets)
                .with_thread_ids(config.include_thread_ids)
                .with_file(config.include_location)
                .with_line_number(config.include_location)
                .with_span_events(FmtSpan::CLOSE);

            // File layer (text format, no ANSI colors)
            let file_layer = fmt::layer()
                .with_writer(non_blocking)
                .with_target(config.include_targets)
                .with_thread_ids(config.include_thread_ids)
                .with_file(config.include_location)
                .with_line_number(config.include_location)
                .with_span_events(FmtSpan::CLOSE)
                .with_ansi(false);

            tracing_subscriber::registry()
                .with(filter)
                .with(console_layer)
                .with(file_layer)
                .try_init()?;
        },
        LogFormat::Json => {
            // Console layer (JSON format)
            let console_layer = fmt::layer()
                .json()
                .with_writer(std::io::stdout)
                .with_target(config.include_targets)
                .with_thread_ids(config.include_thread_ids)
                .with_file(config.include_location)
                .with_line_number(config.include_location)
                .with_span_events(FmtSpan::CLOSE);

            // File layer (JSON format, no ANSI colors)
            let file_layer = fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_target(config.include_targets)
                .with_thread_ids(config.include_thread_ids)
                .with_file(config.include_location)
                .with_line_number(config.include_location)
                .with_span_events(FmtSpan::CLOSE)
                .with_ansi(false);

            tracing_subscriber::registry()
                .with(filter)
                .with(console_layer)
                .with(file_layer)
                .try_init()?;
        },
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_str() {
        assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);
        assert_eq!("DEBUG".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("Info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("ERROR".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert!("invalid".parse::<LogLevel>().is_err());
    }

    #[test]
    fn test_log_output_from_str() {
        assert_eq!("console".parse::<LogOutput>().unwrap(), LogOutput::Console);
        assert_eq!("file".parse::<LogOutput>().unwrap(), LogOutput::File);
        assert_eq!("both".parse::<LogOutput>().unwrap(), LogOutput::Both);
        assert!("invalid".parse::<LogOutput>().is_err());
    }

    #[test]
    fn test_log_format_from_str() {
        assert_eq!("text".parse::<LogFormat>().unwrap(), LogFormat::Text);
        assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert!("invalid".parse::<LogFormat>().is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = LogConfig::builder()
            .level(LogLevel::Debug)
            .output(LogOutput::File)
            .format(LogFormat::Json)
            .log_dir("/var/log/bdp")
            .log_file_prefix("test")
            .build();

        assert_eq!(config.level, LogLevel::Debug);
        assert_eq!(config.output, LogOutput::File);
        assert_eq!(config.format, LogFormat::Json);
        assert_eq!(config.log_dir, PathBuf::from("/var/log/bdp"));
        assert_eq!(config.log_file_prefix, "test");
    }
}
