import { test, expect } from '@playwright/test';
import { expectUuid, expectJwt } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// Decode a JWT payload segment (base64url) without verifying the signature — we only
// need to inspect the claims the server minted, not trust them.
function decodeJwtClaims(token: string): Record<string, unknown> {
  const seg = token.split('.')[1] ?? '';
  return JSON.parse(Buffer.from(seg, 'base64url').toString('utf8'));
}

// Proof for the customer order tracking-link handoff:
//   order mint -> trackUrl with ?t=<opaque code>
//   POST /api/customer/track/exchange -> reissued 7-day customer JWT
//   clean-profile open of trackUrl -> page self-authenticates, strips ?t=, renders order
//
// Bootstraps entirely from PUBLIC endpoints (no owner/dev auth). It still PLACES real
// orders, so it is a mutating spec — requireStaging() in beforeAll refuses to run it
// against prod (a real order must never be minted there by a test).

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const SLUG = process.env.TRACK_SLUG || 'sushi-durres'; // OTP off, min_order 0, real coords
const TS = Date.now();

// Location's own coordinates → delivery pin is in range. Overridable per env.
const DELIVERY_LAT = Number(process.env.TRACK_LAT ?? 41.315347);
const DELIVERY_LNG = Number(process.env.TRACK_LNG ?? 19.4449964);

test.describe.configure({ mode: 'serial' });

