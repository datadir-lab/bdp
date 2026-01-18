//! Job queries

pub mod get_job;
pub mod get_sync_status;
pub mod list_jobs;

pub use get_job::{GetJobError, GetJobQuery, JobDetails};
pub use get_sync_status::{
    GetSyncStatusQuery, ListSyncStatusQuery, ListSyncStatusResponse, SyncStatusError,
    SyncStatusItem,
};
pub use list_jobs::{JobListItem, ListJobsError, ListJobsQuery, ListJobsResponse};
