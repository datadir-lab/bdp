//! Data models for ingestion
//!
//! Models for tracking ingestion status and job state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Organization sync status
///
/// Tracks the current sync state for each organization (e.g., UniProt, ChEMBL).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrganizationSyncStatus {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_version: Option<String>,
    pub last_external_version: Option<String>,
    pub status: SyncStatus,
    pub total_entries: i64,
    pub last_job_id: Option<Uuid>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Sync status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum SyncStatus {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncStatus::Idle => write!(f, "idle"),
            SyncStatus::Running => write!(f, "running"),
            SyncStatus::Completed => write!(f, "completed"),
            SyncStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for SyncStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "idle" => Ok(SyncStatus::Idle),
            "running" => Ok(SyncStatus::Running),
            "completed" => Ok(SyncStatus::Completed),
            "failed" => Ok(SyncStatus::Failed),
            _ => Err(anyhow::anyhow!("Invalid sync status: {}", s)),
        }
    }
}

/// Create sync status request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSyncStatus {
    pub organization_id: Uuid,
}

/// Update sync status request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSyncStatus {
    pub status: Option<SyncStatus>,
    pub last_version: Option<String>,
    pub last_external_version: Option<String>,
    pub total_entries: Option<i64>,
    pub last_job_id: Option<Uuid>,
    pub last_error: Option<String>,
}

impl OrganizationSyncStatus {
    /// Check if sync is currently running
    pub fn is_running(&self) -> bool {
        self.status == SyncStatus::Running
    }

    /// Check if last sync was successful
    pub fn is_successful(&self) -> bool {
        self.status == SyncStatus::Completed && self.last_error.is_none()
    }

    /// Check if sync has failed
    pub fn is_failed(&self) -> bool {
        self.status == SyncStatus::Failed || self.last_error.is_some()
    }

    /// Get time since last sync
    pub fn time_since_last_sync(&self) -> Option<chrono::Duration> {
        self.last_sync_at.map(|t| Utc::now() - t)
    }
}

/// Apalis job record from database
///
/// This represents the structure of jobs stored by apalis in the apalis_jobs table.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApalisJob {
    pub id: String,
    pub job_type: String,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub run_at: DateTime<Utc>,
    pub done_at: Option<DateTime<Utc>>,
    pub lock_at: Option<DateTime<Utc>>,
    pub lock_by: Option<String>,
    pub last_error: Option<String>,
}

impl ApalisJob {
    /// Check if job is completed
    pub fn is_completed(&self) -> bool {
        self.status == "Done" || self.done_at.is_some()
    }

    /// Check if job has failed
    pub fn is_failed(&self) -> bool {
        self.status == "Failed" || (self.attempts >= self.max_attempts && self.last_error.is_some())
    }

    /// Check if job is running
    pub fn is_running(&self) -> bool {
        self.status == "Running" || self.lock_at.is_some()
    }

    /// Check if job is pending
    pub fn is_pending(&self) -> bool {
        self.status == "Pending" && self.lock_at.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_status_display() {
        assert_eq!(SyncStatus::Idle.to_string(), "idle");
        assert_eq!(SyncStatus::Running.to_string(), "running");
        assert_eq!(SyncStatus::Completed.to_string(), "completed");
        assert_eq!(SyncStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_sync_status_from_str() {
        assert_eq!("idle".parse::<SyncStatus>().unwrap(), SyncStatus::Idle);
        assert_eq!("running".parse::<SyncStatus>().unwrap(), SyncStatus::Running);
        assert_eq!("completed".parse::<SyncStatus>().unwrap(), SyncStatus::Completed);
        assert_eq!("failed".parse::<SyncStatus>().unwrap(), SyncStatus::Failed);
        assert!("invalid".parse::<SyncStatus>().is_err());
    }

    #[test]
    fn test_organization_sync_status_is_running() {
        let mut status = OrganizationSyncStatus {
            id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            last_sync_at: None,
            last_version: None,
            last_external_version: None,
            status: SyncStatus::Running,
            total_entries: 0,
            last_job_id: None,
            last_error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(status.is_running());
        status.status = SyncStatus::Idle;
        assert!(!status.is_running());
    }

    #[test]
    fn test_organization_sync_status_is_successful() {
        let mut status = OrganizationSyncStatus {
            id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            last_sync_at: None,
            last_version: None,
            last_external_version: None,
            status: SyncStatus::Completed,
            total_entries: 0,
            last_job_id: None,
            last_error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(status.is_successful());
        status.last_error = Some("error".to_string());
        assert!(!status.is_successful());
    }

    #[test]
    fn test_organization_sync_status_is_failed() {
        let mut status = OrganizationSyncStatus {
            id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            last_sync_at: None,
            last_version: None,
            last_external_version: None,
            status: SyncStatus::Failed,
            total_entries: 0,
            last_job_id: None,
            last_error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(status.is_failed());
        status.status = SyncStatus::Idle;
        status.last_error = Some("error".to_string());
        assert!(status.is_failed());
    }

    #[test]
    fn test_apalis_job_status_checks() {
        let job = ApalisJob {
            id: "test-job".to_string(),
            job_type: "UniProtIngestJob".to_string(),
            status: "Done".to_string(),
            attempts: 1,
            max_attempts: 3,
            run_at: Utc::now(),
            done_at: Some(Utc::now()),
            lock_at: None,
            lock_by: None,
            last_error: None,
        };

        assert!(job.is_completed());
        assert!(!job.is_failed());
        assert!(!job.is_running());
        assert!(!job.is_pending());
    }
}
