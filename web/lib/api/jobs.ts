// Jobs API Client

import { apiClient } from '@/lib/api-client';
import type {
  Job,
  SyncStatus,
  JobsListResponse,
  SyncStatusListResponse,
} from '@/lib/types/job';

export interface ListJobsParams {
  job_type?: string;
  status?: string;
  limit?: number;
  offset?: number;
}

/**
 * List all jobs with optional filters
 * GET /api/v1/jobs
 */
export async function listJobs(
  params: ListJobsParams = {}
): Promise<JobsListResponse> {
  const queryParams: Record<string, string> = {};

  if (params.job_type) queryParams.job_type = params.job_type;
  if (params.status) queryParams.status = params.status;
  if (params.limit) queryParams.limit = params.limit.toString();
  if (params.offset) queryParams.offset = params.offset.toString();

  const response = await apiClient.get<JobsListResponse>(
    '/api/v1/jobs',
    queryParams
  );

  return response.data;
}

/**
 * List all organization sync statuses
 * GET /api/v1/sync-status
 */
export async function listSyncStatus(): Promise<SyncStatusListResponse> {
  const response = await apiClient.get<SyncStatusListResponse>(
    '/api/v1/sync-status'
  );

  return response.data;
}

/**
 * Get single organization sync status
 * GET /api/v1/sync-status/:organizationId
 */
export async function getSyncStatus(organizationId: string): Promise<SyncStatus> {
  const response = await apiClient.get<{ data: SyncStatus }>(
    `/api/v1/sync-status/${organizationId}`
  );

  return response.data.data;
}
