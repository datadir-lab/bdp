use crate::features::data_sources::types::{
    ProteinComment, ProteinCrossReference, ProteinFeature, ProteinPublication,
};
use crate::features::FeatureState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProteinMetadataResponse {
    pub comments: Vec<ProteinComment>,
    pub features: Vec<ProteinFeature>,
    pub cross_references: Vec<ProteinCrossReference>,
    pub publications: Vec<ProteinPublication>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProteinMetadataQuery {
    pub org: String,
    pub slug: String,
    pub version: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GetProteinMetadataError {
    #[error("Data source not found: {0}")]
    NotFound(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl mediator::Request<Result<ProteinMetadataResponse, GetProteinMetadataError>>
    for GetProteinMetadataQuery
{
}

/// Handle protein metadata query
pub async fn handle(
    pool: PgPool,
    query: GetProteinMetadataQuery,
) -> Result<ProteinMetadataResponse, GetProteinMetadataError> {
    // First, get the data source ID
    let data_source_id = get_data_source_id(&pool, &query.org, &query.slug)
        .await
        .map_err(|e| GetProteinMetadataError::NotFound(format!("Data source not found: {}", e)))?;

    // Fetch protein comments
    let comments: Vec<_> = sqlx::query!(
        r#"
        SELECT topic, text
        FROM protein_comments
        WHERE protein_id = $1
        ORDER BY topic
        "#,
        data_source_id
    )
    .fetch_all(&pool)
    .await?
    .into_iter()
    .map(|r| ProteinComment {
        topic: r.topic,
        text: r.text,
    })
    .collect::<Vec<_>>();

    // Fetch protein features (limit to reasonable number)
    let features: Vec<_> = sqlx::query!(
        r#"
        SELECT feature_type, description, start_pos, end_pos
        FROM protein_features
        WHERE protein_id = $1
        ORDER BY start_pos NULLS LAST, feature_type
        "#,
        data_source_id
    )
    .fetch_all(&pool)
    .await?
    .into_iter()
    .map(|r| ProteinFeature {
        feature_type: r.feature_type,
        description: r.description,
        start_pos: r.start_pos,
        end_pos: r.end_pos,
    })
    .collect::<Vec<_>>();

    // Fetch protein cross references
    let cross_refs: Vec<_> = sqlx::query!(
        r#"
        SELECT database, database_id, metadata
        FROM protein_cross_references
        WHERE protein_id = $1
        ORDER BY database, database_id
        "#,
        data_source_id
    )
    .fetch_all(&pool)
    .await?
    .into_iter()
    .map(|r| ProteinCrossReference {
        database: r.database,
        database_id: r.database_id,
        metadata: r.metadata,
    })
    .collect::<Vec<_>>();

    // Fetch protein publications
    let publications: Vec<_> = sqlx::query!(
        r#"
        SELECT reference_number, position,
               comments as "comments!: Vec<String>",
               pubmed_id, doi,
               author_group,
               authors as "authors!: Vec<String>",
               title, location
        FROM protein_publications
        WHERE protein_id = $1
        ORDER BY reference_number
        "#,
        data_source_id
    )
    .fetch_all(&pool)
    .await?
    .into_iter()
    .map(|r| ProteinPublication {
        reference_number: r.reference_number,
        position: r.position,
        comments: r.comments,
        pubmed_id: r.pubmed_id,
        doi: r.doi,
        author_group: r.author_group,
        authors: r.authors,
        title: r.title,
        location: r.location,
    })
    .collect::<Vec<_>>();

    Ok(ProteinMetadataResponse {
        comments,
        features,
        cross_references: cross_refs,
        publications,
    })
}

/// Axum route handler that dispatches through the mediator
pub async fn get_protein_metadata(
    State(state): State<FeatureState>,
    Path(params): Path<ProteinMetadataParams>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let query = GetProteinMetadataQuery {
        org: params.org,
        slug: params.slug,
        version: params.version,
    };

    match state.dispatch(query).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "data": response
        }))),
        Err(GetProteinMetadataError::NotFound(msg)) => Err((StatusCode::NOT_FOUND, msg)),
        Err(GetProteinMetadataError::Database(e)) => {
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))
        },
    }
}

#[derive(Debug, Deserialize)]
pub struct ProteinMetadataParams {
    pub org: String,
    pub slug: String,
    pub version: String,
}

async fn get_data_source_id(pool: &PgPool, org: &str, slug: &str) -> Result<Uuid, sqlx::Error> {
    let result: _ = sqlx::query!(
        r#"
        SELECT ds.id
        FROM data_sources ds
        JOIN registry_entries re ON ds.id = re.id
        JOIN organizations o ON re.organization_id = o.id
        WHERE LOWER(o.slug) = LOWER($1) AND LOWER(re.slug) = LOWER($2)
        "#,
        org,
        slug
    )
    .fetch_one(pool)
    .await?;

    Ok(result.id)
}
