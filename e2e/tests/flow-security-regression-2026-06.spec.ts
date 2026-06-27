import { test, expect, request as pwRequest } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// Regression coverage for the 2026-06 security review fixes.
//   C1 — /dev endpoints require DEV_AUTH_SECRET (anonymous => 404)
//   M1 — order state transitions are tenant-scoped (no cross-tenant IDOR)
//
// The Playwright config injects `x-dev-auth-secret` on every request, so the
// default `request` fixture is the "authorized harness" caller. For C1 we build
// a SECOND context with no secret header to act as the anonymous attacker.

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

// Every known /dev + /api/dev path. The server.ts onRequest guard (isDevRequestAuthorized,
// dev-guard.ts) 404s ALL of them for anonymous callers — assert the whole family, not a sample.
const DEV_PATHS = [
  '/api/dev/mock-auth',
  '/api/dev/create-assignment',
  '/api/dev/seed-data',
  '/api/dev/seed-visual-state',
  '/dev/seed-telegram-target',
  '/dev/repair-test-owner',
];

test.describe.configure({ mode: 'serial' });

test.describe('Security regression 2026-06', () => {
  // mock-auth UPSERTs the dev owner (a write) — never let this run against prod.
  test.beforeAll(() => requireStaging(BASE));

  // ── C1: dev token-minting endpoints reject anonymous callers ──────────────
  for (const path of DEV_PATHS) {
    test(`C1: ${path} without the shared secret returns 404`, async () => {
      const anon = await pwRequest.newContext(); // no extraHTTPHeaders → no secret
      const res = await anon.post(`${BASE}${path}`, { data: { role: 'owner' } });
      expect(res.status(), 'anonymous dev path must be indistinguishable from a missing route').toBe(404);
      await anon.dispose();
    });
  }

  test('C1: harness WITH the secret still mints a real owner JWT', async ({ request }) => {
    const res = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'owner', locationSlug: 'demo' } });
    expect(res.status()).toBe(200);
    const token = (await res.json()).access_token;
    expectJwt(token, 'access_token');
    // Decode the JWT payload and assert the claim — a non-empty error body would pass expectJwt
    // only by accident; the role claim proves the secret minted a genuine owner token.
    const claims = JSON.parse(Buffer.from(String(token).split('.')[1], 'base64url').toString());
    expect(claims.role, 'minted token must carry owner role').toBe('owner');
  });

  // ── M1: order transitions are tenant-scoped ───────────────────────────────
  // Full cross-tenant proof needs an owner of two locations with an order in
  // one; here we assert the location-scoped lookup rejects an order that does
  // not belong to the URL location (a fake order id under the owned location,
  // and any order id under a location the caller does not own).
  //
  // TODO(needs_staging): this asserts a 404-by-absence, not a 404-by-ownership. A true IDOR
  // proof needs a REAL second tenant (distinct owner user) with a REAL order, then transition
  // it with tenant A's token → must 404 while the order exists. mock-auth cannot supply this:
  // every owner mint is the SAME dev user (dev@deliveryos.com), so two slugs share one tenant.
  // Requires seeding a second tenant + order on staging — do not fake with an all-zero id.
  test('M1: confirming under a non-owned location returns 404 (no leak)', async ({ request }) => {
    const auth = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'owner', locationSlug: 'demo' } });
    const token = (await auth.json()).access_token;
    const foreignLocation = '00000000-0000-0000-0000-000000000000';
    const someOrder = '00000000-0000-0000-0000-0000000000aa';

    const res = await request.post(
      `${BASE}/api/owner/locations/${foreignLocation}/orders/${someOrder}/confirm`,
      { headers: { Authorization: `Bearer ${token}` }, data: {} },
    );
    expect(res.status(), 'cross-tenant transition must 404').toBe(404);
  });

  test('M1: confirming an unknown order in the owned location returns 404', async ({ request }) => {
    const auth = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'owner', locationSlug: 'demo' } });
    const body = await auth.json();
    const locationId = body.activeLocationId;
    expectUuid(locationId, 'mock-auth demo location');
    const unknownOrder = '00000000-0000-0000-0000-0000000000bb';

    const res = await request.post(
      `${BASE}/api/owner/locations/${locationId}/orders/${unknownOrder}/confirm`,
      { headers: { Authorization: `Bearer ${body.access_token}` }, data: {} },
    );
    expect(res.status()).toBe(404);
  });

  // ── U1: /admin requires auth (no shell for anonymous users) ───────────────
  // Use a fresh context (no storage at all) so the guard sees a genuinely token-less
  // session on first navigation — clearing localStorage AFTER a goto races the guard,
  // which may already have fired against a populated store.
  test('U1: /admin redirects unauthenticated users to /login', async ({ browser }) => {
    const ctx = await browser.newContext();
    const page = await ctx.newPage();
    await page.goto(`${BASE}/admin`);
    await page.waitForURL('**/login', { timeout: 10_000 });
    await expect(page).toHaveURL(/\/login/);
    await ctx.close();
  });
});
