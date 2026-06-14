import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
let authToken: string;

test.describe('UI: Form Validation — All Roles', () => {
  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;
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

  test('Settings: invalid phone returns 400', async ({ request }) => {
    const res = await request.put(`${BASE}/api/owner/settings`, {
      data: { phone: 'not-a-phone' },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(400);
  });

  test('Settings: invalid delivery fee returns 400', async ({ request }) => {
    const res = await request.put(`${BASE}/api/owner/settings`, {
      data: { deliveryFee: -100 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect([400, 422]).toContain(res.status());
  });

  test('Product: negative price returns 400', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: 'neg-test', price: -50 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect([400, 422]).toContain(res.status());
  });

  test('Product: empty name returns 400', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: '', price: 100 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect([400, 422]).toContain(res.status());
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
    const endpoints = [
      `${BASE}/api/owner/analytics`,
      `${BASE}/api/owner/customers`,
      `${BASE}/api/owner/onboarding/start`,
      `${BASE}/api/courier/me`,
      `${BASE}/api/courier/me/shift`,
      `${BASE}/api/owner/locations/test/dashboard/snapshot`,
    ];
    for (const url of endpoints) {
      const res = await request.post(url, {}).catch(() => null) || await request.get(url);
      if (res) expect(res.status()).toBe(401);
    }
  });
});
