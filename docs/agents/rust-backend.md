# Rust Backend Development

Guide for developing the BDP Rust API server using axum and PostgreSQL.

## Project Structure

```
crates/bdp-server/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Library exports
│   ├── config.rs         # Configuration
│   ├── routes/           # HTTP handlers
│   │   ├── mod.rs
│   │   ├── packages.rs
│   │   ├── versions.rs
│   │   ├── search.rs
│   │   └── auth.rs
│   ├── models/           # Database models
│   │   ├── mod.rs
│   │   ├── package.rs
│   │   ├── version.rs
│   │   └── user.rs
│   ├── services/         # Business logic
│   │   ├── mod.rs
│   │   ├── package_service.rs
│   │   └── resolver.rs
│   ├── middleware/       # Custom middleware
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   └── rate_limit.rs
│   └── error.rs          # Error handling
├── migrations/           # SQL migrations
│   └── 20260116000000_initial.sql
└── tests/
    └── integration/
```

## Application Setup

### main.rs

```rust
use axum::{Router, routing::get};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::{trace::TraceLayer, cors::CorsLayer};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    // Load configuration
    let config = bdp_server::config::Config::from_env()?;

    // Database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    // Build application state
    let app_state = bdp_server::AppState {
        db: pool,
        config: config.clone(),
    };

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1", bdp_server::routes::api_routes())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())  // Configure in production
        .with_state(app_state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Starting server on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}
```

## Configuration

### config.rs

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub jwt_secret: String,
    pub storage_backend: StorageBackend,
    pub storage_endpoint: String,
    pub storage_bucket: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    S3,
    Minio,
    Local,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Config {
            database_url: std::env::var("DATABASE_URL")?,
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8000".to_string())
                .parse()?,
            jwt_secret: std::env::var("JWT_SECRET")?,
            storage_backend: std::env::var("STORAGE_BACKEND")
                .unwrap_or_else(|_| "local".to_string())
                .parse()
                .unwrap_or(StorageBackend::Local),
            storage_endpoint: std::env::var("STORAGE_ENDPOINT")?,
            storage_bucket: std::env::var("STORAGE_BUCKET")
                .unwrap_or_else(|_| "bdp-packages".to_string()),
        })
    }
}
```

## Database Models

### models/package.rs

```rust
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Package {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub repository_url: Option<String>,
    pub homepage_url: Option<String>,
    pub license: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub downloads_total: i64,
}

impl Package {
    /// Find package by name
    pub async fn find_by_name(
        pool: &PgPool,
        name: &str
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Package,
            r#"
            SELECT * FROM packages WHERE name = $1
            "#,
            name
        )
        .fetch_optional(pool)
        .await
    }

    /// List all packages with pagination
    pub async fn list(
        pool: &PgPool,
        page: i64,
        per_page: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let offset = (page - 1) * per_page;

        sqlx::query_as!(
            Package,
            r#"
            SELECT * FROM packages
            ORDER BY downloads_total DESC, name ASC
            LIMIT $1 OFFSET $2
            "#,
            per_page,
            offset
        )
        .fetch_all(pool)
        .await
    }

    /// Create new package
    pub async fn create(
        pool: &PgPool,
        name: &str,
        description: Option<&str>,
        repository_url: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Package,
            r#"
            INSERT INTO packages (name, description, repository_url)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
            name,
            description,
            repository_url
        )
        .fetch_one(pool)
        .await
    }
}
```

## Routes

### routes/packages.rs

```rust
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json, Router,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use crate::{AppState, models::Package, error::AppError};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/packages", get(list_packages).post(create_package))
        .route("/packages/:name", get(get_package))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    success: bool,
    data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<serde_json::Value>,
}

/// GET /packages - List all packages
async fn list_packages(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<Package>>>, AppError> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20).min(100);

    let packages = Package::list(&state.db, page, per_page).await?;

    Ok(Json(ApiResponse {
        success: true,
        data: packages,
        meta: Some(serde_json::json!({
            "pagination": {
                "page": page,
                "per_page": per_page
            }
        })),
    }))
}

