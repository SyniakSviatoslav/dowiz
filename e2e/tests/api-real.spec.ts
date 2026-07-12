import { test, expect } from '@playwright/test';
<<<<<<< Updated upstream
=======
import { expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';
>>>>>>> Stashed changes

function uuid() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
    const r = Math.random() * 16 | 0;
    return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
  });
}

<<<<<<< Updated upstream
const BASE = 'https://dowiz.fly.dev';
=======
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
>>>>>>> Stashed changes

test.describe('Real API — Public Endpoints', () => {

  test('GET /health returns 200 with checks', async ({ request }) => {
    const resp = await request.get(`${BASE}/health`);
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.status).toBeDefined();
    expect(body.checks.postgres.status).toBe('ok');
  });

  test('GET /s/:slug SSR returns Albanian locale', async ({ request }) => {
    const resp = await request.get(`${BASE}/s/demo`);
    expect(resp.status()).toBe(200);
    const html = await resp.text();
    expect(html).toContain('lang="sq"');
    expect(html).toContain('data-text-sq');
    expect(html).toContain('product-card');
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

  test('GET /public/locations/:slug/menu 404s an unknown slug', async ({ request }) => {
    // Exercise the absent-tenant path (apps/api/src/routes/public/menu.ts:170 →
    // NOT_FOUND). Note: /s/:slug is NOT tested for 404 — it serves the SPA shell at
    // HTTP 200 for any slug by design (ssr.ts → serveSpaShell); the client resolves
    // an unknown tenant, so a 404 assertion there would be a false-red.
    const resp = await request.get(`${BASE}/public/locations/this-slug-does-not-exist-xyz/menu`);
    expect(resp.status()).toBe(404);
    const body = await resp.json();
    expect(body.code).toBe('NOT_FOUND');
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
<<<<<<< Updated upstream
=======
  // Source real IDs from the live demo menu — hardcoded UUIDs drift (the old
  // locationId 404'd as "Location not found").
  let LOCATION_ID: string;
  let PRODUCT_ID: string;
  let PIN: { lat: number; lng: number };
  test.beforeAll(async ({ request }) => {
    // This describe POSTs real orders — never let it run against prod.
    requireStaging(BASE);
    const menuResp = await request.get(`${BASE}/public/locations/demo/menu`);
    expect(menuResp.status(), 'demo menu setup must be 200').toBe(200);
    const menu = await menuResp.json();
    LOCATION_ID = menu.location_id;
    PRODUCT_ID = (menu.categories || []).flatMap((c: any) => c.products || c.items || [])[0]?.id;
    // Deliver to the venue's own coordinates so the pin is always inside the
    // delivery zone (a hardcoded Tirana pin was out-of-range for the Durrës demo).
    const info = await (await request.get(`${BASE}/public/locations/demo/info`)).json();
    PIN = { lat: info.lat, lng: info.lng };
    expectUuid(LOCATION_ID, 'demo location_id from menu');
    expectUuid(PRODUCT_ID, 'a demo product_id from menu');
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
>>>>>>> Stashed changes

  test('POST /api/orders validates input schema', async ({ request }) => {
    const resp = await request.post(`${BASE}/api/orders`, {
      data: { invalid: true },
      headers: { 'Content-Type': 'application/json' }
    });
    // EXACT 400 — a Zod parse failure returns VALIDATION_FAILED (apps/api/src/routes/orders.ts:89).
    // A 500 (unhandled throw on bad input) must FAIL this test, not pass as "validation working".
    expect(resp.status()).toBe(400);
    const body = await resp.json();
    expect(body.code).toBe('VALIDATION_FAILED');
    expect(body.error, 'validation error message present').toBeTruthy();
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

  // TODO(needs_staging): the runtime `test.skip` on the §4 velocity gate (429 /
  // soft_confirm) is per-IP and only deterministic on a fresh CI IP. To delete the
  // skip without flaking, run this spec from a dedicated staging IP with a reserved
  // idempotency-test phone prefix so the baseline create is always 201, then assert
  // 200 + same-id replay unconditionally.
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

  test('Owner snapshot rejects an unauthenticated request (negative control)', async ({ request }) => {
    // NEGATIVE control only: no credentials → 401 (auth gate fires before any tenant
    // check). This does NOT prove cross-tenant isolation — see the TODO below.
    const noAuthResp = await request.get(
      `${BASE}/api/owner/locations/00000000-0000-0000-0000-000000000001/dashboard/snapshot`
    );
    expect(noAuthResp.status()).toBe(401);
    // TODO(needs_staging): true cross-tenant isolation requires a POSITIVE control —
    // authenticate as the demo owner, then GET another REAL tenant's location UUID with
    // that valid token and assert 403/404 (not 401). That needs a real owner JWT + a
    // second seeded tenant on staging; an all-zero UUID 404s by absence and proves
    // nothing (Test-Integrity rule 5). Do not fake the token.
  });

  test('CSP present and locked-down on SSR pages', async ({ request }) => {
    const resp = await request.get(`${BASE}/s/demo`);
    const csp = resp.headers()['content-security-policy'];
    // Content, not just presence: the storefront CSP (apps/api/src/lib/spa-shell.ts)
    // must default-deny to 'self' and never widen default-src to a wildcard.
    expect(csp, 'CSP header present').toBeTruthy();
    expect(csp).toContain("default-src 'self'");
    expect(csp).not.toContain('default-src *');
    expect(csp).not.toContain("default-src 'unsafe-eval'");
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
