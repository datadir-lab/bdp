use crate::features::data_sources::types::{
    ProteinComment, ProteinCrossReference, ProteinFeature,
};
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
}

#[derive(Debug, Deserialize)]
pub struct ProteinMetadataParams {
    pub org: String,
    pub slug: String,
    pub version: String,
}

pub async fn get_protein_metadata(
    State(pool): State<PgPool>,
    Path(params): Path<ProteinMetadataParams>,
) -> Result<impl IntoResponse, (StatusCode, String)> {

    // First, get the data source ID
    let data_source_id = get_data_source_id(&pool, &params.org, &params.slug)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("Data source not found: {}", e)))?;

    // Fetch protein comments
    let comments = sqlx::query!(
        r#"
        SELECT topic, text
        FROM protein_comments
        WHERE protein_id = $1
        ORDER BY topic
        "#,
        data_source_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch protein comments: {}", e),
        )
    })?
    .into_iter()
    .map(|r| ProteinComment {
        topic: r.topic,
        text: r.text,
    })
    .collect::<Vec<_>>();

    // Fetch protein features (limit to reasonable number)
    let features = sqlx::query!(
        r#"
        SELECT feature_type, description, start_pos, end_pos
        FROM protein_features
        WHERE protein_id = $1
        ORDER BY start_pos NULLS LAST, feature_type
        "#,
        data_source_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch protein features: {}", e),
        )
    })?
    .into_iter()
    .map(|r| ProteinFeature {
        feature_type: r.feature_type,
        description: r.description,
        start_pos: r.start_pos,
        end_pos: r.end_pos,
    })
    .collect::<Vec<_>>();

    // Fetch protein cross references
    let cross_refs = sqlx::query!(
        r#"
        SELECT database, database_id, metadata
        FROM protein_cross_references
        WHERE protein_id = $1
        ORDER BY database, database_id
        "#,
        data_source_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch protein cross references: {}", e),
        )
    })?
    .into_iter()
    .map(|r| ProteinCrossReference {
        database: r.database,
        database_id: r.database_id,
        metadata: r.metadata,
    })
    .collect::<Vec<_>>();

    let response = ProteinMetadataResponse {
        comments,
        features,
        cross_references: cross_refs,
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

async fn get_data_source_id(pool: &PgPool, org: &str, slug: &str) -> Result<Uuid, sqlx::Error> {
    let result = sqlx::query!(
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
