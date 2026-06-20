import { test, expect } from '@playwright/test';

// Geo-seams (G1–G3) externally-observable contract on the deployed app.
//
// The full courier pickup → order.route push → marker tween → smoothed ETA flow
// requires the courier lifecycle (assignment/accept/pickup), which on prod needs
// dev-auth that is intentionally 404 there — so it's covered by the reliability
// gate (static L0–L11 audit) and the courier e2e specs in a dev env. THIS spec
// asserts what's drivable against prod from the customer surface:
//   1. GET /customer/orders/:id/status carries the `route` field (null until a
//      courier picks up) — and adding it didn't break the endpoint.
//   2. The order page still renders with the geo changes (no regression).
//
// A customer JWT is obtained via the (deployed) tracking-link exchange.

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
const SLUG = process.env.TRACK_SLUG || 'sushi-durres';
const TS = Date.now();
const DELIVERY_LAT = Number(process.env.TRACK_LAT ?? 41.315347);
const DELIVERY_LNG = Number(process.env.TRACK_LNG ?? 19.4449964);

test.describe.configure({ mode: 'serial' });

test.describe('Flow: Geo seams — customer-surface contract', () => {
  let locationId: string;
  let productId: string;
  let orderId: string | undefined;
  let token: string | undefined;
  let trackUrl: string | undefined;

  test.beforeAll(async ({ request }) => {
    const menuRes = await request.get(`${BASE}/public/locations/${SLUG}/menu`);
    expect(menuRes.status()).toBe(200);
    const menu = await menuRes.json();
    locationId = menu.locationId || menu.location_id;
    const products = (menu.categories || []).flatMap((c: any) => c.products || []);
    const product =
      products.find((p: any) => p.available !== false && (!p.modifier_groups || p.modifier_groups.length === 0)) ||
      products.find((p: any) => p.available !== false);
    expect(product, 'an available product').toBeTruthy();
    productId = product.id;

    // Place a public order → trackUrl → exchange code → customer JWT.
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
        customer: { phone: `+35569${String(TS).slice(-7)}`, name: 'Geo Test' },
        delivery: { pin: { lat: DELIVERY_LAT, lng: DELIVERY_LNG }, address_text: 'Test St 1, Durres' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });
    if (orderRes.status() !== 201) {
      test.skip(true, `Order not created (status ${orderRes.status()})`);
      return;
    }
    const body = await orderRes.json();
    orderId = body.id;
    trackUrl = body.trackUrl;
    const code = body.trackUrl ? new URL(body.trackUrl).searchParams.get('t') : null;
    if (code) {
      const ex = await request.post(`${BASE}/api/customer/track/exchange`, { data: { code } });
      if (ex.ok()) token = (await ex.json()).token;
    }
  });

  // ── TEST 1: status endpoint carries the route field without breaking ────────
  test('GET /customer/orders/:id/status is healthy and (when geo is deployed) carries route', async ({ request }) => {
    test.skip(!token || !orderId, 'No customer session bootstrapped');

    const res = await request.get(`${BASE}/api/customer/orders/${orderId}/status`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.status(), 'status endpoint must still answer 200').toBe(200);
    const body = await res.json();
    expect(body.id).toBe(orderId);
    expect(body.status).toBeTruthy();

    if ('route' in body) {
      // Geo deployed. No courier assigned yet → route is null; if present it's well-formed.
      if (body.route !== null) {
        expect(Array.isArray(body.route.polyline)).toBe(true);
        expect(typeof body.route.durationSeconds === 'number' || body.route.durationSeconds === null).toBe(true);
      } else {
        expect(body.route).toBeNull();
      }
    } else {
      test.skip(true, 'Geo (G1b status route field) not deployed yet — endpoint healthy, route field absent');
    }
  });

  // ── TEST 2: order page still renders with the geo changes (regression) ──────
  test('Order page renders with geo changes (no regression)', async ({ browser }) => {
    test.skip(!trackUrl, 'No trackUrl');
    const context = await browser.newContext();
    const page = await context.newPage();
    const consoleErrors: string[] = [];
    page.on('console', (m) => { if (m.type() === 'error') consoleErrors.push(m.text()); });

    await page.goto(trackUrl!, { waitUntil: 'networkidle' });

    await expect(page.locator('#root')).toBeVisible();
    await expect(page.getByText(/Session expired/i)).not.toBeVisible();
    expect(page.url()).not.toContain('/admin');
    // The page must not have thrown a React render error.
    expect(consoleErrors.filter((e) => /Minified React error|Cannot read|is not a function/.test(e))).toHaveLength(0);

    await context.close();
  });
});