test.describe('Flow: Customer Track Link — Exchange Handoff', () => {
  let locationId: string;
  let productId: string;

  let trackUrl: string | undefined;
  let trackCode: string | undefined;
  let orderId: string | undefined;
  let customerToken: string | undefined;

  test.beforeAll(async ({ request }) => {
    requireStaging(BASE); // mutating spec (places orders) — never run against prod
    const menuRes = await request.get(`${BASE}/public/locations/${SLUG}/menu`);
    expect(menuRes.status()).toBe(200);
    const menu = await menuRes.json();
    locationId = menu.locationId || menu.location_id;
    expectUuid(locationId, 'public menu must expose a locationId');

    // Prefer a product with no modifier groups so an order with empty modifiers
    // can't be rejected for a missing required choice.
    const products = (menu.categories || []).flatMap((c: any) => c.products || []);
    const product =
      products.find((p: any) => p.available !== false && (!p.modifier_groups || p.modifier_groups.length === 0)) ||
      products.find((p: any) => p.available !== false);
    expect(product, 'at least one available product').toBeTruthy();
    productId = product.id;
  });

  // ── TEST 1: mint returns a trackUrl carrying an opaque ?t= code ──────────
  test('Order mint returns a trackUrl with an opaque ?t= grant code', async ({ request }) => {
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
        customer: { phone: `+35569${String(TS).slice(-7)}`, name: 'Track Test' },
        delivery: {
          pin: { lat: DELIVERY_LAT, lng: DELIVERY_LNG },
          address_text: 'Test Street 1, Durres',
        },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });

    // Only a 201 with a resolved customer mints a grant. Other business outcomes
    // (422 range/min-order, 429 throttle) are valid responses but can't prove the
    // link — fail loudly only on a true error (500/401), else skip the chain.
    if (orderRes.status() !== 201) {
      const body = await orderRes.json().catch(() => ({}));
      expect(orderRes.status(), `order not 201 (got ${orderRes.status()}: ${JSON.stringify(body)})`).not.toBe(500);
      expect(orderRes.status()).not.toBe(401);
      test.skip(true, `Order not created (status ${orderRes.status()}); cannot mint grant`);
      return;
    }

    const body = await orderRes.json();
    orderId = body.id;

    expect(typeof body.trackUrl).toBe('string');
    expect(body.trackUrl).toContain(`/order/${body.id}`);
    expect(body.trackUrl).toContain('?t=');

    trackUrl = body.trackUrl;
    trackCode = new URL(body.trackUrl).searchParams.get('t') || undefined;
    expect((trackCode || '').length).toBeGreaterThan(20);
  });

  // ── TEST 2: exchange trades the opaque code for a customer JWT ───────────
  test('POST /api/customer/track/exchange returns a customer JWT (no auth header)', async ({ request }) => {
    test.skip(!trackCode, 'No track code minted in TEST 1');

    const res = await request.post(`${BASE}/api/customer/track/exchange`, {
      data: { code: trackCode },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expectJwt(body.token, 'exchange token'); // 3-segment RS256 JWT shape
    customerToken = body.token;

    // Inspect the minted claims — structure alone (split length 3) doesn't prove scope.
    // issueCustomerToken (packages/platform/src/auth/jwt.ts) signs role/orderId/sub(=customerId)/exp.
    // NOTE: phone is intentionally NOT in the JWT (P0-PII), so `sub` is the customerId UUID,
    // never the phone — assert the UUID shape, not the phone.
    const claims = decodeJwtClaims(body.token);
    expect(claims.role).toBe('customer');
    expect(claims.orderId).toBe(orderId);
    expectUuid(claims.sub, 'JWT sub must be the customerId UUID');
    // 7-day token: exp must sit ~7d out (allow 6–8d to absorb signing skew).
    const ttlSec = (claims.exp as number) - Math.floor(Date.now() / 1000);
    expect(ttlSec).toBeGreaterThan(6 * 24 * 3600);
    expect(ttlSec).toBeLessThanOrEqual(8 * 24 * 3600);

    // The reissued JWT must actually authorize the order's status endpoint.
    const statusRes = await request.get(`${BASE}/api/customer/orders/${orderId}/status`, {
      headers: { Authorization: `Bearer ${body.token}` },
    });
    expect(statusRes.status()).toBe(200);

    // Reuse is intentional: the grant is reusable-until-expiry (track.ts: use_count is an
    // abuse signal, NOT a single-use gate), so a second exchange of the same code must
    // still return 200 — this guards against an accidental single-use regression.
    const replay = await request.post(`${BASE}/api/customer/track/exchange`, {
      data: { code: trackCode },
    });
    expect(replay.status()).toBe(200);
  });

  // ── TEST 3: bogus / expired codes are rejected with 410 ─────────────────
  test('Exchange of an unknown code returns 410 Gone', async ({ request }) => {
    const res = await request.post(`${BASE}/api/customer/track/exchange`, {
      data: { code: 'this-is-not-a-real-grant-code-000000000000' },
    });
    expect(res.status()).toBe(410);
  });

  // ── TEST 4: clean-profile open of trackUrl self-authenticates (UI) ──────
  test('Opening trackUrl in a fresh browser context renders the order, not "session expired"', async ({ browser }) => {
    test.skip(!trackUrl, 'No trackUrl minted in TEST 1');

    // Fresh context = no dos_access_token, exactly like tapping the link on a new device.
    const context = await browser.newContext();
    const page = await context.newPage();
    await page.goto(trackUrl!, { waitUntil: 'networkidle' });

    // The auth-expired EmptyState must NOT appear — the page exchanged ?t= for a session.
    await expect(page.getByText(/Session expired/i)).not.toBeVisible();

    // No bounce to the owner login.
    expect(page.url()).not.toContain('/admin');

    // replaceState stripped the secret from the visible URL.
    expect(page.url()).not.toContain('?t=');

    // A customer JWT is now stored — proof the exchange ran client-side.
    const stored = await page.evaluate(() => localStorage.getItem('dos_access_token'));
    expect(stored).toBeTruthy();

    // The app shell rendered (real DOM element visible).
    await expect(page.locator('#root')).toBeVisible();

    // Positive proof the ORDER actually rendered (not just an empty shell / spinner):
    // the live order-status badge is the order-content anchor (OrderStatusPage.tsx).
    await expect(page.locator('[data-testid="order-status-badge"]')).toBeVisible({ timeout: 15000 });

    await context.close();
  });

  // ── TEST 5: cross-order IDOR — a customer JWT is scoped to its own order ──
  // The status route filters `WHERE o.id = $1 AND o.customer_id = $2`
  // (apps/api/src/routes/customer/orders.ts), so order1's token must NOT read order2
  // (placed by a different phone => different customer_id => 404 NOT_FOUND).
  // TODO(needs-staging): requires a live staging run to mint a 2nd real order.
  test('A customer JWT cannot read a different customer order (cross-order IDOR)', async ({ request }) => {
    test.skip(!customerToken, 'No customer JWT minted in TEST 2');

    // Place a SECOND order under a DIFFERENT phone => a different customer_id.
    const order2Res = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
        customer: { phone: `+35568${String(TS).slice(-7)}`, name: 'IDOR Other' },
        delivery: {
          pin: { lat: DELIVERY_LAT, lng: DELIVERY_LNG },
          address_text: 'Test Street 2, Durres',
        },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });
    if (order2Res.status() !== 201) {
      const b = await order2Res.json().catch(() => ({}));
      expect(order2Res.status(), `2nd order not 201 (${order2Res.status()}: ${JSON.stringify(b)})`).not.toBe(500);
      expect(order2Res.status()).not.toBe(401);
      test.skip(true, `2nd order not created (status ${order2Res.status()}); cannot run IDOR check`);
      return;
    }
    const order2Id = (await order2Res.json()).id;
    expectUuid(order2Id, '2nd orderId');
    expect(order2Id).not.toBe(orderId); // genuinely a different order

    // order1's token reading order2 must be rejected (row-scoped to its own customer_id).
    const idorRes = await request.get(`${BASE}/api/customer/orders/${order2Id}/status`, {
      headers: { Authorization: `Bearer ${customerToken}` },
    });
    expect(idorRes.status()).toBe(404);
  });
});
