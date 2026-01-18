import type {
  ApiResponse,
  ApiError,
  PaginatedResponse,
  QueryParams,
} from '@/lib/types';

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';

class ApiClient {
  private baseUrl: string;
  private defaultHeaders: HeadersInit;

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl;
    this.defaultHeaders = {
      'Content-Type': 'application/json',
    };
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<ApiResponse<T>> {
    const url = `${this.baseUrl}${endpoint}`;
    const config: RequestInit = {
      ...options,
      headers: {
        ...this.defaultHeaders,
        ...options.headers,
      },
    };

    try {
      const response = await fetch(url, config);
      const data = await response.json();

      if (!response.ok) {
        const error: ApiError = {
          message: data.message || 'An error occurred',
          code: data.code || 'UNKNOWN_ERROR',
          status: response.status,
          details: data.details,
        };
        throw error;
      }

      return {
        data,
        status: response.status,
        success: true,
      };
    } catch (error) {
      if ((error as ApiError).status) {
        throw error;
      }

      const apiError: ApiError = {
        message: error instanceof Error ? error.message : 'Network error',
        code: 'NETWORK_ERROR',
        status: 0,
      };
      throw apiError;
    }
  }

  async get<T>(endpoint: string, params?: QueryParams): Promise<ApiResponse<T>> {
    const queryString = params ? `?${new URLSearchParams(params as Record<string, string>).toString()}` : '';
    return this.request<T>(`${endpoint}${queryString}`, {
      method: 'GET',
    });
  }

  async post<T>(endpoint: string, data?: unknown): Promise<ApiResponse<T>> {
    return this.request<T>(endpoint, {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async put<T>(endpoint: string, data?: unknown): Promise<ApiResponse<T>> {
    return this.request<T>(endpoint, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  }

  async patch<T>(endpoint: string, data?: unknown): Promise<ApiResponse<T>> {
    return this.request<T>(endpoint, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
  }

  async delete<T>(endpoint: string): Promise<ApiResponse<T>> {
    return this.request<T>(endpoint, {
      method: 'DELETE',
    });
  }

  // Paginated requests
  async getPaginated<T>(
    endpoint: string,
    params?: QueryParams & { page?: number; limit?: number }
  ): Promise<ApiResponse<PaginatedResponse<T>>> {
    return this.get<PaginatedResponse<T>>(endpoint, params);
  }

  // Set authorization token
  setAuthToken(token: string) {
    this.defaultHeaders = {
      ...this.defaultHeaders,
      Authorization: `Bearer ${token}`,
    };
  }

  // Remove authorization token
  clearAuthToken() {
    const { Authorization, ...rest } = this.defaultHeaders as Record<string, string>;
    this.defaultHeaders = rest;
  }
}

// Export a singleton instance
export const apiClient = new ApiClient();

// Export the class for custom instances
export default ApiClient;
