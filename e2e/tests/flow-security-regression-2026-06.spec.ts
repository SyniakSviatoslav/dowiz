import { test, expect, request as pwRequest } from '@playwright/test';

// Regression coverage for the 2026-06 security review fixes.
//   C1 — /dev endpoints require DEV_AUTH_SECRET (anonymous => 404)
//   M1 — order state transitions are tenant-scoped (no cross-tenant IDOR)
//
// The Playwright config injects `x-dev-auth-secret` on every request, so the
// default `request` fixture is the "authorized harness" caller. For C1 we build
// a SECOND context with no secret header to act as the anonymous attacker.

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe.configure({ mode: 'serial' });

test.describe('Security regression 2026-06', () => {
  // ── C1: dev token-minting endpoints reject anonymous callers ──────────────
  test('C1: /api/dev/mock-auth without the shared secret returns 404', async () => {
    const anon = await pwRequest.newContext(); // no extraHTTPHeaders → no secret
    const res = await anon.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'owner' } });
    expect(res.status(), 'anonymous dev-auth must be indistinguishable from a missing route').toBe(404);
    await anon.dispose();
  });

  test('C1: /api/dev/create-assignment without the secret returns 404', async () => {
    const anon = await pwRequest.newContext();
    const res = await anon.post(`${BASE}/api/dev/create-assignment`, { data: {} });
    expect(res.status()).toBe(404);
    await anon.dispose();
  });

  test('C1: harness WITH the secret still mints a token', async ({ request }) => {
    const res = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'owner', locationSlug: 'demo' } });
    expect(res.status()).toBe(200);
    expect((await res.json()).access_token).toBeTruthy();
  });

  // ── M1: order transitions are tenant-scoped ───────────────────────────────
  // Full cross-tenant proof needs an owner of two locations with an order in
  // one; here we assert the location-scoped lookup rejects an order that does
  // not belong to the URL location (a fake order id under the owned location,
  // and any order id under a location the caller does not own).
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
    expect(locationId, 'mock-auth should resolve the demo location').toBeTruthy();
    const unknownOrder = '00000000-0000-0000-0000-0000000000bb';

    const res = await request.post(
      `${BASE}/api/owner/locations/${locationId}/orders/${unknownOrder}/confirm`,
      { headers: { Authorization: `Bearer ${body.access_token}` }, data: {} },
    );
    expect(res.status()).toBe(404);
  });
});
