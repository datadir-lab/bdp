//! Get tile query
//!
//! Fetches a pre-rendered JSON tile from S3-compatible storage for the
//! WebGPU graph view. Tiles are stored under the key:
//! `vectors/tiles/{run_id}/{z}/{x}/{y}.json`

use crate::storage::Storage;
use mediator::Request;
use serde::{Deserialize, Serialize};

/// Query to fetch a single map tile for the vectors graph view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTileQuery {
    pub run_id: String,
    pub z: u32,
    pub x: u32,
    pub y: u32,
}

/// Raw tile bytes returned from storage
#[derive(Debug, Clone)]
pub struct TileResponse {
    pub body: Vec<u8>,
}

/// Errors that can occur when fetching a tile
#[derive(Debug, thiserror::Error)]
pub enum GetTileError {
    #[error("Tile not found")]
    NotFound,
    #[error("Storage error: {0}")]
    Storage(String),
}

impl Request<Result<TileResponse, GetTileError>> for GetTileQuery {}

impl crate::cqrs::middleware::Query for GetTileQuery {}

/// Handles the get tile query
///
/// Fetches raw tile bytes from S3 storage. Tiles are immutable once written,
/// so the route handler applies a long-lived cache-control header.
///
/// # Arguments
///
/// * `storage` - S3-compatible storage backend
/// * `query` - Tile coordinates (run_id, z, x, y)
///
/// # Errors
///
/// - `NotFound` - No tile exists at the given coordinates
/// - `Storage` - An error occurred in the storage backend
#[tracing::instrument(skip(storage))]
pub async fn handle(
    storage: Storage,
    query: GetTileQuery,
) -> Result<TileResponse, GetTileError> {
    let key = format!(
        "vectors/tiles/{}/{}/{}/{}.json",
        query.run_id, query.z, query.x, query.y
    );

    storage
        .download(&key)
        .await
        .map(|body| TileResponse { body })
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("NoSuchKey") || msg.contains("404") {
                GetTileError::NotFound
            } else {
                GetTileError::Storage(msg)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_fields() {
        let q = GetTileQuery {
            run_id: "abc123".to_string(),
            z: 3,
            x: 1,
            y: 2,
        };
        assert_eq!(q.run_id, "abc123");
        assert_eq!(q.z, 3);
    }
}
