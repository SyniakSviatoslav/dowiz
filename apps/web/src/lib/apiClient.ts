import { z } from 'zod';

const API_BASE = import.meta.env?.VITE_API_BASE_URL || '/api';

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
    public data?: any
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

interface ApiClientOptions<T extends z.ZodType> {
  method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  body?: any;
  schema?: T;
  idempotencyKey?: string;
  timeout?: number;
  headers?: Record<string, string>;
}

export const apiClient = async <T extends z.ZodType>(
  endpoint: string,
  options: ApiClientOptions<T> = {}
): Promise<z.infer<T>> => {
  const {
    method = 'GET',
    body,
    schema,
    idempotencyKey,
    timeout = 10000,
    headers: customHeaders = {}
  } = options;

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  // TODO: integrate real useAuth hook later
  // Temporary auth token retrieval from localStorage (mock)
  const token = typeof window !== 'undefined' ? localStorage.getItem('dos_access_token') : null;

  const isFormData = body instanceof FormData;

  const headers: Record<string, string> = {
    ...customHeaders,
  };

  if (!isFormData) {
    headers['Content-Type'] = 'application/json';
  }
  headers['Accept'] = 'application/json';

  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  if (idempotencyKey && ['POST', 'PUT', 'PATCH'].includes(method)) {
    headers['X-Idempotency-Key'] = idempotencyKey;
  }

  try {
    const response = await fetch(`${API_BASE}${endpoint}`, {
      method,
      headers,
      body: body ? (isFormData ? body : JSON.stringify(body)) : undefined,
      signal: controller.signal,
    });

    clearTimeout(timeoutId);

    if (!response.ok) {
      let errorData;
      try {
        errorData = await response.json();
      } catch (err) {
        console.debug('[apiClient] failed to parse error response as JSON:', err);
      }

      // Status -> Action mapping (G2)
      switch (response.status) {
        case 401:
          if (typeof window !== 'undefined') {
            localStorage.removeItem('dos_access_token');
            sessionStorage.setItem('dos_auth_expired', '1');
            window.location.href = '/admin';
          }
          break;
        case 403:
          break;
        case 404:
          break;
        case 422:
          break;
        case 429:
          break;
        default:
          if (response.status >= 500) {
            // 5xx Server Error - Show fallback phone, keep cart intact
          }
      }

      throw new ApiError(response.status, errorData?.error || response.statusText, errorData);
    }

    if (response.status === 204) {
      return undefined as z.infer<T>;
    }

    const json = await response.json();

    if (schema) {
      return schema.parse(json);
    }

    return json;
  } catch (error: any) {
    clearTimeout(timeoutId);
    if (error.name === 'AbortError') {
      throw new ApiError(408, 'Request Timeout');
    }
    throw error;
  }
};
