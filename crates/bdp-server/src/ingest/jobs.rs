//! Job definitions for data ingestion
//!
//! Defines the job types and payloads for apalis job queue.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// UniProt ingestion job payload
///
/// This job is responsible for syncing UniProt data from their FTP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniProtIngestJob {
    /// Organization ID to sync data for
    pub organization_id: Uuid,
    /// Target version to sync (format: YYYY_MM)
    pub target_version: Option<String>,
    /// Whether this is a full sync or incremental
    pub full_sync: bool,
    /// Triggered by (user ID or system)
    pub triggered_by: Option<Uuid>,
    /// Timestamp when job was created
    pub created_at: DateTime<Utc>,
}

impl UniProtIngestJob {
    /// Create a new UniProt ingest job
    pub fn new(organization_id: Uuid, full_sync: bool) -> Self {
        Self {
            organization_id,
            target_version: None,
            full_sync,
            triggered_by: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new job with a specific version
    pub fn with_version(organization_id: Uuid, version: String, full_sync: bool) -> Self {
        Self {
            organization_id,
            target_version: Some(version),
            full_sync,
            triggered_by: None,
            created_at: Utc::now(),
        }
    }

    /// Set the user who triggered this job
    pub fn with_triggered_by(mut self, user_id: Uuid) -> Self {
        self.triggered_by = Some(user_id);
        self
    }
}

/// Statistics collected during ingestion
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IngestStats {
    /// Total entries processed
    pub total_entries: i64,
    /// Entries successfully inserted
    pub entries_inserted: i64,
    /// Entries updated
    pub entries_updated: i64,
    /// Entries skipped
    pub entries_skipped: i64,
    /// Entries that failed processing
    pub entries_failed: i64,
    /// Total bytes processed
    pub bytes_processed: i64,
    /// Duration in seconds
    pub duration_secs: f64,
    /// Version that was synced
    pub version_synced: Option<String>,
    /// Start time
    pub started_at: Option<DateTime<Utc>>,
    /// End time
    pub completed_at: Option<DateTime<Utc>>,
}

impl IngestStats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self {
            started_at: Some(Utc::now()),
            ..Default::default()
        }
    }

