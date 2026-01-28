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
    const isServer = typeof window === 'undefined';
    const url = `${this.baseUrl}${endpoint}`;

    const config: RequestInit = {
      ...options,
      headers: {
        ...this.defaultHeaders,
        ...options.headers,
      },
      // Disable caching completely
      cache: 'no-store',
      // Add next.js specific options
      ...(isServer ? { next: { revalidate: 0 } } : {}),
    };

    let lastError: unknown;

    for (let i = 0; i < retries; i++) {
      try {
        if (isServer) {
          console.log(`[SSR] Fetching: ${url}`);
        }

        const response = await fetch(url, config);

        if (isServer) {
          console.log(`[SSR] Response status: ${response.status}, Content-Type: ${response.headers.get('content-type')}`);
        }

        // Check if response is JSON
        const contentType = response.headers.get('content-type');
        if (!contentType || !contentType.includes('application/json')) {
          const text = await response.text();
          console.error(`Expected JSON response, got ${contentType || 'unknown'}. Status: ${response.status}, Body: ${text.substring(0, 200)}`);

          // Provide more helpful error messages based on status
          if (response.status === 404) {
            throw new Error(`Endpoint not found: ${url}`);
          } else if (response.status >= 500) {
            throw new Error(`Server error (${response.status}): The backend service may be unavailable`);
          } else {
            throw new Error(`API returned ${contentType || 'non-JSON'} response (status ${response.status})`);
          }
        }

        const data = await response.json();

        if (!response.ok) {
          const error: ApiError = {
            message: data.message || 'An error occurred',
            code: data.code || 'UNKNOWN_ERROR',
            status: response.status,
            details: data.details,
          };
          console.error(`API Error: ${error.status} ${error.code} - ${error.message}`);
          throw error;
        }

        return {
          data,
          status: response.status,
          success: true,
        };
      } catch (error) {
        lastError = error;

        console.error(`API request failed (attempt ${i + 1}/${retries}):`, error);

        // Don't retry on client errors (4xx) - these won't get better with retries
        if ((error as ApiError).status && (error as ApiError).status >= 400 && (error as ApiError).status < 500) {
          throw error;
        }

        // Don't retry on structured API errors with 5xx status (e.g., INTERNAL_ERROR)
        // These are application errors from the backend, not transient network issues
        if ((error as ApiError).status && (error as ApiError).status >= 500 && (error as ApiError).code) {
          throw error;
        }

        // Retry on network errors and generic 5xx server errors
        if (i < retries - 1) {
          // Exponential backoff: 1s, 2s, 3s
          const delay = 1000 * (i + 1);
          console.log(`Retrying API request in ${delay}ms (${i + 1}/${retries}): ${url}`);
          await new Promise(resolve => setTimeout(resolve, delay));
          continue;
        }
      }
    }

    // All retries failed - preserve original error if it's an ApiError
    console.error(`All retries failed for ${url}:`, lastError);
    if ((lastError as ApiError).code && (lastError as ApiError).status) {
      throw lastError;
    }
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
