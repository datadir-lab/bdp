//! BDP Server - Main entry point

use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use bdp_common::logging::{init_logging, LogConfig};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, time::Duration};
use tokio::signal;
use tower_http::compression::CompressionLayer;
use tracing::info;

use bdp_server::{
    audit, config::Config, features, ingest, middleware,
    storage::{config::StorageConfig, Storage},
};

/// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    db: sqlx::PgPool,
    storage: Storage,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with configuration from environment
    let log_config = LogConfig::builder()
        .log_file_prefix("bdp-server".to_string())
        .filter_directives("bdp_server=debug,tower_http=debug,axum=trace,sqlx=info".to_string())
        .build();

    // Merge with environment variables (they take precedence)
    let log_config = LogConfig::from_env().unwrap_or(log_config);

    init_logging(&log_config)?;

    info!("Starting BDP Server");

    // Load configuration
    let config = Config::load()?;
    info!(
        "Configuration loaded - server will bind to {}:{}",
        config.server.host, config.server.port
    );

    // Initialize database connection pool
    let db_pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .min_connections(config.database.min_connections)
        .acquire_timeout(Duration::from_secs(config.database.connect_timeout_secs))
        .idle_timeout(Duration::from_secs(config.database.idle_timeout_secs))
        .connect(&config.database.url)
        .await?;

    info!("Database connection pool established");

    // Initialize S3/MinIO storage
    let storage_config = StorageConfig::from_env()?;
    let storage = Storage::new(storage_config).await?;
    info!("Storage client initialized");

    // Run migrations
    sqlx::migrate!("../../migrations")
        .run(&db_pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;

    info!("Database migrations completed");

    // Start ingestion orchestrator if enabled
    let _orchestrator_handle = if let Ok(ingest_config) = ingest::IngestConfig::from_env() {
        if ingest_config.enabled {
            info!("Ingestion is enabled, starting orchestrator");

            // Get or create UniProt organization
            let org_id = get_or_create_uniprot_org(&db_pool).await?;
            info!("Using UniProt organization: {}", org_id);

            // Start orchestrator
            let orchestrator = ingest::IngestOrchestrator::new(
                ingest_config,
                std::sync::Arc::new(db_pool.clone()),
                storage.clone(),
                org_id,
            );
            let handle = orchestrator.start();
            info!("Ingestion orchestrator started successfully");
            Some(handle)
        } else {
            info!("Ingestion is disabled (INGEST_ENABLED=false)");
            None
        }
    } else {
        info!("Ingestion configuration not found or invalid, orchestrator not started");
        None
    };

    // Create application state
    let state = AppState {
        db: db_pool,
        storage,
    };

    // Build the application router
    let app = create_router(state, &config);

    // Create socket address
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    info!("Server listening on {}", addr);

    // Create TCP listener
    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Start server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(config.server.shutdown_timeout_secs))
        .await?;

    info!("Server shut down gracefully");

    Ok(())
}

/// Create the application router with all routes and middleware
fn create_router(state: AppState, config: &Config) -> Router {
    // Create feature state
    let feature_state = features::FeatureState {
        db: state.db.clone(),
        storage: state.storage.clone(),
    };

    // Feature routes (CQRS architecture) - these have mixed states internally
    let feature_routes = features::router(feature_state);

    // Build the main router with middleware stack
    Router::new()
        // .route("/", get(root))  // Commented out to avoid conflicts with feature routes
        .route("/health", get(health_check))
        .route("/stats", get(get_stats))
        .route("/audit", get(query_audit_logs))
        .route("/organizations", get(list_organizations))
        .route("/sources", get(list_sources))
        .with_state(state.clone())
        .nest("/api/v1", feature_routes)
        // Apply layers from innermost to outermost
        .layer(CompressionLayer::new())
        .layer(middleware::tracing_layer())
        .layer(middleware::cors_layer(&config.cors))
        .layer(audit::AuditLayer::new(state.db.clone()))
}

/// Health check handler
async fn health_check(State(state): State<AppState>) -> Result<Response, StatusCode> {
    // Check database connectivity
    match sqlx::query("SELECT 1").fetch_one(&state.db).await {
        Ok(_) => Ok((
            StatusCode::OK,
            Json(json!({
                "status": "healthy",
                "database": "connected"
            })),
        )
            .into_response()),
        Err(e) => {
            tracing::error!("Database health check failed: {:?}", e);
            Err(StatusCode::SERVICE_UNAVAILABLE)
        },
    }
}

