import { test, expect, type APIRequestContext } from '@playwright/test';

// Proof for ADR-0004 owner-token-revocation (P-b logout route + P-d no-regression on staging).
// NOTE: staging login takes the dev-bypass path (signDevToken, 7d, NO refresh token), so P-a (24h
// argon2 access) and P-c (refresh re-derivation) are exercised by unit/guardrail + the grep
// guardrail (scripts/guardrail-owner-active-membership.mjs), not observable via this login.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test owner-revocation --project=desktop --reporter=list
const CREDS = { email: 'test@dowiz.com', password: 'test123456' };

async function token(request: APIRequestContext): Promise<string> {
  const res = await request.post('/api/auth/local/login', { data: CREDS });
  expect(res.ok(), 'owner login').toBeTruthy();
  return (await res.json()).access_token as string;
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
  const body = await t.json();
  const loc = body.activeLocationId;
  expect(loc, 'login returns activeLocationId').toBeTruthy();
  // A requireLocationAccess-gated owner endpoint must still serve the active owner (P-d adds
  // status='active' — must NOT lock out a legitimate active owner). 200 (or 304); never 404/403.
  const res = await request.get(`/api/owner/locations/${loc}/notifications/status`, {
    headers: { authorization: `Bearer ${body.access_token}` },
  });
  expect([200, 304], `active owner allowed (got ${res.status()})`).toContain(res.status());
});
