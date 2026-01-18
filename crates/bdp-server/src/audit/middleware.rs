//! Audit logging middleware for tracking commands
//!
//! This middleware implements comprehensive audit logging following CQRS principles:
//! - Only commands (POST, PUT, PATCH, DELETE) are audited
//! - Queries (GET) are not audited to reduce noise
//! - Captures request body for commands
//! - Extracts user info from auth headers (if present)
//! - Logs after successful command execution
//! - Uses structured logging via tracing

use axum::{
    body::{Body, Bytes},
    extract::{ConnectInfo, Request},
    http::Method,
    response::Response,
};
use http_body_util::BodyExt;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use std::{
    future::Future,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::models::{AuditAction, CreateAuditEntry, ResourceType};
use super::queries::create_audit_entry;

/// Audit logging layer
///
/// This layer wraps services to provide automatic audit logging for
/// command operations (write operations).
#[derive(Clone)]
pub struct AuditLayer {
    pool: PgPool,
}

impl AuditLayer {
    /// Create a new audit layer with database pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl<S> Layer<S> for AuditLayer {
    type Service = AuditMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuditMiddleware {
            inner,
            pool: self.pool.clone(),
        }
    }
}

/// Audit middleware service
#[derive(Clone)]
pub struct AuditMiddleware<S> {
    inner: S,
    pool: PgPool,
}

impl<S> Service<Request> for AuditMiddleware<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::fmt::Display,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let mut inner = self.inner.clone();
        let pool = self.pool.clone();

        Box::pin(async move {
            let method = request.method().clone();
            let uri = request.uri().clone();
            let headers = request.headers().clone();

            // Extract client info
            let ip_address = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ci| ci.0.ip().to_string());

            let user_agent = headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            // Extract user ID from authorization header (if present)
            // In a real implementation, this would parse a JWT or API key
            let user_id = headers
                .get("x-user-id")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| Uuid::parse_str(s).ok());

            // Only audit commands (write operations), not queries (read operations)
            let should_audit =
                matches!(method, Method::POST | Method::PUT | Method::PATCH | Method::DELETE);

            // Capture request body for commands
            let (parts, body) = request.into_parts();
            let body_bytes = if should_audit {
                match body.collect().await {
                    Ok(collected) => {
                        let bytes = collected.to_bytes();
                        debug!(
                            method = %method,
                            uri = %uri,
                            body_size = bytes.len(),
                            "Captured request body"
                        );
                        bytes
                    },
                    Err(e) => {
                        warn!(
                            method = %method,
                            uri = %uri,
                            error = %e,
                            "Failed to capture request body"
                        );
                        Bytes::new()
                    },
                }
            } else {
                Bytes::new()
            };

            // Reconstruct request with captured body
            let request = Request::from_parts(parts, Body::from(body_bytes.clone()));

            if should_audit {
                debug!(
                    method = %method,
                    uri = %uri,
                    ip = ?ip_address,
                    user_id = ?user_id,
                    "Auditable command received"
                );
            }

            // Call the inner service
            let response = inner.call(request).await?;

            // Log audit entry after successful command execution
            if should_audit && response.status().is_success() {
                let action = infer_action(&method, &uri);
                let (resource_type, resource_id) = infer_resource(&uri);

                // Parse request body as JSON (if possible)
                let changes = if !body_bytes.is_empty() {
                    serde_json::from_slice::<JsonValue>(&body_bytes).ok()
                } else {
                    None
                };

                // Build metadata
                let mut metadata = serde_json::Map::new();
                metadata.insert("method".to_string(), JsonValue::String(method.to_string()));
                metadata.insert("uri".to_string(), JsonValue::String(uri.to_string()));
                metadata.insert(
                    "status".to_string(),
                    JsonValue::Number(response.status().as_u16().into()),
                );

                let audit_entry = CreateAuditEntry {
                    user_id,
                    action,
                    resource_type,
                    resource_id,
                    changes,
                    metadata: Some(JsonValue::Object(metadata)),
                    ip_address,
                    user_agent,
                };

                // Log to database (non-blocking, fire and forget)
                tokio::spawn(async move {
                    match create_audit_entry(&pool, audit_entry).await {
                        Ok(entry) => {
                            info!(
                                audit_id = %entry.id,
                                action = %entry.action,
                                resource_type = %entry.resource_type,
                                "Audit log entry created"
                            );
                        },
                        Err(e) => {
                            error!(
                                error = %e,
                                "Failed to create audit log entry"
                            );
                        },
                    }
                });

                debug!(
                    method = %method,
                    uri = %uri,
                    status = %response.status(),
                    "Command executed successfully"
                );
            } else if should_audit {
                warn!(
                    method = %method,
                    uri = %uri,
                    status = %response.status(),
                    "Command failed or returned non-success status"
                );
            }

            Ok(response)
        })
    }
}

