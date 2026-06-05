import { z } from 'zod';

export interface ApiClientConfig {
  baseUrl: string;
  getToken?: () => string | null;
  getRefreshToken?: () => string | null;
  onRefresh?: () => Promise<string | null>;
  timeout?: number;
}

export interface RequestOpts {
  params?: Record<string, string>;
  signal?: AbortSignal;
  idempotencyKey?: string;
}

export class ApiClientError extends Error {
  constructor(
    public status: number,
    message: string,
    public body?: unknown,
  ) {
    super(message);
    this.name = 'ApiClientError';
  }
}

function buildUrl(base: string, path: string, params?: Record<string, string>): string {
  const effective = base || (typeof window !== 'undefined' ? window.location.origin : 'http://localhost:5173');
  const url = new URL(path, effective);
  if (params) {
    Object.entries(params).forEach(([k, v]) => url.searchParams.set(k, v));
  }
  return url.toString();
}

async function handleResponse(res: Response): Promise<unknown> {
  if (!res.ok) {
    let body: unknown;
    try {
      body = await res.json();
    } catch {
      body = null;
    }
    throw new ApiClientError(res.status, res.statusText, body);
  }
  if (res.status === 204 || res.headers.get('content-length') === '0') return null;
  return res.json();
}

export class ApiClient {
  private config: ApiClientConfig;

  constructor(config: ApiClientConfig) {
    this.config = { timeout: 15000, ...config };
  }

  private async request(
    method: string,
    path: string,
    body?: unknown,
    opts?: RequestOpts,
  ): Promise<unknown> {
    const token = this.config.getToken?.();
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    };
    if (token) headers['Authorization'] = `Bearer ${token}`;
    if (opts?.idempotencyKey) headers['Idempotency-Key'] = opts.idempotencyKey;

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);

    const signal = opts?.signal
      ? combineAbortSignals(controller.signal, opts.signal)
      : controller.signal;

    try {
      const res = await fetch(buildUrl(this.config.baseUrl, path, opts?.params), {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
        signal,
      });
      return await handleResponse(res);
    } catch (err) {
      if (err instanceof ApiClientError) throw err;
      if ((err as Error)?.name === 'AbortError') {
        throw new ApiClientError(0, 'Request timed out');
      }
      // Fallback phone on network/5xx errors
      const fallbackUrl = buildUrl(this.config.baseUrl.replace(/\/api/, '/phone-api'), path, opts?.params);
      try {
        const res = await fetch(fallbackUrl, {
          method,
          headers: {
            'Content-Type': 'application/json',
            Accept: 'application/json',
          },
          body: body ? JSON.stringify(body) : undefined,
        });
        return await handleResponse(res);
      } catch {
        throw new ApiClientError(0, 'Network error');
      }
    } finally {
      clearTimeout(timeoutId);
    }
  }

  async get<T = unknown>(path: string, opts?: RequestOpts): Promise<T> {
    return this.request('GET', path, undefined, opts) as Promise<T>;
  }

  async post<T = unknown>(path: string, body?: unknown, opts?: RequestOpts): Promise<T> {
    return this.request('POST', path, body, opts) as Promise<T>;
  }

  async patch<T = unknown>(path: string, body?: unknown, opts?: RequestOpts): Promise<T> {
    return this.request('PATCH', path, body, opts) as Promise<T>;
  }

  async put<T = unknown>(path: string, body?: unknown, opts?: RequestOpts): Promise<T> {
    return this.request('PUT', path, body, opts) as Promise<T>;
  }

  async delete<T = unknown>(path: string, opts?: RequestOpts): Promise<T> {
    return this.request('DELETE', path, undefined, opts) as Promise<T>;
  }
}

function combineAbortSignals(...signals: AbortSignal[]): AbortSignal {
  const controller = new AbortController();
  for (const sig of signals) {
    if (sig.aborted) {
      controller.abort(sig.reason);
      return controller.signal;
    }
    sig.addEventListener('abort', () => controller.abort(sig.reason), { once: true });
  }
  return controller.signal;
}
