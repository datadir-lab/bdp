//! [`PipelineRunner`] implementation for the Gene Ontology pipeline.
//!
//! Downloads `go-basic.obo` and parses it. Statistics (terms and relationships
//! parsed) are returned in [`PipelineStats`].
//!
//! Database persistence is out of scope for this crate; callers that need
//! persistence should use `bdp-server`'s `GoPipeline` or consume the
//! public [`ParsedGo`](crate::pipelines::gene_ontology::parser::ParsedGo)
//! struct directly.

use crate::common::http::download_text;
use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::gene_ontology::parser::parse_obo;
use crate::pipelines::gene_ontology::GO_BASIC_OBO_URL;
use tracing::info;

/// Default HTTP retry count for GO downloads.
const DEFAULT_RETRIES: u32 = 3;

/// Configuration for the GO pipeline runner.
#[derive(Debug, Clone)]
pub struct GoConfig {
    /// URL for the `go-basic.obo` file.
    pub obo_url: String,
    /// Maximum retry attempts for HTTP downloads.
    pub max_retries: u32,
    /// Optional cap on parsed terms (useful in tests).
    pub parse_limit: Option<usize>,
}

impl Default for GoConfig {
    fn default() -> Self {
        Self {
            obo_url: GO_BASIC_OBO_URL.to_string(),
            max_retries: DEFAULT_RETRIES,
            parse_limit: None,
        }
    }
}

/// Standalone Gene Ontology pipeline.
///
/// Implements [`PipelineRunner`] and can be handed to any executor that
/// works with that trait.
pub struct GoPipelineRunner {
    config: GoConfig,
}

impl GoPipelineRunner {
    pub fn new(config: GoConfig) -> Self {
        Self { config }
    }

    /// Convenience constructor using default configuration.
    pub fn with_defaults() -> Self {
        Self::new(GoConfig::default())
    }
}

impl PipelineRunner for GoPipelineRunner {
    fn name(&self) -> &'static str {
        "gene_ontology"
    }

    async fn run(self) -> anyhow::Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());

        info!(url = %self.config.obo_url, "downloading GO basic OBO");
        let obo_content = download_text(&self.config.obo_url, self.config.max_retries).await?;
        info!(bytes = obo_content.len(), "GO OBO download complete");

        let parsed = parse_obo(&obo_content, self.config.parse_limit)?;

        info!(
            terms = parsed.term_count(),
            relationships = parsed.relationship_count(),
            "GO OBO parse complete"
        );

        // Report terms as records_ingested; relationships as part of the total.
        stats.records_ingested = parsed.term_count() as u64;
        // Skipped = obsolete terms
        stats.records_skipped = parsed.terms.iter().filter(|t| t.is_obsolete).count() as u64;

        Ok(stats)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = GoConfig::default();
        assert_eq!(cfg.obo_url, GO_BASIC_OBO_URL);
        assert_eq!(cfg.max_retries, DEFAULT_RETRIES);
        assert!(cfg.parse_limit.is_none());
    }

    #[test]
    fn test_runner_name() {
        let runner = GoPipelineRunner::with_defaults();
        assert_eq!(runner.name(), "gene_ontology");
    }
}
