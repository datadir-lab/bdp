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
}

export interface SearchParams {
  query: string;
  type_filter?: string[];
  organism?: string;
  format?: string;
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

  if (params.organism) {
    queryParams.organism = params.organism;
  }

  if (params.format) {
    queryParams.format = params.format;
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
