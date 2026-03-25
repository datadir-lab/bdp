// crates/bdp-ingest/src/framework/pipeline.rs

use std::future::Future;

/// Statistics returned by a completed pipeline run.
#[derive(Debug, Default, Clone)]
pub struct PipelineStats {
    pub pipeline_name: &'static str,
    pub records_ingested: u64,
    pub records_skipped: u64,
    pub records_failed: u64,
    pub duration_secs: u64,
}

impl PipelineStats {
    pub fn new(name: &'static str) -> Self {
        Self {
            pipeline_name: name,
            ..Default::default()
        }
    }
}

impl std::fmt::Display for PipelineStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: ingested={} skipped={} failed={} duration={}s",
            self.pipeline_name,
            self.records_ingested,
            self.records_skipped,
            self.records_failed,
            self.duration_secs,
        )
    }
}

/// Implemented by every ingest pipeline.
///
/// # Requirements
/// - `Send + 'static` so the pipeline can be spawned via `JoinSet::spawn`
/// - `run()` consumes `self` — pipelines are single-use
pub trait PipelineRunner: Send + 'static {
    fn name(&self) -> &'static str;

    fn run(self) -> impl Future<Output = anyhow::Result<PipelineStats>> + Send;
}
