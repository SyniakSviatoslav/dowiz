import { test, expect } from '@playwright/test';

function uuid() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
    const r = Math.random() * 16 | 0;
    return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
  });
}

const BASE = 'https://dowiz.fly.dev';

test.describe('Real API — Public Endpoints', () => {

  test('GET /health returns 200 with checks', async ({ request }) => {
    const resp = await request.get(`${BASE}/health`);
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.status).toBeDefined();
    expect(body.checks.postgres.status).toBe('ok');
  });

  test('GET /s/:slug SSR returns Albanian locale', async ({ request }) => {
    // Full SSR (the hand-templated menu) is served only to crawlers — real
    // browsers get the SPA shell and hydrate. Send a bot UA to exercise the SSR
    // path this test is named for. (data-text-sq is gone: the SSR renderer now
    // resolves locale text server-side and emits final strings, so we assert the
    // server-rendered menu markup directly.)
    const resp = await request.get(`${BASE}/s/demo`, {
      headers: { 'user-agent': 'Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)' },
    });
    expect(resp.status()).toBe(200);
    const html = await resp.text();
    expect(html).toContain('lang="sq"');
    expect(html).toContain('product-card');
    expect(html).toContain('product-name');
  });

  test('GET /s/:slug?embed=1 returns embed mode', async ({ request }) => {
    const resp = await request.get(`${BASE}/s/demo?embed=1`);
    expect(resp.status()).toBe(200);
    const html = await resp.text();
    expect(html).toContain('embed-mode');
  });

  test('GET /s/:slug/cart returns cart shell', async ({ request }) => {
    const resp = await request.get(`${BASE}/s/demo/cart`);
    expect(resp.status()).toBe(200);
    const html = await resp.text();
    expect(html).toContain('Loading...');
  });

  test('GET /s/:slug/checkout returns checkout shell', async ({ request }) => {
    const resp = await request.get(`${BASE}/s/demo/checkout`);
    expect(resp.status()).toBe(200);
    const html = await resp.text();
    expect(html).toContain('Loading...');
  });

  test('GET /public/locations/:slug/menu returns JSON', async ({ request }) => {
    const resp = await request.get(`${BASE}/public/locations/demo/menu`);
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.default_locale).toBe('sq');
    expect(body.supported_locales).toContain('sq');
    expect(body.categories.length).toBeGreaterThan(0);
  });

  test('GET /public/locations/:id/theme.css returns CSS', async ({ request }) => {
    const resp = await request.get(`${BASE}/public/locations/demo/theme.css`);
    expect(resp.status()).toBe(200);
    const css = await resp.text();
    expect(css).toContain('--brand-primary');
  });

  test('GET /s/:slug/manifest.webmanifest returns manifest', async ({ request }) => {
    const resp = await request.get(`${BASE}/s/demo/manifest.webmanifest`);
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.name).toBe('demo');
    expect(body.icons.length).toBeGreaterThan(0);
  });

  test('GET /api/push/vapid-public-key returns key', async ({ request }) => {
    const resp = await request.get(`${BASE}/api/push/vapid-public-key`);
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.publicKey).toBeTruthy();
  });

  test('POST /api/telemetry accepts events', async ({ request }) => {
    const resp = await request.post(`${BASE}/api/telemetry`, {
      data: { action: 'cart.added', locationId: 'test' }
    });
    expect(resp.status()).toBe(202);
  });

  test('GET /auth/google redirects to Google OAuth', async ({ request }) => {
    const resp = await request.get(`${BASE}/auth/google`, { maxRedirects: 0 });
    expect(resp.status()).toBe(302);
    const location = resp.headers()['location'];
    expect(location).toContain('accounts.google.com');
  });

  test('GET /robots.txt returns robots', async ({ request }) => {
    const resp = await request.get(`${BASE}/robots.txt`);
    expect(resp.status()).toBe(200);
    const text = await resp.text();
    expect(text).toContain('User-agent');
  });
});

test.describe('Real API — Auth-Required Endpoints', () => {

  test('GET /api/owner/locations/:id/dashboard/snapshot requires auth', async ({ request }) => {
    const resp = await request.get(`${BASE}/api/owner/locations/11111111-1111-1111-1111-111111111111/dashboard/snapshot`);
    expect(resp.status()).toBe(401);
  });

  test('GET /api/courier/me requires auth', async ({ request }) => {
    const resp = await request.get(`${BASE}/api/courier/me`);
    expect(resp.status()).toBe(401);
  });

  test('GET /api/owner/locations/:id/signals requires auth', async ({ request }) => {
    const resp = await request.get(`${BASE}/api/owner/locations/11111111-1111-1111-1111-111111111111/signals`);
    expect(resp.status()).toBe(401);
  });
});

