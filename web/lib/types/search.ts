export interface SearchFilters {
  types?: string[]; // 'datasource' | 'tool'
  source_types?: string[]; // 'protein', 'genome', 'organism', etc.
  organizations?: string[]; // Organization slugs
  formats?: string[]; // File formats like 'fasta', 'json', 'gtf', etc.
  dateRange?: {
    from?: Date;
    to?: Date;
  };
  tags?: string[];
}

export interface SearchSuggestion {
  id: string;
  organization_slug: string;
  slug: string;
  name: string;
  entry_type: 'data_source' | 'tool' | 'organization';
  source_type?: string;
  latest_version?: string;
  match_score: number;
}

export interface SearchResult {
  id: string;
  organization_slug: string;
  slug: string;
  name: string;
  description?: string;
  entry_type: string;
  source_type?: string;
  tool_type?: string;
  organism?: {
    scientific_name: string;
    common_name?: string;
    ncbi_taxonomy_id?: number;
  };
  latest_version?: string;
  external_version?: string;
  available_formats: string[];
  total_downloads: number;
  external_id?: string;
  rank: number;
}

export interface SearchPagination {
  page: number;
  per_page: number;
  total: number;
  pages: number;
  has_next: boolean;
  has_prev: boolean;
}
