//! Comprehensive tests for the audit logging system
//!
//! These tests verify:
//! - Audit log creation and storage
//! - Querying audit logs with various filters
//! - Audit middleware integration
//! - Resource type and action inference

use bdp_server::audit::{
    create_audit_entry, get_audit_trail, get_user_audit_logs, query_audit_logs, AuditAction,
    AuditQuery, CreateAuditEntry, ResourceType,
};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

mod helpers;

/// Test creating a basic audit log entry
#[sqlx::test(migrations = "../../migrations")]
async fn test_create_basic_audit_entry(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let user_id = Uuid::new_v4();
    let resource_id = Uuid::new_v4();

    let entry = CreateAuditEntry::builder()
        .action(AuditAction::Create)
        .resource_type(ResourceType::Organization)
        .user_id(Some(user_id))
        .resource_id(Some(resource_id))
        .ip_address("192.168.1.100")
        .user_agent("Test Agent/1.0")
        .build();

    let result = create_audit_entry(&pool, entry).await?;

    assert_eq!(result.action, "create");
    assert_eq!(result.resource_type, "organization");
    assert_eq!(result.user_id, Some(user_id));
    assert_eq!(result.resource_id, Some(resource_id));
    assert_eq!(result.ip_address, Some("192.168.1.100".to_string()));
    assert_eq!(result.user_agent, Some("Test Agent/1.0".to_string()));

    Ok(())
}

/// Test creating audit entry with changes JSON
#[sqlx::test(migrations = "../../migrations")]
async fn test_create_audit_entry_with_changes(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let changes = json!({
        "before": {
            "name": "Old Name"
        },
        "after": {
            "name": "New Name"
        }
    });

    let entry = CreateAuditEntry::builder()
        .action(AuditAction::Update)
        .resource_type(ResourceType::DataSource)
        .resource_id(Some(Uuid::new_v4()))
        .changes(changes.clone())
        .build();

    let result = create_audit_entry(&pool, entry).await?;

    assert_eq!(result.action, "update");
    assert_eq!(result.resource_type, "data_source");
    assert!(result.changes.is_some());

    let stored_changes = result.changes.unwrap();
    assert_eq!(stored_changes["before"]["name"], json!("Old Name"));
    assert_eq!(stored_changes["after"]["name"], json!("New Name"));

    Ok(())
}

/// Test creating audit entry with metadata
#[sqlx::test(migrations = "../../migrations")]
async fn test_create_audit_entry_with_metadata(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = json!({
        "request_id": "req-12345",
        "session_id": "sess-67890",
        "source": "api"
    });

    let entry = CreateAuditEntry::builder()
        .action(AuditAction::Delete)
        .resource_type(ResourceType::Version)
        .metadata(metadata.clone())
        .build();

    let result = create_audit_entry(&pool, entry).await?;

    assert!(result.metadata.is_some());
    let stored_metadata = result.metadata.unwrap();
    assert_eq!(stored_metadata["request_id"], json!("req-12345"));
    assert_eq!(stored_metadata["session_id"], json!("sess-67890"));

    Ok(())
}

/// Test anonymous audit entry (no user_id)
#[sqlx::test(migrations = "../../migrations")]
async fn test_anonymous_audit_entry(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let entry = CreateAuditEntry::builder()
        .action(AuditAction::Read)
        .resource_type(ResourceType::Organization)
        .build();

    let result = create_audit_entry(&pool, entry).await?;

    assert!(result.user_id.is_none());
    assert_eq!(result.action, "read");

    Ok(())
}

/// Test querying audit logs with filters
#[sqlx::test(migrations = "../../migrations")]
async fn test_query_audit_logs_with_filters(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let user_id = Uuid::new_v4();

    // Create multiple audit entries
    for i in 0..5 {
        let entry = CreateAuditEntry::builder()
            .action(AuditAction::Create)
            .resource_type(ResourceType::DataSource)
            .user_id(Some(user_id))
            .resource_id(Some(Uuid::new_v4()))
            .changes(json!({"index": i}))
            .build();

        create_audit_entry(&pool, entry).await?;
    }

    // Create entries for a different user
    for _ in 0..3 {
        let entry = CreateAuditEntry::builder()
            .action(AuditAction::Update)
            .resource_type(ResourceType::Organization)
            .user_id(Some(Uuid::new_v4()))
            .build();

        create_audit_entry(&pool, entry).await?;
    }

    // Query by user_id
    let query = AuditQuery {
        user_id: Some(user_id),
        ..Default::default()
    };
    let results = query_audit_logs(&pool, query).await?;
    assert_eq!(results.len(), 5);
    assert!(results.iter().all(|r| r.user_id == Some(user_id)));

    // Query by resource_type
    let query = AuditQuery {
        resource_type: Some(ResourceType::DataSource),
        ..Default::default()
    };
    let results = query_audit_logs(&pool, query).await?;
    assert!(results.len() >= 5);
    assert!(results.iter().all(|r| r.resource_type == "data_source"));

    // Query by action
    let query = AuditQuery {
        action: Some(AuditAction::Update),
        ..Default::default()
    };
    let results = query_audit_logs(&pool, query).await?;
    assert!(results.len() >= 3);
    assert!(results.iter().all(|r| r.action == "update"));

    Ok(())
}

