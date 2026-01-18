// Organizations API Client

import { apiClient } from '@/lib/api-client';
import type {
  Organization,
  OrganizationListItem,
  OrganizationResponse,
  OrganizationsListResponse,
} from '@/lib/types/organization';

export interface ListOrganizationsParams {
  page?: number;
  limit?: number;
  sort?: string;
  name_contains?: string;
}

/**
 * Get an organization by slug
 * GET /api/v1/organizations/:slug
 */
export async function getOrganization(slug: string): Promise<Organization> {
  const response = await apiClient.get<OrganizationResponse>(
    `/api/v1/organizations/${slug}`
  );
  return response.data.data;
}

/**
 * List all organizations with pagination
 * GET /api/v1/organizations
 */
export async function listOrganizations(
  params: ListOrganizationsParams = {}
): Promise<{
  data: OrganizationListItem[];
  total: number;
  pages: number;
  page: number;
}> {
  const queryParams: Record<string, string> = {};

  if (params.page) queryParams.page = params.page.toString();
  if (params.limit) queryParams.limit = params.limit.toString();
  if (params.sort) queryParams.sort = params.sort;
  if (params.name_contains) queryParams.name_contains = params.name_contains;

  const response = await apiClient.get<OrganizationsListResponse>(
    '/api/v1/organizations',
    queryParams
  );

  return {
    data: response.data.data,
    total: response.data.meta.pagination.total,
    pages: response.data.meta.pagination.pages,
    page: response.data.meta.pagination.page,
  };
}
