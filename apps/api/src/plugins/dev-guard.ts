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
 * Whether the dev-only password bypass (seeded test accounts) is permitted.
 * True only when DEV_AUTH_SECRET is configured, so production — which sets no
 * secret — never honors the hardcoded dev credentials.
 */
export function devLoginAllowed(configuredSecret: string | undefined): boolean {
  return !!configuredSecret;
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
 * - Fails CLOSED: if DEV_AUTH_SECRET is unset/empty, every dev request is rejected,
 *   so production (which sets no secret) has the dev endpoints fully disabled.
 * - Otherwise the caller must present the matching secret via the
 *   `x-dev-auth-secret` header.
 */
export function isDevRequestAuthorized(
  url: string,
  providedSecret: unknown,
  configuredSecret: string | undefined,
): boolean {
  if (!isDevPath(url)) return true;
  if (!configuredSecret) return false;
  return secretMatches(providedSecret, configuredSecret);
}
