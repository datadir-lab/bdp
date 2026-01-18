// Organization Types

export interface Organization {
  id: string;
  slug: string;
  name: string;
  website?: string;
  description?: string;
  logo_url?: string;
  is_system: boolean;
  entry_count?: number;
  statistics?: {
    data_sources: number;
    tools: number;
    total_versions: number;
    total_downloads: number;
  };
  created_at: string;
  updated_at?: string;
}

export interface OrganizationListItem {
  id: string;
  slug: string;
  name: string;
  description?: string;
  logo_url?: string;
  is_system: boolean;
  entry_count: number;
  created_at: string;
}

// API Response types
export interface OrganizationResponse {
  success: boolean;
  data: Organization;
}

export interface OrganizationsListResponse {
  success: boolean;
  data: OrganizationListItem[];
  meta: {
    pagination: {
      page: number;
      per_page: number;
      total: number;
      pages: number;
      has_next?: boolean;
      has_prev?: boolean;
    };
  };
}
