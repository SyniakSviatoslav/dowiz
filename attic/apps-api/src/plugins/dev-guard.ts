import crypto from 'node:crypto';

// Test-only endpoints (mock-auth, create-assignment, seed-data) live under these
// prefixes. They mint real JWTs / mutate data, so they MUST never be reachable
// anonymously. A single guard keyed on the path closes all of them — present and
// future — in one place.
const DEV_PREFIXES = ['/dev/', '/api/dev/'];

export function isDevPath(url: string): boolean {
  const path = url.split('?')[0] ?? url;
  return DEV_PREFIXES.some((p) => path.startsWith(p));
}

/**
 * The env knobs that gate every dev/test auth bypass. A structural subset of the
 * full Env so this module stays decoupled from @deliveryos/config.
 */
export interface DevAuthEnv {
  ALLOW_DEV_LOGIN: 'true' | 'false' | string;
  DEV_AUTH_SECRET?: string;
}

/**
 * Whether dev/test auth bypasses (seeded accounts, mock-auth) are permitted.
 * Requires BOTH the explicit ALLOW_DEV_LOGIN flag AND a configured DEV_AUTH_SECRET
 * (ADR-0003). The secret alone is no longer sufficient — so production, which sets
 * neither (and whose boot-guard D rejects either), can never honor a dev bypass even
 * if the secret leaks again. This is the single source of truth for all six mint sites.
 */
export function devLoginAllowed(env: DevAuthEnv): boolean {
  return env.ALLOW_DEV_LOGIN === 'true' && !!env.DEV_AUTH_SECRET;
}

/** Constant-time comparison; false on any type/length mismatch (never throws). */
function secretMatches(provided: unknown, expected: string): boolean {
  if (typeof provided !== 'string' || provided.length === 0) return false;
  const a = Buffer.from(provided);
  const b = Buffer.from(expected);
  if (a.length !== b.length) return false;
  return crypto.timingSafeEqual(a, b);
}

/**
 * Authorize a request to a /dev or /api/dev endpoint.
 *
 * - Non-dev paths are always authorized (pass-through — this guard is a no-op for them).
 * - Fails CLOSED unless the dev bypass is fully enabled (ALLOW_DEV_LOGIN flag AND a
 *   configured DEV_AUTH_SECRET — ADR-0003). The same flag that gates devLoginAllowed
 *   gates the /dev/* family here, so the mock-auth minters (which ride this guard, not
 *   devLoginAllowed) are closed on prod too — closing the gap that left them exploitable.
 * - When enabled, the caller must additionally present the matching secret via the
 *   `x-dev-auth-secret` header.
 */
export function isDevRequestAuthorized(
  url: string,
  providedSecret: unknown,
  env: DevAuthEnv,
): boolean {
  if (!isDevPath(url)) return true;
  if (!devLoginAllowed(env)) return false;
  return secretMatches(providedSecret, env.DEV_AUTH_SECRET as string);
}
