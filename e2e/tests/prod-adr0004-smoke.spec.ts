import { test, expect } from '@playwright/test';

// PROD-SAFE smoke for the deployed batch (ADR-0004 + audit/owner fixes). NON-MUTATING — no order
// create, no import. Prod uses the argon2 login path (dev-bypass off), so the 24h access TTL is
// observable here (it wasn't on staging's dev-bypass).
// Run: VITE_BASE_URL=https://dowiz.fly.dev pnpm exec playwright test prod-adr0004-smoke --project=desktop --reporter=list
const CREDS = { email: 'test@dowiz.com', password: 'test123456' };

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
  const res = await request.post('/api/auth/local/login', { data: CREDS });
  expect(res.ok(), `owner login (${res.status()})`).toBeTruthy();
  const tok = (await res.json()).access_token as string;
  const p = JSON.parse(Buffer.from(tok.split('.')[1], 'base64url').toString('utf8'));
  const ttl = p.exp - p.iat;
  // 24h = 86400s. Assert ~24h (not 7d=604800, not 1h=3600).
  expect(ttl, `access TTL ${ttl}s ~24h`).toBeGreaterThan(20 * 3600);
  expect(ttl, `access TTL ${ttl}s ~24h`).toBeLessThanOrEqual(24 * 3600 + 120);
});
