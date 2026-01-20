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
    options: RequestInit = {},
    retries = 3
  ): Promise<ApiResponse<T>> {
    const url = `${this.baseUrl}${endpoint}`;
    const config: RequestInit = {
      ...options,
      headers: {
        ...this.defaultHeaders,
        ...options.headers,
      },
    };

    let lastError: any;

    for (let i = 0; i < retries; i++) {
      try {
        const response = await fetch(url, config);

        // Check if response is JSON
        const contentType = response.headers.get('content-type');
        if (!contentType || !contentType.includes('application/json')) {
          throw new Error(`Expected JSON response, got ${contentType || 'unknown'}`);
        }

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
        lastError = error;

        // Don't retry on client errors (4xx) - these won't get better with retries
        if ((error as ApiError).status && (error as ApiError).status >= 400 && (error as ApiError).status < 500) {
          throw error;
        }

        // Retry on network errors and 5xx server errors
        if (i < retries - 1) {
          // Exponential backoff: 1s, 2s, 3s
          await new Promise(resolve => setTimeout(resolve, 1000 * (i + 1)));
          console.log(`Retrying API request (${i + 1}/${retries}): ${url}`);
          continue;
        }
      }
    }

    // All retries failed
    const apiError: ApiError = {
      message: lastError instanceof Error ? lastError.message : 'Network error after retries',
      code: 'NETWORK_ERROR',
      status: 0,
    };
    throw apiError;
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