/// List organizations handler (placeholder)
async fn list_organizations(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query!("SELECT id, slug, name, website, is_system FROM organizations LIMIT 10")
        .fetch_all(&state.db)
        .await
    {
        Ok(orgs) => {
            let org_list: Vec<_> = orgs
                .iter()
                .map(|org| {
                    json!({
                        "id": org.id,
                        "slug": org.slug,
                        "name": org.name,
                        "website": org.website,
                        "is_system": org.is_system
                    })
                })
                .collect();

            (StatusCode::OK, Json(json!({ "organizations": org_list }))).into_response()
        },
        Err(e) => {
            tracing::error!("Failed to fetch organizations: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to fetch organizations" })),
            )
                .into_response()
        },
    }
}

/// List sources handler (placeholder)
async fn list_sources() -> impl IntoResponse {
    Json(json!({
        "sources": []
    }))
}

/// Get platform statistics
async fn get_stats(State(state): State<AppState>) -> impl IntoResponse {
    // Query all stats in parallel
    let datasources_result = sqlx::query!("SELECT COUNT(*) as count FROM data_sources")
        .fetch_one(&state.db);

    let organizations_result = sqlx::query!("SELECT COUNT(*) as count FROM organizations")
        .fetch_one(&state.db);

    let downloads_result = sqlx::query!("SELECT COUNT(*) as count FROM downloads")
        .fetch_one(&state.db);

    // Execute all queries concurrently
    let (datasources_res, organizations_res, downloads_res) =
        tokio::join!(datasources_result, organizations_result, downloads_result);

    match (datasources_res, organizations_res, downloads_res) {
        (Ok(ds), Ok(orgs), Ok(dl)) => {
            (
                StatusCode::OK,
                Json(json!({
                    "datasources": ds.count.unwrap_or(0),
                    "organizations": orgs.count.unwrap_or(0),
                    "downloads": dl.count.unwrap_or(0)
                })),
            )
                .into_response()
        }
        _ => {
            tracing::error!("Failed to fetch stats from database");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to fetch statistics" })),
            )
                .into_response()
        }
    }
}

/// Query audit logs handler
async fn query_audit_logs(
    State(state): State<AppState>,
    Query(query): Query<audit::AuditQuery>,
) -> Result<Response, StatusCode> {
    match audit::query_audit_logs(&state.db, query).await {
        Ok(logs) => Ok((StatusCode::OK, Json(json!({ "data": logs }))).into_response()),
        Err(e) => {
            tracing::error!("Failed to query audit logs: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        },
    }
}

/// Graceful shutdown signal handler
async fn shutdown_signal(timeout_secs: u64) {
    let ctrl_c = async {
        if let Err(e) = signal::ctrl_c().await {
            tracing::error!("Failed to install Ctrl+C handler: {}", e);
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => {
                tracing::error!("Failed to install SIGTERM handler: {}", e);
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, starting graceful shutdown");
        },
        _ = terminate => {
            info!("Received terminate signal, starting graceful shutdown");
        },
    }

    // Give ongoing requests time to complete
    info!("Waiting up to {} seconds for connections to close", timeout_secs);
    tokio::time::sleep(Duration::from_secs(timeout_secs.min(5))).await;
}

/// Get or create UniProt organization
async fn get_or_create_uniprot_org(pool: &sqlx::PgPool) -> Result<uuid::Uuid> {
    const UNIPROT_SLUG: &str = "uniprot";

    // Check for existing UniProt organization by slug
    let result = sqlx::query!(
        r#"SELECT id FROM organizations WHERE slug = $1"#,
        UNIPROT_SLUG
    )
    .fetch_optional(pool)
    .await?;

    if let Some(record) = result {
        Ok(record.id)
    } else {
        // Create organization
        let id = uuid::Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, name, slug, description, is_system)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (slug) DO NOTHING
            "#,
            id,
            "Universal Protein Resource",
            UNIPROT_SLUG,
            "UniProt Knowledgebase - Protein sequences and functional information",
            true
        )
        .execute(pool)
        .await?;

        // Fetch the ID in case another process created it concurrently
        let record = sqlx::query!(
            r#"SELECT id FROM organizations WHERE slug = $1"#,
            UNIPROT_SLUG
        )
        .fetch_one(pool)
        .await?;

        Ok(record.id)
    }
}
