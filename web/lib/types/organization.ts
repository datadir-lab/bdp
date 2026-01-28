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

  // Licensing and citation
  license?: string; // e.g., "CC-BY-4.0", "MIT", "Custom"
  license_url?: string; // Link to full license text
  citation?: string; // How to cite this organization's data
  citation_url?: string; // Link to citation guidelines

  // Versioning strategy (for BDP)
  version_strategy?: string; // e.g., "semantic", "date-based", "release-based"
  version_description?: string; // Description of how versions are managed

  // Additional metadata
  data_source_url?: string; // Link to the original data source
  documentation_url?: string; // Link to documentation
  contact_email?: string; // Contact for questions

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