/// GET /packages/:name - Get package details
async fn get_package(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<Package>>, AppError> {
    let package = Package::find_by_name(&state.db, &name)
        .await?
        .ok_or(AppError::NotFound(format!("Package '{}' not found", name)))?;

    Ok(Json(ApiResponse {
        success: true,
        data: package,
        meta: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreatePackageRequest {
    name: String,
    description: Option<String>,
    repository_url: Option<String>,
}

/// POST /packages - Create new package
async fn create_package(
    State(state): State<AppState>,
    Json(payload): Json<CreatePackageRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Package>>), AppError> {
    // TODO: Add authentication check

    let package = Package::create(
        &state.db,
        &payload.name,
        payload.description.as_deref(),
        payload.repository_url.as_deref(),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse {
            success: true,
            data: package,
            meta: None,
        }),
    ))
}
```

## Error Handling

### error.rs

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    Database(sqlx::Error),
    NotFound(String),
    Unauthorized,
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::Database(err) => {
                tracing::error!("Database error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "An internal error occurred".to_string(),
                )
            }
            AppError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, "NOT_FOUND", msg)
            }
            AppError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "Unauthorized".to_string())
            }
            AppError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg)
            }
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg)
            }
        };

        let body = Json(json!({
            "success": false,
            "error": {
                "code": code,
                "message": message
            }
        }));

        (status, body).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}
```

## Best Practices

### 1. Use Type-Safe Extractors

```rust
use axum::extract::State;

async fn handler(
    State(state): State<AppState>,  // ✅ Type-safe state
    Path(id): Path<Uuid>,           // ✅ Validated UUID
    Json(body): Json<RequestBody>,  // ✅ Deserialized & validated
) -> Result<Json<Response>, AppError> {
    // Handler logic
}
```

### 2. Separate Business Logic

Keep route handlers thin, move logic to services:

```rust
// routes/packages.rs
async fn create_package(
    State(state): State<AppState>,
    Json(req): Json<CreatePackageRequest>,
) -> Result<Json<Package>, AppError> {
    let package = services::package_service::create_package(&state.db, req).await?;
    Ok(Json(package))
}

// services/package_service.rs
pub async fn create_package(
    db: &PgPool,
    req: CreatePackageRequest,
) -> Result<Package, AppError> {
    // Validation
    // Business logic
    // Database operations
}
```

### 3. Use Middleware for Cross-Cutting Concerns

```rust
use tower_http::request_id::{MakeRequestId, RequestId};

Router::new()
    .route("/packages", get(list_packages))
    .layer(SetRequestIdLayer::new(
        HeaderName::from_static("x-request-id"),
        MyMakeRequestId::default(),
    ))
```

### 4. Structured Logging

```rust
use tracing::{info, error, instrument};

#[instrument(skip(db))]
async fn find_package(db: &PgPool, name: &str) -> Result<Package, AppError> {
    info!("Searching for package: {}", name);

    let pkg = Package::find_by_name(db, name).await?;

    match pkg {
        Some(p) => {
            info!("Found package: {}", p.id);
            Ok(p)
        }
        None => {
            error!("Package not found: {}", name);
            Err(AppError::NotFound(format!("Package '{}' not found", name)))
        }
    }
}
```

### 5. Database Migrations

```sql
-- migrations/20260116000000_initial.sql
CREATE TABLE IF NOT EXISTS packages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Add indexes
CREATE INDEX idx_packages_name ON packages(name);
```

Run migrations:
```bash
sqlx migrate run
```

## Testing

### Integration Tests

```rust
// tests/integration/packages.rs
use bdp_server::AppState;
use sqlx::PgPool;

#[sqlx::test]
async fn test_create_package(pool: PgPool) {
    let app_state = AppState { db: pool, config: test_config() };

    let req = CreatePackageRequest {
        name: "test-package".to_string(),
        description: Some("Test".to_string()),
        repository_url: None,
    };

    let result = services::package_service::create_package(&app_state.db, req).await;

    assert!(result.is_ok());
    let package = result.unwrap();
    assert_eq!(package.name, "test-package");
}
```

## Performance Tips

1. **Connection Pooling**: Use appropriate pool size (usually 2-5x CPU cores)
2. **Prepared Statements**: SQLx automatically prepares queries
3. **Batch Operations**: Use `query_as_unchecked` for bulk inserts
4. **Indexes**: Add indexes for frequently queried columns
5. **Lazy Loading**: Don't fetch all related data upfront

## Resources

- [axum Documentation](https://docs.rs/axum/)
- [SQLx Documentation](https://docs.rs/sqlx/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Rust Async Book](https://rust-lang.github.io/async-book/)

---

**Next**: See [CLI Development](./cli-development.md) for CLI implementation.
