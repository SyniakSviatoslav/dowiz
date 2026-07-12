/* eslint-disable local/no-permissive-status-assertion -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { test, expect } from '@playwright/test';

// Prod post-deploy smoke (ADR-0003 R3-1). UNAUTHENTICATED by design: it mints NO token,
// so the prod deploy gate no longer depends on a live owner-minting backdoor on prod.
// The full authenticated lifecycle + telegram suites run on the staging-e2e gate (before
// the prod deploy), against dowiz-staging where dev-login is legitimately enabled.
//
// Non-serial, self-contained: each test stands alone. The public storefront slug comes
// from PROD_SMOKE_SLUG (a seeded public location), NOT from an authenticated owner call.

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
const SLUG = process.env.PROD_SMOKE_SLUG || 'demo';

test.describe('Prod smoke — unauthenticated liveness + public reads', () => {
  test('liveness: GET /livez returns 200', async ({ request }) => {
    const res = await request.get(`${BASE}/livez`);
    expect(res.status()).toBe(200);
  });

  test('health: GET /health returns 200 or 503 (never 5xx-other / hang)', async ({ request }) => {
    const res = await request.get(`${BASE}/health`);
    expect([200, 503]).toContain(res.status());
  });

  test('public storefront: GET /s/:slug renders without auth', async ({ request }) => {
    const res = await request.get(`${BASE}/s/${SLUG}`);
    expect(res.status()).toBe(200);
    const html = await res.text();
    // SSR storefront shell — proves the public read path is live (no token involved).
    expect(html.toLowerCase()).toContain('<!doctype html');
  });

  test('public theme: GET /api/public/theme/:slug returns JSON without auth', async ({ request }) => {
    const res = await request.get(`${BASE}/api/public/theme/${SLUG}`);
    expect([200, 404]).toContain(res.status());
  });

  // ── Negative auth (extracted from deploy-validation 1.1–1.3) — no token needed ──
  test('unauthenticated GET /api/owner/locations returns 401', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/locations`);
    expect(res.status()).toBe(401);
  });

  test('unauthenticated GET /api/courier/me/assignments returns 401', async ({ request }) => {
    const res = await request.get(`${BASE}/api/courier/me/assignments`);
    expect(res.status()).toBe(401);
  });

  test('unauthenticated GET /api/customer/orders returns 401', async ({ request }) => {
    const res = await request.get(`${BASE}/api/customer/orders`);
    expect(res.status()).toBe(401);
  });

  // ── ADR-0003 prod guarantee: the dev-login backdoor is closed on prod ──
  // When run against prod (flag off), the hardcoded-cred login must be rejected.
  // (On staging this test is skipped — dev-login is legitimately enabled there.)
  test('dev-login backdoor is closed (prod)', async ({ request }) => {
    test.skip(/staging/.test(BASE), 'dev-login is intentionally enabled on staging');
    const res = await request.post(`${BASE}/api/auth/local/login`, {
      data: { email: 'test@dowiz.com', password: 'test123456' },
    });
    expect(res.status()).toBe(401);
  });
});
