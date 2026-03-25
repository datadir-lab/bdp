mod common;

#[cfg(test)]
mod tests {
    use bdp_ingest::framework::PipelineRunner;
    use bdp_ingest::pipelines::open_targets::{
        config::OpenTargetsConfig, runner::OpenTargetsPipelineRunner,
    };
    use uuid::Uuid;

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn test_open_targets_schema_exists() {
        let pg = crate::common::TestPostgres::start()
            .await
            .expect("postgres container");
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'gene_disease_associations'"
        )
        .fetch_one(&pg.pool)
        .await
        .expect("query");
        assert_eq!(count, 1, "gene_disease_associations table should exist");
    }

    #[tokio::test]
    #[ignore = "requires Docker + internet (downloads ~2GB)"]
    async fn test_open_targets_pipeline_runs() {
        let pg = crate::common::TestPostgres::start()
            .await
            .expect("postgres container");
        let org_id = Uuid::new_v4();
        let mut config = OpenTargetsConfig::new("25.03", org_id);
        config.parse_limit = Some(1);
        let runner = OpenTargetsPipelineRunner::new(config, pg.pool.clone());
        let stats = runner.run().await.expect("pipeline should not error");
        assert_eq!(stats.pipeline_name, "open_targets");
    }
}
