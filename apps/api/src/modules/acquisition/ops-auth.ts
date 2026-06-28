import crypto from 'node:crypto';

// P6-2 — internal ops-auth for the acquisition/provisioning surface (breaker B4).
//
// DECOUPLED from the dev-login family (ALLOW_DEV_LOGIN / DEV_AUTH_SECRET / the /api/dev dev-guard)
// on purpose: enabling shadow-provisioning in prod must NOT force re-arming the mock-auth owner-JWT
// minter (the CRITICAL backdoor, ADR-0003). This surface mounts OUTSIDE /api/dev (so the global
// dev-guard never applies) and is gated solely by its OWN secret. The secret is read from
// process.env at registration (not the @deliveryos/config red-line schema), so it composes without
// touching the dev-bypass prod-offenders guard.
//
// Fail-CLOSED: when PROVISION_OPS_SECRET is unset/empty the whole surface returns 404 (existence
// hidden), the safe default on any box that hasn't explicitly opted in.

const HEADER = 'x-provision-ops-secret';

function secretMatches(provided: unknown, expected: string): boolean {
  if (typeof provided !== 'string' || provided.length === 0) return false;
  const a = Buffer.from(provided);
  const b = Buffer.from(expected);
  if (a.length !== b.length) return false;
  return crypto.timingSafeEqual(a, b);
}

/** True iff provisioning ops are enabled (secret configured) AND the request presents it. */
export function provisionOpsAuthorized(providedSecret: unknown, secret: string | undefined): boolean {
  if (!secret) return false; // disabled / fail-closed
  return secretMatches(providedSecret, secret);
}

export const PROVISION_OPS_HEADER = HEADER;