test.describe('Real API — Idempotency & Order Flow', () => {

  test('POST /api/orders validates input schema', async ({ request }) => {
    const resp = await request.post(`${BASE}/api/orders`, {
      data: { invalid: true },
      headers: { 'Content-Type': 'application/json' }
    });
    expect(resp.status()).toBeGreaterThanOrEqual(400);
  });

  test('POST /api/orders creates order with valid data', async ({ request }) => {
    const idemKey = uuid();
    const validOrder = {
      locationId: '1f609add-062a-4bb5-89bf-d695f963ede6',
      type: 'delivery',
      items: [{ product_id: '1b4e1275-3f37-47e5-8652-1ebd6c8de04a', quantity: 1 }],
      customer: { phone: '+355600000001', name: 'Test' },
      delivery: { pin: { lat: 41.3275, lng: 19.8187 }, address_text: 'Test Street' },
      payment: { method: 'cash' },
      idempotency_key: idemKey,
    };

    const resp = await request.post(`${BASE}/api/orders`, {
      data: validOrder,
      headers: { 'Content-Type': 'application/json' }
    });
    const body = await resp.json();
    expect(resp.status()).toBe(201);
    expect(body.id).toBeDefined();
    expect(body.status).toBe('PENDING');
  });

  test('POST /api/orders rejects duplicate idempotency key', async ({ request }) => {
    const idemKey = uuid();
    const validOrder = {
      locationId: '1f609add-062a-4bb5-89bf-d695f963ede6',
      type: 'delivery',
      items: [{ product_id: '1b4e1275-3f37-47e5-8652-1ebd6c8de04a', quantity: 1 }],
      customer: { phone: '+355600000002', name: 'Test2' },
      delivery: { pin: { lat: 41.3275, lng: 19.8187 }, address_text: 'Test Street 2' },
      payment: { method: 'cash' },
      idempotency_key: idemKey,
    };

    const resp1 = await request.post(`${BASE}/api/orders`, {
      data: validOrder,
      headers: { 'Content-Type': 'application/json' }
    });
    expect(resp1.status()).toBe(201);

    const resp2 = await request.post(`${BASE}/api/orders`, {
      data: validOrder,
      headers: { 'Content-Type': 'application/json' }
    });
    expect(resp2.status()).toBe(200);
    const body2 = await resp2.json();
    const body1 = await resp1.json();
    expect(body2.id).toBe(body1.id);
  });
});

test.describe('Real API — Security', () => {

  test('No cookies set on any public endpoint', async ({ request }) => {
    const endpoints = ['/s/demo', '/health', '/public/locations/demo/menu'];
    for (const ep of endpoints) {
      const resp = await request.get(`${BASE}${ep}`);
      const cookies = resp.headers()['set-cookie'];
      expect(cookies, `${ep} set cookies`).toBeUndefined();
    }
  });

  test('Cross-tenant access rejected', async ({ request }) => {
    const wrongLocationResp = await request.get(
      `${BASE}/api/owner/locations/00000000-0000-0000-0000-000000000001/dashboard/snapshot`
    );
    expect(wrongLocationResp.status()).toBe(401);
  });

  test('CSP present on SSR pages', async ({ request }) => {
    const resp = await request.get(`${BASE}/s/demo`);
    expect(resp.headers()['content-security-policy']).toBeDefined();
  });

  test('Rate limit headers present', async ({ request }) => {
    const resp = await request.get(`${BASE}/s/demo`);
    expect(resp.headers()['x-ratelimit-limit']).toBeDefined();
    expect(resp.headers()['x-ratelimit-remaining']).toBeDefined();
  });
});

test.describe('Real API — Caching', () => {

  test('menu_version header present on SSR', async ({ request }) => {
    const resp = await request.get(`${BASE}/s/demo`);
    expect(resp.headers()['x-menu-version']).toBeDefined();
  });

  test('Cache-Control set on public menu', async ({ request }) => {
    const resp = await request.get(`${BASE}/public/locations/demo/menu`);
    expect(resp.headers()['cache-control']).toContain('max-age');
  });
});
