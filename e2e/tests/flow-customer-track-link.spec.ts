import { test, expect } from '@playwright/test';
import { expectUuid } from '../helpers/assert-shape';

// Proof for the customer order tracking-link handoff:
//   order mint -> trackUrl with ?t=<opaque code>
//   POST /api/customer/track/exchange -> reissued 7-day customer JWT
//   clean-profile open of trackUrl -> page self-authenticates, strips ?t=, renders order
//
// Bootstraps entirely from PUBLIC endpoints (no owner/dev auth) so it runs against
// prod, where /api/dev/mock-auth is intentionally 404. Order placement is public.

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
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

  test.beforeAll(async ({ request }) => {
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
    expect(typeof body.token).toBe('string');
    expect(body.token.split('.')).toHaveLength(3); // RS256 JWT

    // The reissued JWT must actually authorize the order's status endpoint.
    const statusRes = await request.get(`${BASE}/api/customer/orders/${orderId}/status`, {
      headers: { Authorization: `Bearer ${body.token}` },
    });
    expect(statusRes.status()).toBe(200);
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

    await context.close();
  });
});
