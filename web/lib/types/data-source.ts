// Data Source Types
import type { Organization } from './organization';

export interface Organism {
  ncbi_taxonomy_id: number;
  scientific_name: string;
  common_name?: string;
  rank?: string;
  taxonomy_organization_slug?: string;
  taxonomy_slug?: string;
  taxonomy_version?: string;
}

export interface ProteinComment {
  topic: string;
  text: string;
}

export interface ProteinFeature {
  feature_type: string;
  description?: string;
  start_pos?: number;
  end_pos?: number;
}

export interface ProteinCrossReference {
  database: string;
  database_id: string;
  metadata?: Record<string, unknown>;
}

export interface ProteinPublication {
  reference_number: number;
  position?: string;
  comments: string[];
  pubmed_id?: string;
  doi?: string;
  author_group?: string;
  authors: string[];
  title?: string;
  location?: string;
}

export interface ProteinMetadata {
  accession: string;
  entry_name?: string;
  protein_name?: string;
  gene_name?: string;
  sequence_length?: number;
  mass_da?: number;
  sequence_checksum?: string;
  alternative_names?: string[];
  ec_numbers?: string[];
  protein_existence?: number;
  keywords?: string[];
  organelle?: string;
  entry_created?: string;
  sequence_updated?: string;
  annotation_updated?: string;
}

export interface VersionFile {
  id: string;
  format: string; // 'fasta', 'xml', 'json', 'dat', etc.
  checksum: string;
  size_bytes: number;
  compression?: string; // 'gzip', 'bzip2', 'none'
  s3_key?: string;
  created_at: string;
}

export interface Citation {
  id: string;
  citation_type: string; // 'primary', 'method', 'review'
  doi?: string;
  pubmed_id?: string;
  title: string;
  journal?: string;
  publication_date?: string;
  authors?: string;
  url?: string;
}

export interface DataSourceVersion {
  id: string;
  version: string; // Internal semantic version: '1.0', '1.5'
  external_version?: string; // External version: '2025_01', 'v2.14.0'
  release_date?: string;
  size_bytes?: number;
  download_count: number;
  files: VersionFile[];
  citations?: Citation[];
  has_dependencies: boolean;
  dependency_count: number;
  published_at: string;
  updated_at?: string;
  additional_metadata?: Record<string, unknown>;
}

// List item type - matches backend list API response (flat structure)
export interface DataSourceListItem {
  id: string;
  organization_id: string;
  organization_slug: string;
  slug: string;
  name: string;
  source_type: string;
  external_id?: string;
  organism_scientific_name?: string;
  latest_version?: string;
  total_downloads: number;
  created_at: string;
}

// Full data source type - matches backend detail API response (nested structure)
export interface DataSource {
  id: string;
  organization: Organization;
  slug: string;
  name: string;
  description?: string;
  source_type: string; // 'protein', 'genome', 'annotation', 'structure', 'other'
  external_id?: string;
  organism?: Organism;
  protein_metadata?: ProteinMetadata;
  versions: DataSourceVersion[];
  latest_version?: string;
  total_downloads: number;
  tags?: string[];
  created_at: string;
  updated_at: string;
}

export interface Dependency {
  id: string;
  organization_slug: string;
  entry_slug: string;
  entry_name: string;
  entry_type: string;
  required_version: string;
  dependency_type: string;
}

export interface DependenciesResponse {
  source: string;
  format?: string;
  dependency_count: number;
  tree_checksum?: string;
  dependencies: Dependency[];
  pagination: {
    page: number;
    per_page: number;
    total: number;
    pages: number;
    has_next: boolean;
    has_prev: boolean;
  };
}

// API Response types
export interface DataSourceResponse {
  success: boolean;
  data: DataSource;
}

export interface DataSourceVersionResponse {
  success: boolean;
  data: DataSourceVersion & {
    organization: string;
    name: string;
  };
}

export interface DependenciesApiResponse {
  success: boolean;
  data: DependenciesResponse;
}
