import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';

// PROD-SAFE smoke for the deployed batch (ADR-0004 + audit/owner fixes). NON-MUTATING — no order
// create, no import. Prod uses the argon2 login path (dev-bypass off), so the 24h access TTL is
// observable here (it wasn't on staging's dev-bypass).
// Run: VITE_BASE_URL=https://dowiz.fly.dev pnpm exec playwright test prod-adr0004-smoke --project=desktop --reporter=list
// Credentials come from env (TEST_OWNER_EMAIL / TEST_OWNER_PASSWORD) — never hardcode real creds in source.
const CREDS = {
  email: process.env.TEST_OWNER_EMAIL ?? '',
  password: process.env.TEST_OWNER_PASSWORD ?? '',
};
// Fail explicitly (not silently happy-path) when the owner creds are not provisioned.
function requireCreds(): void {
  expect(CREDS.email, 'TEST_OWNER_EMAIL must be set').not.toBe('');
  expect(CREDS.password, 'TEST_OWNER_PASSWORD must be set').not.toBe('');
}

test('prod up + app shell serves', async ({ request }) => {
  // Env-agnostic liveness (slug data differs per env): the app root must serve.
  const res = await request.get('/');
  expect([200, 304], `prod root (${res.status()})`).toContain(res.status());
});

test('security: Google OAuth backend gated on prod (404)', async ({ request }) => {
  const res = await request.get('/api/auth/google', { maxRedirects: 0 });
  expect(res.status(), 'GET /api/auth/google').toBe(404);
});

test('P-b: /api/auth/logout wired + authenticated on prod (was 404)', async ({ request }) => {
  const res = await request.post('/api/auth/logout', {});
  expect(res.status(), 'logout requires auth, not 404').toBe(401);
});

test('P-a: prod argon2 login mints a ~24h access token (was 7d)', async ({ request }) => {
  requireCreds();
  const res = await request.post('/api/auth/local/login', { data: CREDS });
  expect(res.status(), `owner login (${res.status()})`).toBe(200);
  const body = await res.json();
  const tok = body.access_token as string;
  expectJwt(tok, 'access_token');
  const p = JSON.parse(Buffer.from(tok.split('.')[1], 'base64url').toString('utf8'));
  // ADR-0004 core claims: role-derived owner identity carried in the token.
  expect(p.role, 'token role').toBe('owner');
  expectUuid(p.sub, 'token sub');
  const ttl = p.exp - p.iat;
  // 24h = 86400s. Assert ~24h — lower bound 23h catches a regression to 12h/1h; upper bound rejects 7d/48h.
  expect(ttl, `access TTL ${ttl}s ~24h`).toBeGreaterThanOrEqual(23 * 3600);
  expect(ttl, `access TTL ${ttl}s ~24h`).toBeLessThanOrEqual(24 * 3600 + 120);
});

test('P-a-neg: wrong password is rejected with 401 + no token', async ({ request }) => {
  requireCreds();
  const res = await request.post('/api/auth/local/login', {
    data: { email: CREDS.email, password: 'wrong-password-not-real' },
  });
  expect(res.status(), `bad-password login (${res.status()})`).toBe(401);
  const body = await res.json().catch(() => ({}));
  expect(body.access_token, 'no token on failed login').toBeUndefined();
});

// TODO(needs_staging): ADR-0004 per-request status='active' invariant — obtain a valid token, set the
// owner's status='suspended' on a STAGING fixture, then assert the next authed request is 401/403.
// MUTATING (suspends an account) — must NOT run against prod; add to a staging spec guarded by
// requireStaging(VITE_BASE_URL).
// TODO(needs_staging): P-b logout actually invalidates a previously valid token — login → capture token
// → POST /api/auth/logout with Authorization: Bearer <token> → assert the same token is now rejected
// (401) on an authed route. MUTATING (logout) — defer to a staging spec, do not run against prod.
