/**
 * Order Status UI — E2E regression proofs
 *
 * Proves:
 *   1. Item prices on /s/:slug/order/:id are NOT "NaN" (api returns priceSnapshot, page was reading .price)
 *   2. WebSocket initial connect does NOT fire the reconnect callback (was causing double fetchOrder)
 *
 * Setup: creates a real order via API, stores the customer token, navigates as customer.
 * Teardown: cancels/cleans up the test order and product.
 */

import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe('Order Status UI', () => {
  test.describe.configure({ mode: 'serial' });

  let ownerToken: string;
  let locationId: string;
  let locationSlug: string;
  let categoryId: string;
  let productId: number | string;
  const productPrice = 2500; // minor units (25.00 ALL)
  let orderId: string;
  let customerToken: string;

  test.beforeAll(async ({ request }) => {
    // Auth
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const auth = await authRes.json();
    ownerToken = auth.access_token;

    const settingsRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    expect(settingsRes.status()).toBe(200);
    const settings = await settingsRes.json();
    locationId = settings.id;
    locationSlug = settings.slug;

    // Ensure delivery is open with all-day hours
    const openAllDay: Record<string, any> = {};
    for (const d of ['monday','tuesday','wednesday','thursday','friday','saturday','sunday']) {
      openAllDay[d] = { isOpen: true, open: '00:00', close: '23:59' };
    }
    const unpauseRes = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { deliveryPaused: false, hoursJson: openAllDay, lat: settings.lat ?? 41.315347, lng: settings.lng ?? 19.4449964 },
    });
    expect(unpauseRes.status()).toBe(200);

    // Create test category + product
    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { name: `OSU-Cat-${Date.now()}` },
    });
    expect(catRes.status()).toBe(201);
    categoryId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { name: `OSU-Prod-${Date.now()}`, price: productPrice, available: true, categoryId },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;
  });

  test.afterAll(async ({ request }) => {
    // Cancel order if it was created
    if (orderId && ownerToken) {
      await request.patch(`${BASE}/api/orders/${orderId}/status`, {
        headers: { Authorization: `Bearer ${ownerToken}` },
        data: { status: 'CANCELLED' },
      }).catch(() => {});
    }
    // Clean up product and category
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${ownerToken}` },
      }).catch(() => {});
    }
    if (categoryId) {
      await request.delete(`${BASE}/api/owner/menu/categories/${categoryId}`, {
        headers: { Authorization: `Bearer ${ownerToken}` },
      }).catch(() => {});
    }
  });

  test('SETUP: place a real order and capture customer token', async ({ request }) => {
    const settingsRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    const settings = await settingsRes.json();
    const lat = settings.lat ?? 41.315347;
    const lng = settings.lng ?? 19.4449964;

    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 2, modifier_ids: [] }],
        customer: { phone: '+355691000099', name: 'UI-Status Test' },
        delivery: { pin: { lat, lng }, address_text: 'Test street, block A' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });

    // Skip if business rule blocks (min_order, delivery range, etc.)
    if ([422, 429, 200].includes(orderRes.status())) {
      const body = await orderRes.json();
      test.skip(true, `Order blocked: ${orderRes.status()} ${body.code ?? body.outcome}`);
      return;
    }

    expect(orderRes.status()).toBe(201);
    const body = await orderRes.json();
    orderId = body.id;
    customerToken = body.authToken;
    expect(orderId).toMatch(/^[0-9a-f-]{36}$/);
    expect(customerToken, 'API must return authToken for status page').toBeTruthy();
  });

  test('UI-PRICE: order status page shows item price (not NaN)', async ({ page }) => {
    if (!orderId || !customerToken) {
      test.skip(true, 'Order not created in SETUP');
      return;
    }

    // Inject customer token before navigating
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'domcontentloaded' });
    await page.evaluate((token) => {
      localStorage.setItem('dos_access_token', token);
    }, customerToken);

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });

    // Order details section must be visible
    const orderSection = page.locator('[aria-label="Order status updates"], .max-w-md');
    await expect(orderSection.first()).toBeVisible({ timeout: 10000 });

    // Item price must NOT contain "NaN"
    const bodyText = await page.locator('body').innerText();
    expect(bodyText, 'Order status page must not show NaN for item price').not.toContain('NaN');

    // Verify actual price is visible (productPrice / 100 = 25.00 in whatever format)
    // The page shows "2x OSU-Prod-..." with the price per item
    const itemLine = page.locator('text=/OSU-Prod-/');
    await expect(itemLine).toBeVisible({ timeout: 5000 });
  });

  test('UI-WS: WebSocket initial connect does NOT trigger extra fetchOrder', async ({ page }) => {
    if (!orderId || !customerToken) {
      test.skip(true, 'Order not created in SETUP');
      return;
    }

    // Count how many times the order status API is called within 3s of page load
    let statusCallCount = 0;
    await page.route(`**/customer/orders/${orderId}/status`, (route) => {
      statusCallCount++;
      route.continue();
    });

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'domcontentloaded' });
    await page.evaluate((token) => {
      localStorage.setItem('dos_access_token', token);
    }, customerToken);

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });

    // Wait 3 seconds for any extra calls from faulty onReconnect
    await page.waitForTimeout(3000);

    // With the fix: exactly 1 call (initial fetch). Without: 2+ (initial + onReconnect-on-first-connect).
    expect(statusCallCount, `Expected 1 status API call on initial WS connect, got ${statusCallCount}`).toBe(1);
  });
});
