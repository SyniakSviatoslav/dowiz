// Thin typed fetch wrapper over the S1 storefront-read surface, mirroring the parity contract
// documented in REBUILD-MAP inventory 11 §6.1 (apps/web/src/lib/apiClient.ts + publicApi.ts):
// this is the "public bypass" lane only — no auth, no refresh, no token, server-side-safe.
//
// Base URL comes from an env var, never hardcoded (parity requirement — see task constraints).
import type { PublicMenu, PublicLocationInfo, PublicTheme, ErrorEnvelope } from './api-types';

export class ApiError extends Error {
  status: number;
  code: string;
  correlationId?: string;
  constructor(envelope: ErrorEnvelope, status: number) {
    super(envelope.message || envelope.error || 'Request failed');
    this.status = status;
    this.code = envelope.code;
    this.correlationId = envelope.correlationId;
  }
}

function apiBaseUrl(): string {
  // Astro env schema (astro.config.mjs `env.schema.PUBLIC_API_BASE_URL`) — no hardcoded host,
  // defaults to same-origin '/api' so local dev can proxy to the existing Node API unchanged.
  const fromEnv = (import.meta as unknown as { env?: Record<string, string | undefined> }).env
    ?.PUBLIC_API_BASE_URL;
  return fromEnv ?? '/api';
}

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(`${apiBaseUrl()}${path}`);
  if (!res.ok) {
    let envelope: ErrorEnvelope;
    try {
      envelope = (await res.json()) as ErrorEnvelope;
    } catch {
      envelope = {
        code: 'INTERNAL',
        message: res.statusText || 'Request failed',
        correlationId: '',
        status: res.status,
        error: res.statusText || 'Request failed',
      };
    }
    throw new ApiError(envelope, res.status);
  }
  return (await res.json()) as T;
}

/** GET /public/locations/{locationIdOrSlug}/menu?locale= — the hottest storefront read. */
export function getPublicMenu(slug: string, locale?: string): Promise<PublicMenu> {
  const qs = locale ? `?locale=${encodeURIComponent(locale)}` : '';
  return getJson<PublicMenu>(`/public/locations/${encodeURIComponent(slug)}/menu${qs}`);
}

/** GET /public/locations/{slug}/info — venue open/closed/busy + fee-mirror inputs. */
export function getPublicLocationInfo(slug: string): Promise<PublicLocationInfo> {
  return getJson<PublicLocationInfo>(`/public/locations/${encodeURIComponent(slug)}/info`);
}

/** GET /api/public/theme/{slug} — tenant branding for SSR CSS-var injection. */
export function getPublicTheme(slug: string): Promise<PublicTheme> {
  return getJson<PublicTheme>(`/api/public/theme/${encodeURIComponent(slug)}`);
}
