// crates/bdp-server/src/features/vectors/queries/semantic_search.rs
use mediator::Request;
use moka::future::Cache;
use once_cell::sync::Lazy;
use pgvector::HalfVector;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// In-process LRU cache: query string → halfvec(512)
// 128 entries × ~1KB each ≈ 128KB
static EMBED_CACHE: Lazy<Cache<String, Arc<Vec<f32>>>> = Lazy::new(|| {
    Cache::new(128)
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchQuery {
    pub q: String,
    #[serde(default = "default_k")]
    pub k: i64,
}

fn default_k() -> i64 { 20 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchItem {
    pub entry_id: Uuid,
    pub slug: String,
    pub name: String,
    pub entry_type: String,
    pub source_type: Option<String>,
    pub org_slug: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub similarity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResponse {
    pub items: Vec<SemanticSearchItem>,
}

#[derive(Debug, thiserror::Error)]
pub enum SemanticSearchError {
    #[error("Query is required")]
    QueryEmpty,
    #[error("k must be between 1 and 100")]
    InvalidK,
    #[error("Embedding service unavailable: {0}")]
    EmbeddingUnavailable(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<SemanticSearchResponse, SemanticSearchError>> for SemanticSearchQuery {}
impl crate::cqrs::middleware::Query for SemanticSearchQuery {}

impl SemanticSearchQuery {
    pub fn validate(&self) -> Result<(), SemanticSearchError> {
        if self.q.trim().is_empty() {
            return Err(SemanticSearchError::QueryEmpty);
        }
        if !(1..=100).contains(&self.k) {
            return Err(SemanticSearchError::InvalidK);
        }
        Ok(())
    }
}

/// Embed a query string via OpenAI, using the in-process cache.
async fn embed_query(q: &str) -> Result<HalfVector, SemanticSearchError> {
    let cache_key = q.to_lowercase();

    if let Some(cached) = EMBED_CACHE.get(&cache_key).await {
        let hv = HalfVector::from(cached.as_slice().iter().map(|&f| f as f32).collect::<Vec<_>>());
        return Ok(hv);
    }

    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let client = async_openai::Client::new().with_api_key(api_key);

    let request = async_openai::types::CreateEmbeddingRequestArgs::default()
        .model("text-embedding-3-small")
        .input(q)
        .dimensions(512u32)
        .build()
        .map_err(|e| SemanticSearchError::EmbeddingUnavailable(e.to_string()))?;

    let response = client
        .embeddings()
        .create(request)
        .await
        .map_err(|e| SemanticSearchError::EmbeddingUnavailable(e.to_string()))?;

    let floats: Vec<f32> = response.data[0].embedding.iter().map(|&f| f as f32).collect();
    EMBED_CACHE.insert(cache_key, Arc::new(floats.clone())).await;

    Ok(HalfVector::from(floats))
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: SemanticSearchQuery,
) -> Result<SemanticSearchResponse, SemanticSearchError> {
    query.validate()?;

    let vector = embed_query(&query.q).await?;

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
        ORDER BY e.vector <=> $1::halfvec
        LIMIT $2
        "#,
        vector as HalfVector,
        query.k,
    )
    .fetch_all(&pool)
    .await?;

    Ok(SemanticSearchResponse {
        items: rows.into_iter().map(|r| SemanticSearchItem {
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
    fn test_validate_empty_query() {
        let q = SemanticSearchQuery { q: "".to_string(), k: 20 };
        assert!(matches!(q.validate(), Err(SemanticSearchError::QueryEmpty)));
    }

    #[test]
    fn test_validate_invalid_k() {
        let q = SemanticSearchQuery { q: "insulin".to_string(), k: 0 };
        assert!(matches!(q.validate(), Err(SemanticSearchError::InvalidK)));
        let q2 = SemanticSearchQuery { q: "insulin".to_string(), k: 101 };
        assert!(matches!(q2.validate(), Err(SemanticSearchError::InvalidK)));
    }

    #[test]
    fn test_validate_ok() {
        let q = SemanticSearchQuery { q: "insulin".to_string(), k: 10 };
        assert!(q.validate().is_ok());
    }
}
