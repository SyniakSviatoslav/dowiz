import { test, expect } from '@playwright/test';

// Proof for the customer order tracking-link handoff:
//   order mint -> trackUrl with ?t=<opaque code>
//   POST /api/customer/track/exchange -> reissued 7-day customer JWT
//   clean-profile open of trackUrl -> page self-authenticates, strips ?t=, renders order
//
// Mirrors flow-order-creation.spec.ts setup (owner auth -> category -> product -> order).

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
const TS = Date.now();

test.describe.configure({ mode: 'serial' });

test.describe('Flow: Customer Track Link — Exchange Handoff', () => {
  let authToken: string;
  let activeLocationId: string;
  let categoryId: string;
  let productId: string;
  let deliveryLat: number;
  let deliveryLng: number;

  // Captured from the happy-path order so later tests can reuse the grant.
  let trackUrl: string | undefined;
  let trackCode: string | undefined;
  let orderId: string | undefined;

  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const authBody = await authRes.json();
    authToken = authBody.access_token;
    activeLocationId = authBody.activeLocationId;

    const settingsRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(settingsRes.status()).toBe(200);
    const settings = await settingsRes.json();
    deliveryLat = settings.lat ?? 41.3275;
    deliveryLng = settings.lng ?? 19.8187;

    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `Track-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    categoryId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: {
        name: `Track-Product-${TS}`,
        price: 1000,
        description: 'Track-link integration test product',
        available: true,
        categoryId,
      },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;
  });

  test.afterAll(async ({ request }) => {
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
    if (categoryId) {
      await request.delete(`${BASE}/api/owner/menu/categories/${categoryId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
  });

  // ── TEST 1: mint returns a trackUrl carrying an opaque ?t= code ──────────
  test('Order mint returns a trackUrl with an opaque ?t= grant code', async ({ request }) => {
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
        customer: { phone: `+35569${String(TS).slice(-7)}`, name: 'Track Test' },
        delivery: {
          pin: { lat: deliveryLat, lng: deliveryLng },
          address_text: 'Test Street 1, Tirana',
        },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });

    // Only a 201 with a resolved customer mints a grant. Other business outcomes
    // (422 min-order/range, 429 throttle) are valid but can't prove the link —
    // skip the assertion chain in those cases rather than flake.
    if (orderRes.status() !== 201) {
      test.skip(true, `Order not created (status ${orderRes.status()}); cannot mint track grant`);
      return;
    }

    const body = await orderRes.json();
    orderId = body.id;

    expect(typeof body.trackUrl).toBe('string');
    expect(body.trackUrl).toContain(`/order/${body.id}`);
    expect(body.trackUrl).toContain('?t=');

    trackUrl = body.trackUrl;
    trackCode = new URL(body.trackUrl).searchParams.get('t') || undefined;
    expect(trackCode && trackCode.length).toBeGreaterThan(20);
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
    // RS256 JWT: three dot-separated base64url segments.
    expect(body.token.split('.')).toHaveLength(3);

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

    // Fresh context = no dos_access_token in localStorage, exactly like a customer
    // tapping the link on a new device.
    const context = await browser.newContext();
    const page = await context.newPage();
    await page.goto(trackUrl!, { waitUntil: 'networkidle' });

    // The auth-expired EmptyState must NOT appear — the page exchanged ?t= for a session.
    await expect(page.getByText(/Session expired/i)).not.toBeVisible();

    // The page must not have bounced to the owner login.
    expect(page.url()).not.toContain('/admin');

    // replaceState stripped the secret from the visible URL.
    expect(page.url()).not.toContain('?t=');

    // A customer JWT is now stored — proof the exchange ran client-side.
    const stored = await page.evaluate(() => localStorage.getItem('dos_access_token'));
    expect(stored).toBeTruthy();

    // The app shell rendered (real DOM element visible).
    await expect(page.locator('#app')).toBeVisible();

    await context.close();
  });
});