/// Infer audit action from HTTP method and URI
fn infer_action(method: &Method, uri: &axum::http::Uri) -> AuditAction {
    match method {
        &Method::POST => {
            if uri.path().contains("/login") {
                AuditAction::Login
            } else if uri.path().contains("/logout") {
                AuditAction::Logout
            } else if uri.path().contains("/register") {
                AuditAction::Register
            } else if uri.path().contains("/upload") {
                AuditAction::Upload
            } else if uri.path().contains("/publish") {
                AuditAction::Publish
            } else {
                AuditAction::Create
            }
        },
        &Method::PUT | &Method::PATCH => AuditAction::Update,
        &Method::DELETE => {
            if uri.path().contains("/archive") {
                AuditAction::Archive
            } else {
                AuditAction::Delete
            }
        },
        _ => AuditAction::Other,
    }
}

/// Infer resource type and ID from URI
fn infer_resource(uri: &axum::http::Uri) -> (ResourceType, Option<Uuid>) {
    let path = uri.path();

    // Extract resource ID (UUID) from path if present
    let resource_id = path
        .split('/')
        .find_map(|segment| Uuid::parse_str(segment).ok());

    // Infer resource type from path
    let resource_type = if path.contains("/organizations") {
        ResourceType::Organization
    } else if path.contains("/sources") || path.contains("/data_sources") {
        ResourceType::DataSource
    } else if path.contains("/versions") {
        ResourceType::Version
    } else if path.contains("/tools") {
        ResourceType::Tool
    } else if path.contains("/entries") {
        ResourceType::RegistryEntry
    } else if path.contains("/files") {
        ResourceType::VersionFile
    } else if path.contains("/dependencies") {
        ResourceType::Dependency
    } else if path.contains("/organisms") {
        ResourceType::Organism
    } else if path.contains("/proteins") {
        ResourceType::ProteinMetadata
    } else if path.contains("/citations") {
        ResourceType::Citation
    } else if path.contains("/tags") {
        ResourceType::Tag
    } else if path.contains("/downloads") {
        ResourceType::Download
    } else if path.contains("/mappings") {
        ResourceType::VersionMapping
    } else if path.contains("/users") {
        ResourceType::User
    } else if path.contains("/sessions") {
        ResourceType::Session
    } else if path.contains("/api_keys") {
        ResourceType::ApiKey
    } else {
        ResourceType::Other
    };

    (resource_type, resource_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::{get, post},
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "ok"
    }

    #[test]
    fn test_infer_action() {
        let uri: axum::http::Uri = "/api/v1/organizations".parse().unwrap();
        assert_eq!(infer_action(&Method::POST, &uri), AuditAction::Create);
        assert_eq!(infer_action(&Method::PUT, &uri), AuditAction::Update);
        assert_eq!(infer_action(&Method::DELETE, &uri), AuditAction::Delete);

        let login_uri: axum::http::Uri = "/api/v1/login".parse().unwrap();
        assert_eq!(infer_action(&Method::POST, &login_uri), AuditAction::Login);
    }

    #[test]
    fn test_infer_resource() {
        let uri: axum::http::Uri = "/api/v1/organizations".parse().unwrap();
        let (resource_type, resource_id) = infer_resource(&uri);
        assert_eq!(resource_type, ResourceType::Organization);
        assert!(resource_id.is_none());

        let uuid = Uuid::new_v4();
        let uri_with_id: axum::http::Uri =
            format!("/api/v1/organizations/{}", uuid).parse().unwrap();
        let (resource_type, resource_id) = infer_resource(&uri_with_id);
        assert_eq!(resource_type, ResourceType::Organization);
        assert_eq!(resource_id, Some(uuid));
    }

    #[test]
    fn test_infer_various_resources() {
        let test_cases = vec![
            ("/api/v1/sources", ResourceType::DataSource),
            ("/api/v1/versions", ResourceType::Version),
            ("/api/v1/tools", ResourceType::Tool),
            ("/api/v1/organisms", ResourceType::Organism),
            ("/api/v1/users", ResourceType::User),
        ];

        for (path, expected_type) in test_cases {
            let uri: axum::http::Uri = path.parse().unwrap();
            let (resource_type, _) = infer_resource(&uri);
            assert_eq!(resource_type, expected_type);
        }
    }
}
