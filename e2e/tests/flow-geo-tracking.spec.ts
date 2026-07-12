import { test, expect } from '@playwright/test';
import { requireStaging } from '../helpers/staging-guard';
import { expectJwt, expectUuid } from '../helpers/assert-shape';

// Geo-seams (G1–G3) externally-observable contract on the deployed app.
//
// The full courier pickup → order.route push → marker tween → smoothed ETA flow
// requires the courier lifecycle (assignment/accept/pickup), which on prod needs
// dev-auth that is intentionally 404 there — so it's covered by the reliability
// gate (static L0–L11 audit) and the courier e2e specs in a dev env. This spec
// MUTATES (it places an order) so it is guarded to STAGING only and asserts what
// is drivable from the customer surface:
//   1. GET /customer/orders/:id/status carries the `route` field (null until a
//      courier picks up) — and adding it didn't break the endpoint.
//   2. The order page still renders with the geo changes (no regression).
//
// A customer JWT is obtained via the (deployed) tracking-link exchange.

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
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
    requireStaging(BASE); // mutating spec (places an order) — never run against prod
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
    expectUuid(orderId, 'created order id');
    const code = trackUrl ? new URL(trackUrl).searchParams.get('t') : null;
    expect(code, 'trackUrl must carry a tracking code (?t=)').toBeTruthy();
    // A non-ok exchange is an AUTH regression — fail, never silently skip the suite.
    const ex = await request.post(`${BASE}/api/customer/track/exchange`, { data: { code } });
    expect(ex.status(), 'token exchange must succeed').toBe(200);
    token = (await ex.json()).token;
    expectJwt(token, 'customer token');
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
    // Exact enum membership (order_status) — not just any non-empty string.
    expect(
      ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED', 'REJECTED', 'CANCELLED', 'SCHEDULED', 'PICKED_UP'],
      `unexpected order status ${body.status}`,
    ).toContain(body.status);

    // Geo (G1b) is deployed: `route` is ALWAYS a key on the response (null until a
    // courier picks up). A missing key is a regression, not a skip.
    expect('route' in body, 'status response must carry the route field (G1b)').toBe(true);
    if (body.route !== null) {
      expect(Array.isArray(body.route.polyline), 'route.polyline must be an array').toBe(true);
      const ds = body.route.durationSeconds;
      expect(ds === null || typeof ds === 'number', `route.durationSeconds must be number|null, got ${typeof ds}`).toBe(true);
    } else {
      // No courier assigned yet — route is null (the common case on a fresh order).
      expect(body.route).toBeNull();
    }
  });

  // ── TEST 1b: status endpoint enforces auth + ownership (negative controls) ──
  test('GET /customer/orders/:id/status rejects no-token (401) and unknown order (404)', async ({ request }) => {
    test.skip(!token || !orderId, 'No customer session bootstrapped');

    // Negative control — no Authorization header → 401 (the gate isn't open to anon).
    const anon = await request.get(`${BASE}/api/customer/orders/${orderId}/status`);
    expect(anon.status(), 'no-token request must be rejected 401').toBe(401);

    // Ownership control — a valid customer token cannot read an order it does not own.
    // WHERE o.customer_id = token.sub → a non-owned order id resolves to 404 (no leak/no 500).
    // TODO(needs_staging): a fabricated UUID 404s by absence; a TRUE IDOR proof needs a REAL
    // second tenant's real order id (assert 404, not 200) — requires a 2nd seeded customer.
    const fabricated = '11111111-1111-4111-8111-111111111111';
    const idor = await request.get(`${BASE}/api/customer/orders/${fabricated}/status`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(idor.status(), 'unknown/non-owned order must be 404 (no cross-tenant leak)').toBe(404);
  });

  // ── TEST 2: order page still renders with the geo changes (regression) ──────
  test('Order page renders with geo changes (no regression)', async ({ browser }) => {
    test.skip(!trackUrl, 'No trackUrl');
    const context = await browser.newContext();
    const page = await context.newPage();
    const consoleErrors: string[] = [];
    page.on('console', (m) => { if (m.type() === 'error') consoleErrors.push(m.text()); });

    await page.goto(trackUrl!, { waitUntil: 'networkidle' });

    // Specific render proof: the order-status UI mounted (a 500/redirect/spinner would fail).
    await expect(page.getByTestId('order-status-badge')).toBeVisible();
    await expect(page.getByText(/Session expired/i)).not.toBeVisible();
    expect(page.url()).not.toContain('/admin');
    // The page must not have thrown a React render error.
    expect(consoleErrors.filter((e) => /Minified React error|Cannot read|is not a function/.test(e))).toHaveLength(0);

    await context.close();
  });
});
