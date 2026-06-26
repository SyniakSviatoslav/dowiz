import { test, expect } from '@playwright/test';

function uuid() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
    const r = Math.random() * 16 | 0;
    return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
  });
}

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

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

  test('GET /s/:slug?embed=true enters embed mode', async ({ page }) => {
    // embed-mode is a client-applied body class (ClientLayout), so a no-JS request
    // can't see it — drive a browser. Canonical flag is `embed=true` (the server
    // iframe-CSP and ClientLayout both gate on it; `embed=1` is not honoured).
    const resp = await page.goto(`${BASE}/s/demo?embed=true`, { waitUntil: 'networkidle' });
    expect(resp?.status()).toBe(200);
    await expect(page.locator('body.embed-mode')).toBeAttached({ timeout: 15000 });
  });

  test('GET /s/:slug/cart returns the SPA shell', async ({ request }) => {
    const resp = await request.get(`${BASE}/s/demo/cart`);
    expect(resp.status()).toBe(200);
    // The storefront is a hydrated SPA — the shell serves the React mount, not a
    // server-rendered "Loading..." string (that moved into the client skeleton).
    const html = await resp.text();
    expect(html).toContain('id="root"');
  });

  test('GET /s/:slug/checkout returns the SPA shell', async ({ request }) => {
    const resp = await request.get(`${BASE}/s/demo/checkout`);
    expect(resp.status()).toBe(200);
    const html = await resp.text();
    expect(html).toContain('id="root"');
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
    // name is the tenant display name ("Dubin & Sushi"), not the slug; assert a real
    // per-tenant manifest: a non-empty name, the slug-scoped start_url, and icons.
    expect(body.name).toBeTruthy();
    expect(body.start_url).toContain('/s/demo');
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

  test('GET /api/auth/google honours the OAuth gate', async ({ request }) => {
    // The route is mounted under /api and is flag-gated: 302→Google when
    // GOOGLE_OAUTH_ENABLED=true, else a deliberate 404 (off by default, e.g. staging).
    const resp = await request.get(`${BASE}/api/auth/google`, { maxRedirects: 0 });
    expect([302, 404]).toContain(resp.status());
    if (resp.status() === 302) {
      expect(resp.headers()['location']).toContain('accounts.google.com');
    }
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
  // Source real IDs from the live demo menu — hardcoded UUIDs drift (the old
  // locationId 404'd as "Location not found").
  let LOCATION_ID: string;
  let PRODUCT_ID: string;
  let PIN: { lat: number; lng: number };
  test.beforeAll(async ({ request }) => {
    const menu = await (await request.get(`${BASE}/public/locations/demo/menu`)).json();
    LOCATION_ID = menu.location_id;
    PRODUCT_ID = (menu.categories || []).flatMap((c: any) => c.products || c.items || [])[0]?.id;
    // Deliver to the venue's own coordinates so the pin is always inside the
    // delivery zone (a hardcoded Tirana pin was out-of-range for the Durrës demo).
    const info = await (await request.get(`${BASE}/public/locations/demo/info`)).json();
    PIN = { lat: info.lat, lng: info.lng };
    expect(LOCATION_ID, 'demo location_id from menu').toBeTruthy();
    expect(PRODUCT_ID, 'a demo product_id from menu').toBeTruthy();
    expect(PIN.lat && PIN.lng, 'demo venue coordinates from info').toBeTruthy();
  });

  // The storefront order endpoint has a short per-IP rate-limit; an idempotent
  // replay isn't a new order, so wait out a 429 and retry rather than flake.
  async function postOrder(request: any, data: any) {
    let resp = await request.post(`${BASE}/api/orders`, { data, headers: { 'Content-Type': 'application/json' } });
    if (resp.status() === 429) {
      await new Promise(r => setTimeout(r, 8000));
      resp = await request.post(`${BASE}/api/orders`, { data, headers: { 'Content-Type': 'application/json' } });
    }
    return resp;
  }

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
      locationId: LOCATION_ID,
      type: 'delivery',
      items: [{ product_id: PRODUCT_ID, quantity: 1 }],
      customer: { phone: '+355600000001', name: 'Test' },
      delivery: { pin: PIN, address_text: 'Test Street' },
      payment: { method: 'cash' },
      idempotency_key: idemKey,
    };

    const resp = await postOrder(request, validOrder);
    const body = await resp.json();
    expect(resp.status()).toBe(201);
    expect(body.id).toBeDefined();
    expect(body.status).toBe('PENDING');
  });

  test('POST /api/orders rejects duplicate idempotency key', async ({ request }) => {
    const idemKey = uuid();
    const validOrder = {
      locationId: LOCATION_ID,
      type: 'delivery',
      items: [{ product_id: PRODUCT_ID, quantity: 1 }],
      customer: { phone: '+355600000002', name: 'Test2' },
      delivery: { pin: PIN, address_text: 'Test Street 2' },
      payment: { method: 'cash' },
      idempotency_key: idemKey,
    };

    const resp1 = await postOrder(request, validOrder);
    expect(resp1.status()).toBe(201);

    const resp2 = await postOrder(request, validOrder);
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

  test('menu_version header present on the public menu', async ({ request }) => {
    // X-Menu-Version is emitted by the cached public-menu endpoint (the FE polls it
    // for invalidation), not by the SPA shell at /s/:slug.
    const resp = await request.get(`${BASE}/public/locations/demo/menu`);
    expect(resp.headers()['x-menu-version']).toBeDefined();
  });

  test('Cache-Control set on public menu', async ({ request }) => {
    const resp = await request.get(`${BASE}/public/locations/demo/menu`);
    expect(resp.headers()['cache-control']).toContain('max-age');
  });
});