    /// Mark stats as completed
    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
        if let (Some(start), Some(end)) = (self.started_at, self.completed_at) {
            self.duration_secs = (end - start).num_milliseconds() as f64 / 1000.0;
        }
    }

    /// Increment insert count
    pub fn inc_inserted(&mut self) {
        self.entries_inserted += 1;
        self.total_entries += 1;
    }

    /// Increment update count
    pub fn inc_updated(&mut self) {
        self.entries_updated += 1;
        self.total_entries += 1;
    }

    /// Increment skip count
    pub fn inc_skipped(&mut self) {
        self.entries_skipped += 1;
        self.total_entries += 1;
    }

    /// Increment failed count
    pub fn inc_failed(&mut self) {
        self.entries_failed += 1;
        self.total_entries += 1;
    }

    /// Add bytes processed
    pub fn add_bytes(&mut self, bytes: i64) {
        self.bytes_processed += bytes;
    }

    /// Set the version that was synced
    pub fn set_version(&mut self, version: String) {
        self.version_synced = Some(version);
    }

    /// Calculate entries per second
    pub fn entries_per_second(&self) -> f64 {
        if self.duration_secs > 0.0 {
            self.total_entries as f64 / self.duration_secs
        } else {
            0.0
        }
    }

    /// Create empty stats (no time information)
    pub fn empty() -> Self {
        Self::default()
    }

    /// Merge another IngestStats into this one
    pub fn merge(self, other: Self) -> Self {
        Self {
            total_entries: self.total_entries + other.total_entries,
            entries_inserted: self.entries_inserted + other.entries_inserted,
            entries_updated: self.entries_updated + other.entries_updated,
            entries_skipped: self.entries_skipped + other.entries_skipped,
            entries_failed: self.entries_failed + other.entries_failed,
            bytes_processed: self.bytes_processed + other.bytes_processed,
            duration_secs: self.duration_secs + other.duration_secs,
            version_synced: other.version_synced.or(self.version_synced),
            started_at: self.started_at.or(other.started_at),
            completed_at: other.completed_at.or(self.completed_at),
        }
    }

    /// Calculate megabytes per second
    pub fn megabytes_per_second(&self) -> f64 {
        if self.duration_secs > 0.0 {
            (self.bytes_processed as f64 / 1_000_000.0) / self.duration_secs
        } else {
            0.0
        }
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_entries > 0 {
            ((self.entries_inserted + self.entries_updated) as f64 / self.total_entries as f64) * 100.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniprot_ingest_job_new() {
        let org_id = Uuid::new_v4();
        let job = UniProtIngestJob::new(org_id, false);

        assert_eq!(job.organization_id, org_id);
        assert!(!job.full_sync);
        assert!(job.target_version.is_none());
        assert!(job.triggered_by.is_none());
    }

    #[test]
    fn test_uniprot_ingest_job_with_version() {
        let org_id = Uuid::new_v4();
        let job = UniProtIngestJob::with_version(org_id, "2024_01".to_string(), true);

        assert_eq!(job.organization_id, org_id);
        assert!(job.full_sync);
        assert_eq!(job.target_version, Some("2024_01".to_string()));
    }

    #[test]
    fn test_uniprot_ingest_job_with_triggered_by() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let job = UniProtIngestJob::new(org_id, false).with_triggered_by(user_id);

        assert_eq!(job.triggered_by, Some(user_id));
    }

    #[test]
    fn test_ingest_stats_new() {
        let stats = IngestStats::new();
        assert_eq!(stats.total_entries, 0);
        assert!(stats.started_at.is_some());
        assert!(stats.completed_at.is_none());
    }

    #[test]
    fn test_ingest_stats_counters() {
        let mut stats = IngestStats::new();

        stats.inc_inserted();
        stats.inc_updated();
        stats.inc_skipped();
        stats.inc_failed();

        assert_eq!(stats.entries_inserted, 1);
        assert_eq!(stats.entries_updated, 1);
        assert_eq!(stats.entries_skipped, 1);
        assert_eq!(stats.entries_failed, 1);
        assert_eq!(stats.total_entries, 4);
    }

    #[test]
    fn test_ingest_stats_complete() {
        let mut stats = IngestStats::new();
        std::thread::sleep(std::time::Duration::from_millis(100));
        stats.complete();

        assert!(stats.completed_at.is_some());
        assert!(stats.duration_secs > 0.0);
    }

    #[test]
    fn test_ingest_stats_entries_per_second() {
        let mut stats = IngestStats::new();
        stats.total_entries = 1000;
        stats.duration_secs = 10.0;

        assert_eq!(stats.entries_per_second(), 100.0);
    }

    #[test]
    fn test_ingest_stats_megabytes_per_second() {
        let mut stats = IngestStats::new();
        stats.bytes_processed = 10_000_000; // 10 MB
        stats.duration_secs = 2.0;

        assert_eq!(stats.megabytes_per_second(), 5.0);
    }

    #[test]
    fn test_ingest_stats_success_rate() {
        let mut stats = IngestStats::new();
        stats.entries_inserted = 80;
        stats.entries_updated = 15;
        stats.entries_failed = 5;
        stats.total_entries = 100;

        assert_eq!(stats.success_rate(), 95.0);
    }

    #[test]
    fn test_ingest_stats_add_bytes() {
        let mut stats = IngestStats::new();
        stats.add_bytes(1000);
        stats.add_bytes(2000);

        assert_eq!(stats.bytes_processed, 3000);
    }

    #[test]
    fn test_ingest_stats_set_version() {
        let mut stats = IngestStats::new();
        stats.set_version("2024_05".to_string());

        assert_eq!(stats.version_synced, Some("2024_05".to_string()));
    }
}
