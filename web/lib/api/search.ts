// Search API Client

import { apiClient } from '@/lib/api-client';
import type {
  SearchSuggestion,
  SearchResult,
  SearchPagination,
} from '@/lib/types/search';

export interface SearchSuggestionsParams {
  q: string;
  limit?: number;
  type_filter?: string[];
  source_type_filter?: string[];
}

export interface SearchParams {
  query: string;
  type_filter?: string[];
  source_type_filter?: string[];
  organism?: string;
  format?: string;
  version?: string;
  page?: number;
  per_page?: number;
}

/**
 * Get autocomplete suggestions for search
 * GET /api/v1/search/suggestions
 */
export async function getSuggestions(
  params: SearchSuggestionsParams
): Promise<SearchSuggestion[]> {
  const queryParams: Record<string, string> = {
    q: params.q,
  };

  if (params.limit) {
    queryParams.limit = params.limit.toString();
  }

  if (params.type_filter && params.type_filter.length > 0) {
    queryParams.type_filter = params.type_filter.join(',');
  }

  if (params.source_type_filter && params.source_type_filter.length > 0) {
    queryParams.source_type_filter = params.source_type_filter.join(',');
  }

  const response = await apiClient.get<{
    success: boolean;
    data: SearchSuggestion[];
  }>('/api/v1/search/suggestions', queryParams);

  return response.data.data;
}

/**
 * Perform full-text search with filters and pagination
 * GET /api/v1/search
 */
export async function searchFullText(params: SearchParams): Promise<{
  items: SearchResult[];
  pagination: SearchPagination;
}> {
  const queryParams: Record<string, string> = {
    query: params.query,
  };

  if (params.type_filter && params.type_filter.length > 0) {
    queryParams.type_filter = params.type_filter.join(',');
  }

  if (params.source_type_filter && params.source_type_filter.length > 0) {
    queryParams.source_type_filter = params.source_type_filter.join(',');
  }

  if (params.organism) {
    queryParams.organism = params.organism;
  }

  if (params.format) {
    queryParams.format = params.format;
  }

  if (params.version) {
    queryParams.version = params.version;
  }

  if (params.page) {
    queryParams.page = params.page.toString();
  }

  if (params.per_page) {
    queryParams.per_page = params.per_page.toString();
  }

  const response = await apiClient.get<{
    success: boolean;
    data: SearchResult[];
    meta: {
      pagination: SearchPagination;
    };
  }>('/api/v1/search', queryParams);

  return {
    items: response.data.data,
    pagination: response.data.meta.pagination,
  };
}

/**
 * Get available source types for data sources
 * GET /api/v1/data-sources/source-types
 */
export async function getAvailableSourceTypes(): Promise<string[]> {
  // Fallback list
  const fallbackTypes = [
    'annotation',
    'bundle',
    'genome',
    'organism',
    'other',
    'pathway',
    'protein',
    'structure',
    'taxonomy',
    'transcript',
  ];

  try {
    const response = await apiClient.get<{
      success: boolean;
      data: string[];
    }>('/api/v1/data-sources/source-types');

    // Validate response
    if (response.data.data && Array.isArray(response.data.data) && response.data.data.length > 0) {
      return response.data.data.sort();
    }

    console.warn('API returned empty or invalid source types, using fallback');
    return fallbackTypes;
  } catch (error) {
    console.warn('Failed to fetch source types from API, using fallback:', error);
    return fallbackTypes;
  }
}
