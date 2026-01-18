// Re-export all types
export * from './organization'; // Export organization first to avoid conflicts
export * from './search';
export * from './data-source';

// Common API types
export interface ApiResponse<T> {
  data: T;
  status: number;
  success: boolean;
}

export interface ApiError {
  message: string;
  code: string;
  status: number;
  details?: Record<string, any>;
}

export interface PaginatedResponse<T> {
  data: T[];
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

export interface QueryParams {
  [key: string]: string | number | boolean | undefined;
}