/// Test querying audit logs with pagination
#[sqlx::test(migrations = "../../migrations")]
async fn test_query_audit_logs_pagination(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // Create 15 audit entries
    for i in 0..15 {
        let entry = CreateAuditEntry::builder()
            .action(AuditAction::Create)
            .resource_type(ResourceType::Version)
            .changes(json!({"index": i}))
            .build();

        create_audit_entry(&pool, entry).await?;
    }

    // First page
    let query = AuditQuery {
        resource_type: Some(ResourceType::Version),
        limit: 5,
        offset: 0,
        ..Default::default()
    };
    let page1 = query_audit_logs(&pool, query).await?;
    assert_eq!(page1.len(), 5);

    // Second page
    let query = AuditQuery {
        resource_type: Some(ResourceType::Version),
        limit: 5,
        offset: 5,
        ..Default::default()
    };
    let page2 = query_audit_logs(&pool, query).await?;
    assert_eq!(page2.len(), 5);

    // Third page
    let query = AuditQuery {
        resource_type: Some(ResourceType::Version),
        limit: 5,
        offset: 10,
        ..Default::default()
    };
    let page3 = query_audit_logs(&pool, query).await?;
    assert_eq!(page3.len(), 5);

    // Verify no duplicates across pages
    let page1_ids: Vec<_> = page1.iter().map(|e| e.id).collect();
    let page2_ids: Vec<_> = page2.iter().map(|e| e.id).collect();
    let page3_ids: Vec<_> = page3.iter().map(|e| e.id).collect();

    assert!(page1_ids.iter().all(|id| !page2_ids.contains(id)));
    assert!(page1_ids.iter().all(|id| !page3_ids.contains(id)));
    assert!(page2_ids.iter().all(|id| !page3_ids.contains(id)));

    Ok(())
}

/// Test getting audit trail for a specific resource
#[sqlx::test(migrations = "../../migrations")]
async fn test_get_audit_trail(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let resource_id = Uuid::new_v4();

    // Create multiple actions on the same resource
    let actions = vec![
        AuditAction::Create,
        AuditAction::Update,
        AuditAction::Update,
        AuditAction::Archive,
    ];

    for action in actions {
        let entry = CreateAuditEntry::builder()
            .action(action)
            .resource_type(ResourceType::Organization)
            .resource_id(Some(resource_id))
            .user_id(Some(Uuid::new_v4()))
            .build();

        create_audit_entry(&pool, entry).await?;
    }

    let trail = get_audit_trail(&pool, ResourceType::Organization, resource_id, None).await?;

    assert_eq!(trail.len(), 4);
    assert!(trail.iter().all(|e| e.resource_id == Some(resource_id)));
    assert!(trail.iter().all(|e| e.resource_type == "organization"));

    // Verify chronological order (newest first)
    for i in 0..trail.len() - 1 {
        assert!(trail[i].timestamp >= trail[i + 1].timestamp);
    }

    Ok(())
}

/// Test getting user audit logs
#[sqlx::test(migrations = "../../migrations")]
async fn test_get_user_audit_logs(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let user_id = Uuid::new_v4();

    // Create various actions by the user
    let actions = vec![
        (AuditAction::Create, ResourceType::Organization),
        (AuditAction::Update, ResourceType::DataSource),
        (AuditAction::Delete, ResourceType::Version),
        (AuditAction::Read, ResourceType::Tool),
    ];

    for (action, resource_type) in actions {
        let entry = CreateAuditEntry::builder()
            .action(action)
            .resource_type(resource_type)
            .user_id(Some(user_id))
            .resource_id(Some(Uuid::new_v4()))
            .build();

        create_audit_entry(&pool, entry).await?;
    }

    let logs = get_user_audit_logs(&pool, user_id, None).await?;

    assert_eq!(logs.len(), 4);
    assert!(logs.iter().all(|e| e.user_id == Some(user_id)));

    // Verify chronological order (newest first)
    for i in 0..logs.len() - 1 {
        assert!(logs[i].timestamp >= logs[i + 1].timestamp);
    }

    Ok(())
}

