import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

/**
 * Seed bootstrap — Playwright Agents' starting position (Tooling Plan v2, Step 4).
 *
 * A deliberately "boring" fixture: it drives the app into a known logged-in,
 * seeded state (owner + courier via the dev mock-auth endpoint) and asserts the
 * owner lands on a live /admin surface. The Planner agent explores from here;
 * the Generator/Healer build real flows on top.
 *
 * No secrets live in this file — auth comes from the dev mock-auth endpoint and
 * the token is injected via localStorage (the app's `dos_access_token` key).
 *
 * This spec WRITES (mock-auth upserts the dev owner) and drives the dev mock-auth
 * backdoor, so it must NEVER touch prod: BASE defaults to staging and
 * requireStaging() fails fast if VITE_BASE_URL points at the prod host. (The dev
 * gate also 404s on prod, but we refuse to even attempt it.)
 */

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

const NIL_UUID = '00000000-0000-0000-0000-000000000000';

type MockAuth = { access_token: string; userId: string; activeLocationId: string };

test.describe('seed: bootstrap to a logged-in, seeded state', () => {
  test.beforeAll(() => {
    requireStaging(BASE);
  });

  test('owner & courier obtain dev sessions with an active location', async ({ request }) => {
    const owner = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(owner.status(), 'owner mock-auth').toBe(200);
    const ownerBody = (await owner.json()) as MockAuth;
    expectJwt(ownerBody.access_token, 'owner token');
    expectUuid(ownerBody.activeLocationId, 'owner active location');

    const courier = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } });
    expect(courier.status(), 'courier mock-auth').toBe(200);
    const courierBody = (await courier.json()) as MockAuth;
    expectJwt(courierBody.access_token, 'courier token');

    // HIGH #2 — the owner token is SCOPED to its own location (not a super-admin token).
    // The dashboard snapshot route is guarded by requireRole('owner') + requireLocationAccess
    // (apps/api/src/routes/owner/dashboard.ts:15-17), so an active-membership owner reading its
    // OWN location returns 200, while any location it has no membership on returns 404
    // (requireLocationAccess owner branch — apps/api/src/plugins/auth.ts:148-154).
    // TODO(needs-staging): asserts live data — requires a real seeded owner + DB; the dev gate
    // 404s mock-auth on prod, so this only runs against staging (VITE_BASE_URL + DEV_AUTH_SECRET).
    const ownSnap = await request.get(
      `${BASE}/api/owner/locations/${ownerBody.activeLocationId}/dashboard/snapshot`,
      { headers: { authorization: `Bearer ${ownerBody.access_token}` } },
    );
    expect(ownSnap.status(), 'owner reads its OWN location dashboard').toBe(200);

    const foreignSnap = await request.get(
      `${BASE}/api/owner/locations/${NIL_UUID}/dashboard/snapshot`,
      { headers: { authorization: `Bearer ${ownerBody.access_token}` } },
    );
    expect(foreignSnap.status(), 'owner DENIED cross-tenant location (no membership)').toBe(404);

    // HIGH #3 — role isolation: the courier token cannot reach an owner-only endpoint.
    // requireRole('owner') runs before requireLocationAccess (dashboard.ts:16-17), so the courier
    // is rejected with 403 'Forbidden role' (auth.ts:110-112) regardless of location.
    // TODO(needs-staging): live auth path — staging only (same dev-gate reason as above).
    const courierOnOwner = await request.get(
      `${BASE}/api/owner/locations/${ownerBody.activeLocationId}/dashboard/snapshot`,
      { headers: { authorization: `Bearer ${courierBody.access_token}` } },
    );
    expect(courierOnOwner.status(), 'courier token rejected from owner-only endpoint').toBe(403);
  });

  test('owner lands on a live /admin surface', async ({ page, request }) => {
    const res = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(res.status()).toBe(200);
    const { access_token } = (await res.json()) as MockAuth;

    await page.addInitScript((t: string) => localStorage.setItem('dos_access_token', t), access_token);
    await page.goto(`${BASE}/admin`, { waitUntil: 'load' });

    // Structural proof: the live owner dashboard renders its WS connection indicator
    // (apps/web/src/pages/admin/DashboardPage.tsx:439). A loose body.toContainText(/orders.../)
    // would also pass on an error page or loading spinner — assert a real admin-shell element.
    // TODO(needs-staging): requires the deployed /admin to actually render (staging only).
    await expect(page.getByTestId('ws-status-dot')).toBeVisible({ timeout: 15000 });
  });
});
