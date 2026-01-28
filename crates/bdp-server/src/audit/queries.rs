//! Database queries for audit logs

use sqlx::PgPool;
use tracing::debug;

use super::models::{
    AuditEntry, AuditQuery, CreateAuditEntry, ResourceType,
    DEFAULT_AUDIT_QUERY_LIMIT, MAX_AUDIT_QUERY_LIMIT,
};
use crate::error::ServerResult;

/// Create a new audit log entry
///
/// This function inserts a new audit record into the audit_log table.
/// It returns the complete audit entry with generated ID and timestamp.
pub async fn create_audit_entry(
    pool: &PgPool,
    entry: CreateAuditEntry,
) -> ServerResult<AuditEntry> {
    let record = sqlx::query_as::<_, AuditEntry>(
        r#"
        INSERT INTO audit_log (
            user_id, action, resource_type, resource_id,
            changes, ip_address, user_agent, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, user_id, action, resource_type, resource_id,
                  changes, ip_address, user_agent, timestamp, metadata
        "#,
    )
    .bind(entry.user_id)
    .bind(entry.action.as_str())
    .bind(entry.resource_type.as_str())
    .bind(entry.resource_id)
    .bind(&entry.changes)
    .bind(&entry.ip_address)
    .bind(&entry.user_agent)
    .bind(&entry.metadata)
    .fetch_one(pool)
    .await?;

    debug!(
        audit_id = %record.id,
        action = %entry.action,
        resource_type = %entry.resource_type,
        "Created audit log entry"
    );

    Ok(record)
}

/// Query audit logs with filters
///
/// This function builds a dynamic query based on the provided filters
/// and returns matching audit log entries.
pub async fn query_audit_logs(pool: &PgPool, query: AuditQuery) -> ServerResult<Vec<AuditEntry>> {
    let limit = query.limit.min(MAX_AUDIT_QUERY_LIMIT);

    let mut sql = String::from(
        r#"
        SELECT
            id, user_id, action, resource_type, resource_id,
            changes, ip_address, user_agent, timestamp, metadata
        FROM audit_log
        WHERE 1=1
        "#,
    );

    let mut bind_count = 1;
    let mut conditions = Vec::new();

    // Build dynamic query based on filters
    if query.user_id.is_some() {
        conditions.push(format!("user_id = ${}", bind_count));
        bind_count += 1;
    }
    if query.action.is_some() {
        conditions.push(format!("action = ${}", bind_count));
        bind_count += 1;
    }
    if query.resource_type.is_some() {
        conditions.push(format!("resource_type = ${}", bind_count));
        bind_count += 1;
    }
    if query.resource_id.is_some() {
        conditions.push(format!("resource_id = ${}", bind_count));
        bind_count += 1;
    }
    if query.start_time.is_some() {
        conditions.push(format!("timestamp >= ${}", bind_count));
        bind_count += 1;
    }
    if query.end_time.is_some() {
        conditions.push(format!("timestamp <= ${}", bind_count));
        bind_count += 1;
    }

    for condition in conditions {
        sql.push_str(" AND ");
        sql.push_str(&condition);
    }

    sql.push_str(" ORDER BY timestamp DESC");
    sql.push_str(&format!(" LIMIT ${}", bind_count));
    bind_count += 1;
    sql.push_str(&format!(" OFFSET ${}", bind_count));

    let mut query_builder = sqlx::query_as::<_, AuditEntry>(&sql);

    // Bind parameters in order
    if let Some(user_id) = query.user_id {
        query_builder = query_builder.bind(user_id);
    }
    if let Some(action) = query.action {
        query_builder = query_builder.bind(action.as_str());
    }
    if let Some(resource_type) = query.resource_type {
        query_builder = query_builder.bind(resource_type.as_str());
    }
    if let Some(resource_id) = query.resource_id {
        query_builder = query_builder.bind(resource_id);
    }
    if let Some(start_time) = query.start_time {
        query_builder = query_builder.bind(start_time);
    }
    if let Some(end_time) = query.end_time {
        query_builder = query_builder.bind(end_time);
    }

    query_builder = query_builder.bind(limit).bind(query.offset);

    let records = query_builder.fetch_all(pool).await?;

    debug!(count = records.len(), "Queried audit logs");

    Ok(records)
}

/// Get audit trail for a specific resource
///
/// Returns all audit log entries for a given resource, ordered by timestamp (newest first).
pub async fn get_audit_trail(
    pool: &PgPool,
    resource_type: ResourceType,
    resource_id: uuid::Uuid,
    limit: Option<i64>,
) -> ServerResult<Vec<AuditEntry>> {
    let limit = limit.unwrap_or(DEFAULT_AUDIT_QUERY_LIMIT).min(MAX_AUDIT_QUERY_LIMIT);

    let records = sqlx::query_as::<_, AuditEntry>(
        r#"
        SELECT id, user_id, action, resource_type, resource_id,
               changes, ip_address, user_agent, timestamp, metadata
        FROM audit_log
        WHERE resource_type = $1 AND resource_id = $2
        ORDER BY timestamp DESC
        LIMIT $3
        "#,
    )
    .bind(resource_type.as_str())
    .bind(resource_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    debug!(
        resource_type = %resource_type,
        resource_id = %resource_id,
        count = records.len(),
        "Retrieved audit trail"
    );

    Ok(records)
}

/// Get recent audit logs for a specific user
///
/// Returns the most recent audit log entries for a given user.
pub async fn get_user_audit_logs(
    pool: &PgPool,
    user_id: uuid::Uuid,
    limit: Option<i64>,
) -> ServerResult<Vec<AuditEntry>> {
    let limit = limit.unwrap_or(DEFAULT_AUDIT_QUERY_LIMIT).min(MAX_AUDIT_QUERY_LIMIT);

    let records = sqlx::query_as::<_, AuditEntry>(
        r#"
        SELECT id, user_id, action, resource_type, resource_id,
               changes, ip_address, user_agent, timestamp, metadata
        FROM audit_log
        WHERE user_id = $1
        ORDER BY timestamp DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    debug!(
        user_id = %user_id,
        count = records.len(),
        "Retrieved user audit logs"
    );

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[sqlx::test]
    async fn test_create_audit_entry(pool: PgPool) -> ServerResult<()> {
        let entry = CreateAuditEntry {
            user_id: Some(Uuid::new_v4()),
            action: AuditAction::Create,
            resource_type: ResourceType::Organization,
            resource_id: Some(Uuid::new_v4()),
            changes: Some(json!({"name": "Test Org"})),
            metadata: None,
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("Test Agent".to_string()),
        };

        let result = create_audit_entry(&pool, entry).await?;

        assert_eq!(result.action, "create");
        assert_eq!(result.resource_type, "organization");

        Ok(())
    }

    #[sqlx::test]
    async fn test_query_audit_logs(pool: PgPool) -> ServerResult<()> {
        // Create test entries
        for i in 0..5 {
            let entry = CreateAuditEntry {
                user_id: Some(Uuid::new_v4()),
                action: AuditAction::Create,
                resource_type: ResourceType::DataSource,
                resource_id: Some(Uuid::new_v4()),
                changes: Some(json!({"index": i})),
                metadata: None,
                ip_address: None,
                user_agent: None,
            };
            create_audit_entry(&pool, entry).await?;
        }

        // Query all entries
        let query = AuditQuery::default();
        let results = query_audit_logs(&pool, query).await?;

        assert!(results.len() >= 5);

        // Query filtered by resource type
        let query = AuditQuery {
            resource_type: Some(ResourceType::DataSource),
            ..Default::default()
        };
        let results = query_audit_logs(&pool, query).await?;

        assert!(results.len() >= 5);
        assert!(results.iter().all(|r| r.resource_type == "data_source"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_audit_trail(pool: PgPool) -> ServerResult<()> {
        let resource_id = Uuid::new_v4();

        // Create test entries for the same resource
        for _ in 0..3 {
            let entry = CreateAuditEntry {
                user_id: Some(Uuid::new_v4()),
                action: AuditAction::Update,
                resource_type: ResourceType::Version,
                resource_id: Some(resource_id),
                changes: Some(json!({"updated": true})),
                metadata: None,
                ip_address: None,
                user_agent: None,
            };
            create_audit_entry(&pool, entry).await?;
        }

        let trail = get_audit_trail(&pool, ResourceType::Version, resource_id, None).await?;

        assert_eq!(trail.len(), 3);
        assert!(trail.iter().all(|r| r.resource_id == Some(resource_id)));

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_user_audit_logs(pool: PgPool) -> ServerResult<()> {
        let user_id = Uuid::new_v4();

        // Create test entries for the same user
        for _ in 0..4 {
            let entry = CreateAuditEntry {
                user_id: Some(user_id),
                action: AuditAction::Read,
                resource_type: ResourceType::Organization,
                resource_id: Some(Uuid::new_v4()),
                changes: None,
                metadata: None,
                ip_address: None,
                user_agent: None,
            };
            create_audit_entry(&pool, entry).await?;
        }

        let logs = get_user_audit_logs(&pool, user_id, None).await?;

        assert_eq!(logs.len(), 4);
        assert!(logs.iter().all(|r| r.user_id == Some(user_id)));

        Ok(())
    }
}