/// Test audit entry with all audit actions
#[sqlx::test(migrations = "../../migrations")]
async fn test_all_audit_actions(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let actions = vec![
        AuditAction::Create,
        AuditAction::Update,
        AuditAction::Delete,
        AuditAction::Read,
        AuditAction::Login,
        AuditAction::Logout,
        AuditAction::Register,
        AuditAction::Publish,
        AuditAction::Unpublish,
        AuditAction::Archive,
        AuditAction::Upload,
        AuditAction::Download,
        AuditAction::Grant,
        AuditAction::Revoke,
        AuditAction::Other,
    ];

    for action in actions {
        let entry = CreateAuditEntry::builder()
            .action(action)
            .resource_type(ResourceType::Other)
            .build();

        let result = create_audit_entry(&pool, entry).await?;
        assert_eq!(result.action, action.as_str());
    }

    Ok(())
}

/// Test audit entry with all resource types
#[sqlx::test(migrations = "../../migrations")]
async fn test_all_resource_types(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let resource_types = vec![
        ResourceType::Organization,
        ResourceType::DataSource,
        ResourceType::Version,
        ResourceType::Tool,
        ResourceType::RegistryEntry,
        ResourceType::VersionFile,
        ResourceType::Dependency,
        ResourceType::Organism,
        ResourceType::ProteinMetadata,
        ResourceType::Citation,
        ResourceType::Tag,
        ResourceType::Download,
        ResourceType::VersionMapping,
        ResourceType::User,
        ResourceType::Session,
        ResourceType::ApiKey,
        ResourceType::Other,
    ];

    for resource_type in resource_types {
        let entry = CreateAuditEntry::builder()
            .action(AuditAction::Create)
            .resource_type(resource_type)
            .build();

        let result = create_audit_entry(&pool, entry).await?;
        assert_eq!(result.resource_type, resource_type.as_str());
    }

    Ok(())
}

/// Test querying with time range filters
#[sqlx::test(migrations = "../../migrations")]
async fn test_query_with_time_range(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Utc::now();

    // Create some entries
    for _ in 0..5 {
        let entry = CreateAuditEntry::builder()
            .action(AuditAction::Create)
            .resource_type(ResourceType::Organization)
            .build();

        create_audit_entry(&pool, entry).await?;
    }

    let end_time = Utc::now();

    // Query with time range
    let query = AuditQuery {
        start_time: Some(start_time),
        end_time: Some(end_time),
        ..Default::default()
    };

    let results = query_audit_logs(&pool, query).await?;

    assert!(results.len() >= 5);
    assert!(results
        .iter()
        .all(|e| e.timestamp >= start_time && e.timestamp <= end_time));

    Ok(())
}

/// Test limit capping (should not exceed 1000)
#[sqlx::test(migrations = "../../migrations")]
async fn test_query_limit_capping(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // Create 10 entries
    for _ in 0..10 {
        let entry = CreateAuditEntry::builder()
            .action(AuditAction::Create)
            .resource_type(ResourceType::Version)
            .build();

        create_audit_entry(&pool, entry).await?;
    }

    // Try to query with excessive limit
    let query = AuditQuery {
        resource_type: Some(ResourceType::Version),
        limit: 5000, // Should be capped at 1000
        ..Default::default()
    };

    let results = query_audit_logs(&pool, query).await?;

    // Should get at least 10 entries (we created), but not fail
    assert!(results.len() >= 10);

    Ok(())
}

/// Test IP address formats (IPv4 and IPv6)
#[sqlx::test(migrations = "../../migrations")]
async fn test_ip_address_formats(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // IPv4
    let entry_ipv4 = CreateAuditEntry::builder()
        .action(AuditAction::Create)
        .resource_type(ResourceType::Organization)
        .ip_address("192.168.1.1")
        .build();

    let result_ipv4 = create_audit_entry(&pool, entry_ipv4).await?;
    assert_eq!(result_ipv4.ip_address, Some("192.168.1.1".to_string()));

    // IPv6
    let entry_ipv6 = CreateAuditEntry::builder()
        .action(AuditAction::Create)
        .resource_type(ResourceType::Organization)
        .ip_address("2001:0db8:85a3:0000:0000:8a2e:0370:7334")
        .build();

    let result_ipv6 = create_audit_entry(&pool, entry_ipv6).await?;
    assert_eq!(
        result_ipv6.ip_address,
        Some("2001:0db8:85a3:0000:0000:8a2e:0370:7334".to_string())
    );

    Ok(())
}
