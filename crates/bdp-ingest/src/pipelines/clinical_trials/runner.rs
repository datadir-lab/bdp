use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

use crate::framework::{PipelineRunner, PipelineStats};
use crate::pipelines::clinical_trials::{
    aact_loader::parse_studies_csv,
    api_fetcher::fetch_updated_studies,
    config::{ClinicalTrialsConfig, CT_API_BASE},
    storage::ClinicalTrialsStorage,
};

pub struct ClinicalTrialsPipelineRunner {
    pub config: ClinicalTrialsConfig,
    pub pool: PgPool,
}

impl ClinicalTrialsPipelineRunner {
    pub fn new(config: ClinicalTrialsConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }
}

impl PipelineRunner for ClinicalTrialsPipelineRunner {
    fn name(&self) -> &'static str {
        "clinical_trials"
    }

    async fn run(self) -> Result<PipelineStats> {
        let mut stats = PipelineStats::new(self.name());
        let storage = ClinicalTrialsStorage::new(self.pool.clone());

        if let Some(dump_path) = &self.config.aact_dump_path {
            info!("loading AACT dump from {:?}", dump_path);
            let content = tokio::fs::read_to_string(dump_path).await?;
            let rows = parse_studies_csv(&content)?;
            info!(rows = rows.len(), "parsed AACT studies");
            let inserted = storage.upsert_studies(&rows).await?;
            stats.records_ingested = inserted as u64;
        } else if let Some(from_date) = self.config.from_date {
            let client = reqwest::Client::new();
            info!("fetching CT.gov delta since {}", from_date);
            let raw = fetch_updated_studies(
                &client,
                CT_API_BASE,
                from_date,
                self.config.api_page_size,
                self.config.max_retries,
            )
            .await?;
            info!(count = raw.len(), "fetched CT.gov studies via API");
            let rows: Vec<_> = raw
                .iter()
                .filter_map(|v| {
                    let nct_id = v
                        .pointer("/protocolSection/identificationModule/nctId")?
                        .as_str()?
                        .to_string();
                    Some(
                        crate::pipelines::clinical_trials::aact_loader::AactStudyRow {
                            nct_id,
                            brief_title: v
                                .pointer(
                                    "/protocolSection/identificationModule/briefTitle",
                                )
                                .and_then(|t| t.as_str())
                                .map(String::from),
                            overall_status: v
                                .pointer("/protocolSection/statusModule/overallStatus")
                                .and_then(|t| t.as_str())
                                .map(String::from),
                            phase: v
                                .pointer("/protocolSection/designModule/phases/0")
                                .and_then(|t| t.as_str())
                                .map(String::from),
                            start_date: None,
                            completion_date: None,
                            source: None,
                            conditions: Vec::new(),
                            interventions: Vec::new(),
                        },
                    )
                })
                .collect();
            let inserted = storage.upsert_studies(&rows).await?;
            stats.records_ingested = inserted as u64;
        } else {
            anyhow::bail!("ClinicalTrialsConfig: set aact_dump_path or from_date");
        }

        Ok(stats)
    }
}
