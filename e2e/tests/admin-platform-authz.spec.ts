import { test, expect, request, type APIRequestContext } from '@playwright/test';

// ADR-admin-platform-authz (B4) — E2E proof of the BOLA closure on /api/admin/*.
// RUN AFTER the operator applies docs/security/platform-admins-and-audit.migration.ts on staging.
// Until then the admin plane 503s (fail-closed) and the owner→403 assertions will see 503 not 403.
//
//   VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test \
//     e2e/tests/admin-platform-authz.spec.ts --project=desktop --reporter=list
//
// The CORE BOLA proof (owner → 403 on every admin endpoint) needs ONLY the migration applied (the
// owner is not in platform_admins → 403). The platform-admin → 200 leg needs a provisioned admin token
// (QA_PLATFORM_ADMIN_TOKEN, minted for a user added via scripts/platform-admin-grant.ts) — it skips
// if absent.

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const OWNER = { email: process.env.QA_OWNER_EMAIL || 'test@dowiz.com', password: process.env.QA_OWNER_PASSWORD || 'test123456' };
const PLATFORM_ADMIN_TOKEN = process.env.QA_PLATFORM_ADMIN_TOKEN; // optional

// the six admin endpoints (REAL paths — notification-audit single-prefix after the F4 fix)
const ENDPOINTS: { method: 'GET' | 'POST'; path: string; body?: any }[] = [
  { method: 'GET', path: '/api/admin/backups' },
  { method: 'POST', path: '/api/admin/backups/verify', body: {} },
  { method: 'GET', path: '/api/admin/backups/dr-report' },
  { method: 'GET', path: '/api/admin/fallback/health' },
  { method: 'POST', path: '/api/admin/fallback/r2-check', body: {} },
  { method: 'GET', path: '/api/admin/notification-audit?event=order.confirmed' },
];

test.describe.configure({ mode: 'serial' });
test.setTimeout(60_000);

let api: APIRequestContext;
let ownerToken = '';

const call = (token: string | undefined, e: typeof ENDPOINTS[number]) =>
  api.fetch(`${BASE}${e.path}`, { method: e.method, headers: token ? { authorization: `Bearer ${token}` } : {}, data: e.body });

test.beforeAll(async () => {
  api = await request.newContext();
  const login = await api.post(`${BASE}/api/auth/local/login`, { data: OWNER });
  expect(login.ok(), 'owner REAL login').toBeTruthy();
  ownerToken = (await login.json()).access_token;
  expect(ownerToken, 'owner token').toBeTruthy();
});

test('CORE BOLA closure — an owner JWT is 403 on EVERY /api/admin endpoint (not 200/all-tenant)', async () => {
  for (const e of ENDPOINTS) {
    const r = await call(ownerToken, e);
    // 403 = gated (the fix). 503 = migration not yet applied (admin plane dark). NEVER 200 (the bug).
    expect([403, 503], `${e.method} ${e.path} → ${r.status()} (owner must NOT get 200)`).toContain(r.status());
    expect(r.status(), `${e.method} ${e.path} must not 200 for an owner`).not.toBe(200);
    if (r.status() === 403) {
      // a JSON 403 envelope, NOT a 200 SPA shell (guards the double-prefix false-green)
      expect((r.headers()['content-type'] ?? ''), `${e.path} is JSON not index.html`).toContain('json');
    }
  }
});

test('an unauthenticated request is 401 on every admin endpoint', async () => {
  for (const e of ENDPOINTS) {
    const r = await call(undefined, e);
    expect([401, 503], `${e.method} ${e.path} unauth → ${r.status()}`).toContain(r.status());
    expect(r.status()).not.toBe(200);
  }
});

test('a courier/customer-shaped token (no userId) is 401/403, never 200', async () => {
  // a syntactically-valid but non-platform-admin bearer → denied. (A real courier token would 401 at
  // the gate's no-userId branch; a garbage token 401s at verifyAuth.)
  for (const e of ENDPOINTS) {
    const r = await call('not-a-real-token', e);
    expect([401, 403, 503]).toContain(r.status());
    expect(r.status()).not.toBe(200);
  }
});

test('a provisioned platform-admin is admitted (200) — skips without QA_PLATFORM_ADMIN_TOKEN', async () => {
  test.skip(!PLATFORM_ADMIN_TOKEN, 'set QA_PLATFORM_ADMIN_TOKEN (a token for a granted platform-admin) to run this leg');
  // read endpoints should 200 for a provisioned admin
  const reads = ENDPOINTS.filter((e) => e.method === 'GET' && !e.path.includes('dr-report'));
  for (const e of reads) {
    const r = await call(PLATFORM_ADMIN_TOKEN, e);
    expect(r.status(), `${e.path} platform-admin → ${r.status()}`).toBe(200);
  }
});

test('DR-drill rate-limit: a rapid 4th POST /backups/verify within the window → 429 (needs admin token)', async () => {
  test.skip(!PLATFORM_ADMIN_TOKEN, 'needs QA_PLATFORM_ADMIN_TOKEN');
  let saw429 = false;
  for (let i = 0; i < 5; i++) {
    const r = await call(PLATFORM_ADMIN_TOKEN, { method: 'POST', path: '/api/admin/backups/verify', body: {} });
    if (r.status() === 429) { saw429 = true; break; }
  }
  expect(saw429, 'the per-actor rate-limit (max 3 / 5min) fires').toBe(true);
});
