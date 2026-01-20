// Data Sources API Client

import { apiClient } from '@/lib/api-client';
import type {
  DataSource,
  DataSourceVersion,
  DependenciesResponse,
  DataSourceResponse,
  DataSourceVersionResponse,
  DependenciesApiResponse,
} from '@/lib/types/data-source';

export interface ListDataSourcesParams {
  org?: string;
  type?: string;
  organism?: string;
  page?: number;
  limit?: number;
  sort?: string;
}

export interface GetDependenciesParams {
  format?: string;
  page?: number;
  limit?: number;
  search?: string;
}

/**
 * Get a data source with all versions
 * GET /api/v1/data-sources/:org/:name
 */
export async function getDataSource(
  org: string,
  name: string
): Promise<DataSource> {
  const response = await apiClient.get<DataSourceResponse>(
    `/api/v1/data-sources/${org}/${name}`
  );
  return response.data.data;
}

/**
 * Get a specific version of a data source
 * GET /api/v1/data-sources/:org/:name/:version
 */
export async function getDataSourceVersion(
  org: string,
  name: string,
  version: string
): Promise<DataSourceVersion & { organization: string; name: string }> {
  const response = await apiClient.get<DataSourceVersionResponse>(
    `/api/v1/data-sources/${org}/${name}/${version}`
  );
  return response.data.data;
}

/**
 * Get dependencies for a data source version
 * GET /api/v1/data-sources/:org/:name/:version/dependencies
 */
export async function getDependencies(
  org: string,
  name: string,
  version: string,
  params: GetDependenciesParams = {}
): Promise<DependenciesResponse> {
  const queryParams: Record<string, string> = {};

  if (params.format) queryParams.format = params.format;
  if (params.page) queryParams.page = params.page.toString();
  if (params.limit) queryParams.limit = params.limit.toString();
  if (params.search) queryParams.search = params.search;

  const response = await apiClient.get<DependenciesApiResponse>(
    `/api/v1/data-sources/${org}/${name}/${version}/dependencies`,
    queryParams
  );
  return response.data.data;
}

/**
 * List data sources with filters
 * GET /api/v1/data-sources
 */
export async function listDataSources(
  params: ListDataSourcesParams = {}
): Promise<{ data: DataSource[]; total: number; pages: number }> {
  const queryParams: Record<string, string> = {};

  if (params.org) queryParams.org = params.org;
  if (params.type) queryParams.type = params.type;
  if (params.organism) queryParams.organism = params.organism;
  if (params.page) queryParams.page = params.page.toString();
  if (params.limit) queryParams.limit = params.limit.toString();
  if (params.sort) queryParams.sort = params.sort;

  const response = await apiClient.get<{
    success: boolean;
    data: DataSource[];
    meta: {
      pagination: {
        total: number;
        pages: number;
        page: number;
        per_page: number;
      };
    };
  }>('/api/v1/data-sources', queryParams);

  return {
    data: response.data.data,
    total: response.data.meta.pagination.total,
    pages: response.data.meta.pagination.pages,
  };
}
