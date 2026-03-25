mod common;

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_get_disease_by_mondo_id() {
    let pg = common::TestPostgres::start().await.expect("start postgres");
    let org_id = common::create_test_org(&pg.pool, "test")
        .await
        .expect("create org");

    common::seed_disease(&pg.pool, org_id)
        .await
        .expect("seed disease");

    let result = bdp_mcp::db::queries::get_disease(&pg.pool, "MONDO:0004975")
        .await
        .expect("query ok");

    assert!(result.is_some());
    let d = result.unwrap();
    assert_eq!(d.mondo_id, "MONDO:0004975");
    assert_eq!(d.name, "Alzheimer disease");
    assert_eq!(d.omim_id.as_deref(), Some("104300"));
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_resolve_disease_by_mondo_id() {
    let pg = common::TestPostgres::start().await.expect("start postgres");
    let org_id = common::create_test_org(&pg.pool, "test2")
        .await
        .expect("create org");
    common::seed_disease(&pg.pool, org_id).await.expect("seed");

    let id = bdp_mcp::db::resolve::disease_by_mondo_id(&pg.pool, "MONDO:0004975")
        .await
        .expect("resolve ok");
    assert!(id.is_some());
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_audit_log_writes() {
    use bdp_mcp::db::audit::{log_tool_call, AuditEntry};

    let pg = common::TestPostgres::start().await.expect("start postgres");

    log_tool_call(
        &pg.pool,
        AuditEntry {
            agent_id: Some("test-agent"),
            tool_name: "get_disease",
            query_params: serde_json::json!({"mondo_id": "MONDO:0004975"}),
            dataset_versions: serde_json::json!({}),
            result_count: Some(1),
            duration_ms: Some(42),
        },
    )
    .await;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_query_log")
        .fetch_one(&pg.pool)
        .await
        .expect("count");
    assert_eq!(count, 1);
}
