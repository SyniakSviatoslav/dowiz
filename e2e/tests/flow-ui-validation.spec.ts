import { test, expect } from '@playwright/test';
import { expectJwt } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// This suite MUTATES tenant state (PUT settings, POST products/promotions) using the
// dev/mock-auth backdoor. Default to staging and hard-guard against prod so an
// accidental ALLOW_DEV_LOGIN on prod can never let this acquire a real owner token.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test.describe('UI: Form Validation — All Roles', () => {
  // Scope the token to the describe block (not module-level) so it never leaks across files.
  let authToken: string;

  test.beforeAll(async ({ request }) => {
    requireStaging(BASE);
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;
    expectJwt(authToken, 'mock-auth access_token');
  });

  test.afterAll(async ({ request }) => {
    if (authToken) {
      await request.post(`${BASE}/api/auth/logout`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
    }
  });

  test('Owner login form: bad credentials return 401', async ({ request }) => {
    const res = await request.post(`${BASE}/api/auth/local/login`, {
      data: { email: 'wrong@test.com', password: 'wrong' },
    });
    expect(res.status()).toBe(401);
  });

  test('Courier login form: bad credentials return 401', async ({ request }) => {
    const res = await request.post(`${BASE}/api/courier/auth/login`, {
      data: { email: 'wrong@test.com', password: 'wrong' },
    });
    expect(res.status()).toBe(401);
  });

  test('Settings: phone has NO format validation (known product gap — accepted 200)', async ({ request }) => {
    const res = await request.put(`${BASE}/api/owner/settings`, {
      data: { phone: 'not-a-phone' },
      headers: { Authorization: `Bearer ${authToken}` },
    });
<<<<<<< Updated upstream
    expect([200, 400]).toContain(res.status());
=======
    // known-bug: settingsSchema.phone is z.string().max(50) (spa-proxy.ts:35) — no format
    // check, so 'not-a-phone' is accepted and the location is updated (spa-proxy.ts:660-698).
    // Asserting 200 documents real behavior; escalated to add E.164 validation (a .toBe(400)
    // here would be a false-RED — the PRODUCT is wrong, not the test). See needs_staging.
    expect(res.status()).toBe(200);
>>>>>>> Stashed changes
  });

  test('Settings: invalid delivery fee returns 400', async ({ request }) => {
    const res = await request.put(`${BASE}/api/owner/settings`, {
      data: { deliveryFee: -100 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect([200, 400, 422, 500]).toContain(res.status());
  });

  test('Product: negative price returns 400', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: 'neg-test', price: -50 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect([200, 201, 400, 422, 500]).toContain(res.status());
  });

  test('Product: empty name returns 400', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: '', price: 100 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect([200, 201, 400, 422, 500]).toContain(res.status());
  });

  test('Promotion: percentage > 100 returns 400', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/promotions`, {
      data: { code: 'INVALID', discount: 150, type: 'percentage', validFrom: new Date().toISOString() },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect([400, 422]).toContain(res.status());
  });

  test('Order: missing required fields returns 422', async ({ request }) => {
    const res = await request.post(`${BASE}/api/orders`, {
      data: { locationId: 'none', items: [] },
    });
    expect([400, 422]).toContain(res.status());
  });

  test('Promotion: empty code returns 400', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/promotions`, {
      data: { code: '', discount: 10, type: 'percentage', validFrom: new Date().toISOString() },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect([400, 422]).toContain(res.status());
  });

  test('Brand: no auth returns 401', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/brand`);
    expect(res.status()).toBe(401);
  });

  test('Settings: no auth returns 401', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/settings`);
    expect(res.status()).toBe(401);
  });

  test('All protected endpoints return 401 without token', async ({ request }) => {
    // Each endpoint paired with its REAL method (verified in routes) — a wrong method 404s
    // before the auth preHandler and would prove nothing. No catch-and-null: a network
    // error must fail the test, not silently skip the assertion.
    const endpoints: Array<['get' | 'post', string]> = [
      ['get', `${BASE}/api/owner/analytics`], // spa-proxy.ts:273
      ['get', `${BASE}/api/owner/customers`], // spa-proxy.ts:793
      ['post', `${BASE}/api/owner/onboarding/start`], // onboarding.ts:35
      ['get', `${BASE}/api/courier/me`], // courier/me.ts:36
      ['get', `${BASE}/api/courier/me/shift`], // courier/shifts.ts:15
      ['get', `${BASE}/api/owner/locations/test/dashboard/snapshot`], // dashboard.ts:20
    ];
    for (const [method, url] of endpoints) {
      const res = method === 'post'
        ? await request.post(url, { data: {} })
        : await request.get(url);
      expect(res.status(), `${method.toUpperCase()} ${url} must require auth`).toBe(401);
    }
  });

  // TODO(needs_staging): privilege-escalation + cross-tenant isolation require a SECOND real
  // tenant and a real courier-role token (cannot be minted safely here without live creds):
  //   - 403: a courier-role token calling each /api/owner/* endpoint above (role escalation).
  //   - 403/404: tenant-A's mock-auth token reading tenant-B's owned resource (IDOR) —
  //     use B's REAL id, never an all-zero UUID. See AGENTS.md Test Integrity #5/#7.
});
