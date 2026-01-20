/**
 * Job status types matching the ingestion_jobs table
 */
export type JobStatus = 'pending' | 'downloading' | 'download_verified' | 'parsing' | 'storing' | 'completed' | 'failed';

/**
 * Job type for ingestion jobs
 */
export type JobType = 'UniProt' | 'NCBI Taxonomy' | 'GenBank';

/**
 * Job interface matching the ingestion_jobs table structure
 */
export interface Job {
  id: string;
  job_type: string;
  status: string;
  started_at: string | null;
  completed_at: string | null;
  created_at: string;
  total_records: number | null;
  records_processed: number;
  records_stored: number;
  records_failed: number;
}

/**
 * Sync status interface matching the organization_sync_status table
 */
export interface SyncStatus {
  id: string;
  organization_id: string;
  last_sync_at: string | null;
  last_version: string | null;
  last_external_version: string | null;
  status: string;
  total_entries: number;
  last_job_id: string | null;
  last_error: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * API response wrapper for jobs list
 */
export interface JobsListResponse {
  jobs: Job[];
  total: number;
}

/**
 * API response wrapper for sync status list
 */
export interface SyncStatusListResponse {
  statuses: SyncStatus[];
}

/**
 * Organization job summary combining organization, jobs, and sync status
 */
export interface OrganizationJobSummary {
  organization: {
    id: string;
    name: string;
    description: string | null;
    logo_url: string | null;
    website_url: string | null;
  };
  recent_jobs: Job[];
  sync_status: SyncStatus | null;
  current_status: JobStatus | 'idle';
}
