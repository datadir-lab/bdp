// crates/bdp-server/src/features/vectors/queries/get_neighbors.rs
use mediator::Request;
use pgvector::HalfVector;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::semantic_search::SemanticSearchItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNeighborsQuery {
    pub entry_id: Uuid,
    #[serde(default = "default_k")]
    pub k: i64,
}

fn default_k() -> i64 { 10 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNeighborsResponse {
    pub neighbors: Vec<SemanticSearchItem>,
}

#[derive(Debug, thiserror::Error)]
pub enum GetNeighborsError {
    #[error("Entry not found or has no embedding")]
    NotFound,
    #[error("k must be between 1 and 100")]
    InvalidK,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<GetNeighborsResponse, GetNeighborsError>> for GetNeighborsQuery {}
impl crate::cqrs::middleware::Query for GetNeighborsQuery {}

impl GetNeighborsQuery {
    pub fn validate(&self) -> Result<(), GetNeighborsError> {
        if !(1..=100).contains(&self.k) {
            return Err(GetNeighborsError::InvalidK);
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: GetNeighborsQuery,
) -> Result<GetNeighborsResponse, GetNeighborsError> {
    query.validate()?;

    // Fetch seed vector
    let seed = sqlx::query_scalar!(
        r#"SELECT vector AS "vector!: HalfVector" FROM entry_embeddings WHERE entry_id = $1"#,
        query.entry_id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(GetNeighborsError::NotFound)?;

    // KNN excluding self
    let rows = sqlx::query!(
        r#"
        SELECT
            e.entry_id               AS "entry_id!: Uuid",
            re.slug                  AS "slug!",
            re.name                  AS "name!",
            re.entry_type            AS "entry_type!",
            ds.source_type           AS "source_type?",
            o.slug                   AS "org_slug!",
            ep.x                     AS "x?: f32",
            ep.y                     AS "y?: f32",
            (1.0 - (e.vector <=> $1::halfvec))::float4 AS "similarity!"
        FROM entry_embeddings e
        JOIN registry_entries re ON re.id = e.entry_id
        JOIN organizations o ON o.id = re.organization_id
        LEFT JOIN data_sources ds ON ds.id = re.id
        LEFT JOIN entry_projections ep ON ep.entry_id = e.entry_id
        WHERE e.entry_id != $2
        ORDER BY e.vector <=> $1::halfvec
        LIMIT $3
        "#,
        seed as HalfVector,
        query.entry_id,
        query.k,
    )
    .fetch_all(&pool)
    .await?;

    Ok(GetNeighborsResponse {
        neighbors: rows.into_iter().map(|r| SemanticSearchItem {
            entry_id: r.entry_id,
            slug: r.slug,
            name: r.name,
            entry_type: r.entry_type,
            source_type: r.source_type,
            org_slug: r.org_slug,
            x: r.x,
            y: r.y,
            similarity: r.similarity,
        }).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_k() {
        let q = GetNeighborsQuery { entry_id: Uuid::new_v4(), k: 0 };
        assert!(matches!(q.validate(), Err(GetNeighborsError::InvalidK)));
    }
}
