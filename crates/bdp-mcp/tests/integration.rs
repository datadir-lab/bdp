mod common;

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_mcp_get_disease_roundtrip() {
    let pg = common::TestPostgres::start().await.expect("start postgres");
    let org_id = common::create_test_org(&pg.pool, "roundtrip-test")
        .await
        .expect("org");
    common::seed_disease(&pg.pool, org_id).await.expect("seed");

    let params = bdp_mcp::tools::diseases::GetDiseaseParams {
        id: "MONDO:0004975".into(),
    };
    let result = bdp_mcp::tools::diseases::get_disease(&pg.pool, params)
        .await
        .expect("tool call ok");

    assert!(!result.is_error.unwrap_or(false));
    let structured = result.structured_content.expect("has structured content");
    assert_eq!(structured["mondo_id"], "MONDO:0004975");
    assert_eq!(structured["name"], "Alzheimer disease");
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_stub_returns_not_yet_available() {
    let result = bdp_mcp::tools::common::stub_result("search_literature", "needs PubMed", "BDP-84");
    let s = result.structured_content.expect("has structured content");
    assert_eq!(s["status"], "not_yet_available");
    assert_eq!(s["tracking"], "BDP-84");
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_audit_log_written_on_tool_call() {
    let pg = common::TestPostgres::start().await.expect("start postgres");
    let org_id = common::create_test_org(&pg.pool, "audit-test")
        .await
        .expect("org");
    common::seed_disease(&pg.pool, org_id).await.expect("seed");

    let count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_query_log")
        .fetch_one(&pg.pool)
        .await
        .expect("count");

    let params = bdp_mcp::tools::diseases::GetDiseaseParams {
        id: "MONDO:0004975".into(),
    };
    bdp_mcp::tools::diseases::get_disease(&pg.pool, params)
        .await
        .expect("tool call ok");

    let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_query_log")
        .fetch_one(&pg.pool)
        .await
        .expect("count");

    assert_eq!(count_after, count_before + 1, "audit log should have 1 new row after tool call");
}

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
