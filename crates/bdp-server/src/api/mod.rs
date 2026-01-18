pub mod response;

use crate::config::Config;
use crate::db;
use crate::features;
use crate::storage::Storage;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use sqlx::PgPool;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub storage: Storage,
}

pub async fn serve(config: Config) -> anyhow::Result<()> {
    let db_config = db::DbConfig::from_env()?;
    let db = db::create_pool(&db_config).await?;

    let storage_config = crate::storage::config::StorageConfig::from_env()?;
    let storage = Storage::new(storage_config).await?;

    let state = AppState { db, storage };
    let app = create_router(state);

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn create_router(state: AppState) -> Router {
    let feature_state = features::FeatureState {
        db: state.db.clone(),
        storage: state.storage.clone(),
    };

    let api_v1 = features::router(feature_state);

    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .nest("/api/v1", api_v1)
        .layer(TraceLayer::new_for_http())
}

async fn root() -> impl IntoResponse {
    Json(json!({
        "name": "BDP Server",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running"
    }))
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

pub struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": self.0.to_string()
            })),
        )
            .into_response()
    }
}

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
