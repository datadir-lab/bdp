use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteDataSourceCommand {
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteDataSourceResponse {
    pub id: Uuid,
    pub deleted: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum DeleteDataSourceError {
    #[error("Data source with ID '{0}' not found")]
    NotFound(Uuid),
    #[error("Cannot delete data source '{0}': it has associated versions")]
    HasVersions(Uuid),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<DeleteDataSourceResponse, DeleteDataSourceError>> for DeleteDataSourceCommand {}

impl crate::cqrs::middleware::Command for DeleteDataSourceCommand {}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    command: DeleteDataSourceCommand,
) -> Result<DeleteDataSourceResponse, DeleteDataSourceError> {
    let result = sqlx::query!(
        r#"
        DELETE FROM registry_entries
        WHERE id = $1 AND entry_type = 'data_source'
        RETURNING id
        "#,
        command.id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.is_foreign_key_violation() {
                return DeleteDataSourceError::HasVersions(command.id);
            }
        }
        DeleteDataSourceError::Database(e)
    })?;

    match result {
        Some(_) => Ok(DeleteDataSourceResponse {
            id: command.id,
            deleted: true,
        }),
        None => Err(DeleteDataSourceError::NotFound(command.id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_handle_deletes_data_source(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "test-protein",
            "Test Protein"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id,
            "protein"
        )
        .execute(&pool)
        .await?;

        let cmd = DeleteDataSourceCommand { id: entry_id };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.id, entry_id);
        assert!(response.deleted);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_not_found(pool: PgPool) -> sqlx::Result<()> {
        let cmd = DeleteDataSourceCommand { id: Uuid::new_v4() };

        let result = handle(pool.clone(), cmd).await;
        assert!(matches!(result, Err(DeleteDataSourceError::NotFound(_))));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_has_versions(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "test-protein",
            "Test Protein"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id,
            "protein"
        )
        .execute(&pool)
        .await?;

        sqlx::query!("INSERT INTO versions (entry_id, version) VALUES ($1, $2)", entry_id, "1.0")
            .execute(&pool)
            .await?;

        let cmd = DeleteDataSourceCommand { id: entry_id };

        let result = handle(pool.clone(), cmd).await;
        assert!(matches!(result, Err(DeleteDataSourceError::HasVersions(_))));
        Ok(())
    }
}
