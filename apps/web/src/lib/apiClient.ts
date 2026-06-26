import { safeStorage } from './safeStorage.js';
import { z } from 'zod';

const API_BASE = import.meta.env?.VITE_API_BASE_URL || '/api';

// ---- Transparent access-token refresh ---------------------------------------------------
// The owner session uses a short-lived access token (1h) + a rotating refresh token (7d).
// On a 401 we silently refresh and retry ONCE before bouncing to login — so the session
// rolls forward instead of expiring "too soon". Refresh is single-flight in-tab (one shared
// promise) AND cross-tab (Web Locks), so concurrent 401s never present the same refresh token
// twice — which would trip the server's reuse-detection and revoke the whole family.
let inflightRefresh: Promise<string | null> | null = null;

// Best-effort decode of the (possibly-expired) access token's activeLocationId, so a refresh
// preserves a multi-location owner's working tenant instead of letting the server re-pick (P-c).
function currentActiveLocationId(): string | undefined {
  try {
    const part = safeStorage.get('dos_access_token')?.split('.')[1];
    if (!part) return undefined;
    const payload = JSON.parse(atob(part.replace(/-/g, '+').replace(/_/g, '/')));
    return typeof payload?.activeLocationId === 'string' ? payload.activeLocationId : undefined;
  } catch { return undefined; }
}

async function doRefresh(): Promise<string | null> {
  const refreshToken = typeof window !== 'undefined' ? safeStorage.get('dos_refresh_token') : null;
  if (!refreshToken) return null;
  let res: Response;
  try {
    const activeLocationId = currentActiveLocationId();
    res = await fetch(`${API_BASE}/auth/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: JSON.stringify(activeLocationId ? { refresh_token: refreshToken, active_location_id: activeLocationId } : { refresh_token: refreshToken }),
    });
  } catch {
    return null;
  }
  if (res.status === 409) {
    // A concurrent request already rotated this family; the winner stored the new token.
    return safeStorage.get('dos_access_token') || null;
  }
  if (!res.ok) return null;
  const data = await res.json().catch(() => null);
  if (!data?.access_token) return null;
  safeStorage.set('dos_access_token', data.access_token);
  try { sessionStorage.setItem('dos_access_token', data.access_token); } catch { /* private mode */ }
  if (data.refresh_token) safeStorage.set('dos_refresh_token', data.refresh_token);
  return data.access_token;
}

export async function refreshAccessToken(): Promise<string | null> {
  if (inflightRefresh) return inflightRefresh;
  inflightRefresh = (async () => {
    try {
      const locks = typeof navigator !== 'undefined' ? (navigator as any).locks : undefined;
      // Web Lock serialises refresh across tabs; doRefresh re-reads the stored token INSIDE
      // the lock so a tab that lost the race picks up the winner's freshly-rotated token.
      if (locks?.request) return await locks.request('dos-token-refresh', doRefresh);
      return await doRefresh();
    } finally {
      inflightRefresh = null;
    }
  })();
  return inflightRefresh;
}

// Guards the dead-session bounce so it fires at most once per page session: without this,
// many in-flight admin requests can each 401 and stack redirects, re-triggering the
// settings-refetch loop and tripping the rate limiter. Reset on a full page load.
let authRedirectInFlight = false;

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
    public data?: any
  ) {
    super(message);
    this.name = 'ApiError';
  }

  /**
   * Stable SCREAMING_SNAKE machine code from the A1 error envelope (ADR-0010), e.g.
   * MIN_ORDER_NOT_MET / CASH_AMOUNT_TOO_LOW. Undefined for legacy/non-JSON responses.
   * (The A2 `mapApiError` matrix will branch on this; do not branch on `message`.)
   */
  get code(): string | undefined {
    return typeof this.data?.code === 'string' ? this.data.code : undefined;
  }

  /** Server-generated correlation id — show as the on-screen "support code" so an
   *  operator can grep it straight to Pino/Sentry. */
  get correlationId(): string | undefined {
    return typeof this.data?.correlationId === 'string' ? this.data.correlationId : undefined;
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

  const isFormData = body instanceof FormData;

  const buildHeaders = (accessToken: string | null): Record<string, string> => {
    const headers: Record<string, string> = { ...customHeaders };
    if (!isFormData) headers['Content-Type'] = 'application/json';
    headers['Accept'] = 'application/json';
    if (accessToken) headers['Authorization'] = `Bearer ${accessToken}`;
    if (idempotencyKey && ['POST', 'PUT', 'PATCH'].includes(method)) {
      headers['X-Idempotency-Key'] = idempotencyKey;
    }
    return headers;
  };

  const send = (accessToken: string | null) =>
    fetch(`${API_BASE}${endpoint}`, {
      method,
      headers: buildHeaders(accessToken),
      body: body ? (isFormData ? body : JSON.stringify(body)) : undefined,
      signal: controller.signal,
    });

  try {
    const token = typeof window !== 'undefined' ? safeStorage.get('dos_access_token') : null;
    let response = await send(token);

    // Transparent refresh: on 401, silently rotate the access token once and retry the
    // request before surfacing the error / bouncing to login. Keeps owner sessions alive
    // across the short 1h access-token window.
    if (
      response.status === 401 &&
      typeof window !== 'undefined' &&
      safeStorage.get('dos_refresh_token')
    ) {
      const newToken = await refreshAccessToken();
      if (newToken) response = await send(newToken);
    }

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
          // Owner-session-expiry handling is scoped to the owner app. A 401 on a
          // customer surface (/s/:slug/...) or the courier app must NOT bounce the
          // visitor to the owner login — the page handles its own missing/expired
          // session (e.g. OrderStatusPage shows a "reload the menu" message).
          // Reached only after a refresh attempt already failed above — the session is
          // genuinely dead, so clear both tokens before bouncing to login.
          // Bounce to /login (NOT /admin): /admin re-fetches /owner/settings which re-401s,
          // looping ~45× until the rate limiter trips. /login is a public page that does no
          // authed fetch, breaking the cycle. The module-level guard ensures the redirect
          // fires at most once per page session even if several requests 401 in parallel.
          if (
            typeof window !== 'undefined' &&
            window.location.pathname.startsWith('/admin') &&
            !authRedirectInFlight
          ) {
            authRedirectInFlight = true;
            safeStorage.remove('dos_access_token');
            safeStorage.remove('dos_refresh_token');
            sessionStorage.setItem('dos_auth_expired', '1');
            window.location.href = '/login';
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

      // Prefer the A1 envelope `message`; fall back to legacy `error`, then statusText.
      throw new ApiError(response.status, errorData?.message || errorData?.error || response.statusText, errorData);
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
