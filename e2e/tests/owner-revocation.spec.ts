import { test, expect, type APIRequestContext } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// Proof for ADR-0004 owner-token-revocation (P-b logout route + P-d no-regression on staging).
// NOTE: staging login takes the dev-bypass path (signDevToken, 7d, NO refresh token), so P-a (24h
// argon2 access) and P-c (refresh re-derivation) are exercised by unit/guardrail + the grep
// guardrail (scripts/guardrail-owner-active-membership.mjs), not observable via this login.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test owner-revocation --project=desktop --reporter=list
//
// TODO(needs_staging): cross-tenant IDOR — drive owner-A's bearer token against a location owned by
//   a REAL second tenant (E2E_TENANT_B_LOCATION_ID) and assert 403/404. Cannot be added without a
//   real second-tenant fixture (a nil-UUID would 404 by absence and prove nothing — AGENTS.md §5).
// TODO(needs_staging): post-logout access-token rejection — per ADR-0004 the stateless access token
//   stays valid ≤24h after logout (only refresh families are revoked), so immediate rejection is NOT
//   the contract and cannot be asserted on the dev-bypass login (no observable refresh token here).
const CREDS = {
  email: process.env.E2E_OWNER_EMAIL ?? 'test@dowiz.com',
  password: process.env.E2E_OWNER_PASSWORD ?? 'test123456',
};

// These specs MUTATE auth state (logout revokes refresh families) — fail fast against prod/unknown.
test.beforeAll(() => requireStaging(process.env.VITE_BASE_URL));

async function token(request: APIRequestContext): Promise<string> {
  const res = await request.post('/api/auth/local/login', { data: CREDS });
  expect(res.status(), 'owner login').toBe(200);
  const accessToken = (await res.json()).access_token;
  expectJwt(accessToken, 'access_token');
  return accessToken as string;
}

test('P-b: POST /api/auth/logout is wired + authenticated (was 404)', async ({ request }) => {
  // No bearer → 401 from verifyAuth (NOT 404 — the route now exists).
  const noauth = await request.post('/api/auth/logout', {});
  expect(noauth.status(), 'logout requires auth').toBe(401);
});

test('P-b: authenticated logout succeeds (revokes refresh families, 204)', async ({ request }) => {
  const t = await token(request);
  const res = await request.post('/api/auth/logout', { headers: { authorization: `Bearer ${t}` } });
  expect(res.status(), 'authenticated logout').toBe(204);
});

test('P-d no-regression: an ACTIVE owner still resolves its location (owner endpoint reachable)', async ({ request }) => {
  const t = await request.post('/api/auth/local/login', { data: CREDS });
  expect(t.status(), 'login').toBe(200);
  const body = await t.json();
  const loc = body.activeLocationId;
  expectUuid(loc, 'activeLocationId');
  expectJwt(body.access_token, 'access_token');
  // A requireLocationAccess-gated owner endpoint must still serve the active owner (P-d adds
  // status='active' — must NOT lock out a legitimate active owner). Route always replies 200.
  const res = await request.get(`/api/owner/locations/${loc}/notifications/status`, {
    headers: { authorization: `Bearer ${body.access_token}` },
  });
  expect(res.status(), `active owner allowed (got ${res.status()})`).toBe(200);
});
