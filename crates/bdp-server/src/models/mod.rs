//! Database models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Dataset model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Dataset {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Dataset version model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DatasetVersion {
    pub id: i64,
    pub dataset_id: i64,
    pub version: String,
    pub checksum: String,
    pub size: i64,
    pub file_path: String,
    pub created_at: DateTime<Utc>,
}

/// Dataset file model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DatasetFile {
    pub id: i64,
    pub version_id: i64,
    pub file_name: String,
    pub checksum: String,
    pub size: i64,
    pub created_at: DateTime<Utc>,
}

impl Dataset {
    /// Create a new dataset
    pub fn new(name: String, description: Option<String>) -> Self {
        Self {
            id: 0, // Will be set by database
            name,
            description,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl DatasetVersion {
    /// Create a new dataset version
    pub fn new(
        dataset_id: i64,
        version: String,
        checksum: String,
        size: i64,
        file_path: String,
    ) -> Self {
        Self {
            id: 0, // Will be set by database
            dataset_id,
            version,
            checksum,
            size,
            file_path,
            created_at: Utc::now(),
        }
    }
}
