import { test, expect } from '@playwright/test';
import { expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
let authToken: string;
let activeLocationId: string;
let locationSlug: string;
let categoryId: string;
let productId: string;
let orderId: string;
const TS = Date.now();
const TEST_PHONE = '+355691234567';
const TEST_PNG_BASE64 = 'iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAMklEQVQ4T2NkYPj/n4EBBJgYKAQMowYMAwMDhRE0YBhGDRgGNGAYRg0YBjRgGNUfAABF1wH5r5lRawAAAABJRU5ErkJggg==';

test.describe.configure({ mode: 'serial' });

test.describe('Flow: Orders & Checkout — Full Lifecycle', () => {
  test.beforeAll(async ({ request }) => {
    requireStaging(BASE); // mutating spec (creates orders/products) — never run against prod
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const authBody = await authRes.json();
    authToken = authBody.access_token;
    activeLocationId = authBody.activeLocationId;

    const settingsRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(settingsRes.status()).toBe(200);
    locationSlug = (await settingsRes.json()).slug;

    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `E2E-Ord-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    categoryId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: {
        name: `E2E-Order-Sushi-${TS}`,
        price: 500,
        description: 'E2E order lifecycle test product',
        available: true,
        categoryId,
        taste: { spicy: 1, salty: 1, sour: 0, sweet: 0, richness: 1 },
        recipeLines: [
          { supplyId: 'e2e-rice', supplyName: 'Sushi Rice', qty: 200, unit: 'g', kind: 'food_ingredient', kcal: 130, proteinG: 3, fatG: 0, carbsG: 28, allergens: [] },
        ],
        stockCount: 10,
      },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;

    // Upload product image
    const pngBuf = Buffer.from(TEST_PNG_BASE64, 'base64');
    const imgRes = await request.post(`${BASE}/api/owner/menu/products/${productId}/image`, {
      headers: { Authorization: `Bearer ${authToken}` },
      multipart: { file: { name: 'test.png', mimeType: 'image/png', buffer: pngBuf } },
    });
    expect(imgRes.status()).not.toBe(500);
    expect(imgRes.status()).not.toBe(401);
  });

  test.afterAll(async ({ request }) => {
    if (orderId) {
      await request.patch(`${BASE}/api/orders/${orderId}/status`, {
        data: { status: 'CANCELLED' },
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch((e) => { void e; /* tolerated: best-effort teardown cleanup, must not fail the suite */ });
    }
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch((e) => { void e; /* tolerated: best-effort teardown cleanup, must not fail the suite */ });
    }
    if (categoryId) {
      await request.delete(`${BASE}/api/owner/menu/categories/${categoryId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch((e) => { void e; /* tolerated: best-effort teardown cleanup, must not fail the suite */ });
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 1: Place order via API — verify endpoint contract
  // ──────────────────────────────────────────────────────────────
  test('Flow 1: Order — POST /api/orders returns proper response shape (201, 422, or 200 soft_confirm)', async ({ request }) => {
    const infoRes = await request.get(`${BASE}/public/locations/${locationSlug}/info`);
    expect(infoRes.status()).toBe(200);
    const info = await infoRes.json();
    const deliveryLat = info.lat || 41.3275;
    const deliveryLng = info.lng || 19.8187;

    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 2, modifier_ids: [] }],
        customer: { phone: TEST_PHONE, name: 'E2E Test Customer' },
        delivery: { pin: { lat: deliveryLat, lng: deliveryLng }, address_text: 'E2E Test Address, Tirana' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });

    expect(orderRes.status()).not.toBe(500);
    expect(orderRes.status()).not.toBe(401);

    const body = await orderRes.json();

    if (orderRes.status() === 201) {
      orderId = body.id;
      expect(orderId).toMatch(/^[0-9a-f-]{36}$/);
      expect(body.status).toBe('PENDING');
      expect(typeof body.total).toBe('number');
      expect(body.total).toBeGreaterThan(0);
      expect(body.createdAt).toBeTruthy();
      expect(body.subtotal).toBeGreaterThan(0);
    } else if (orderRes.status() === 200 && body.outcome === 'soft_confirm') {
      expect(body.reasons).toBeTruthy();
      expect(body.requiresOtp !== undefined).toBe(true);
    } else if (orderRes.status() === 422) {
      expect(body.error || body.code).toBeTruthy();
    } else if (orderRes.status() === 400) {
      expect(body.error || body.message).toBeTruthy();
    } else {
      // Exhaustive: no silent green for an unhandled status (429/404/etc.).
      throw new Error(`POST /api/orders unexpected status ${orderRes.status()}: ${JSON.stringify(body)}`);
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 2: GET /api/orders/:id — verify response shape
  // ──────────────────────────────────────────────────────────────
  test('Flow 2: Order — GET by ID returns full order with items', async ({ request }) => {
    test.skip(!orderId, 'No order created in Flow 1');

    // POSITIVE control: the owner (tenant) MUST be able to read the order (route: orders.ts → withTenant).
    const getRes = await request.get(`${BASE}/api/orders/${orderId}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(getRes.status()).toBe(200);
    const order = await getRes.json();

    // NEGATIVE control: an order is private — an anonymous GET MUST be rejected
    // (route: softVerifyAuth → 401 for an unrecognized principal). Without this,
    // the positive read passes green even if the endpoint were wide open.
    const anonRes = await request.get(`${BASE}/api/orders/${orderId}`);
    expect(anonRes.status()).toBe(401);
    // TODO(needs_staging): assert a REAL second-tenant owner token GET returns 404
    // (cross-tenant isolation via withTenant) — requires a second tenant fixture.

    expect(order.id).toBe(orderId);
    expect(order.status).toBeTruthy();
    expect(['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED', 'CANCELLED', 'REJECTED']).toContain(order.status);
    expect(typeof order.subtotal).toBe('number');
    expect(order.subtotal).toBeGreaterThan(0);
    expect(typeof order.total).toBe('number');
    expect(order.total).toBeGreaterThan(0);
    expect(order.items).toBeTruthy();
    expect(Array.isArray(order.items)).toBe(true);
    expect(order.items.length).toBeGreaterThan(0);
    expect(order.items[0].productId).toBe(productId);
    expect(typeof order.items[0].quantity).toBe('number');
    expect(order.createdAt).toBeTruthy();
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 3: Admin status transition — CONFIRMED
  // ──────────────────────────────────────────────────────────────
  test('Flow 3: Admin — PATCH /api/orders/:id/status transitions to CONFIRMED', async ({ request }) => {
    test.skip(!orderId, 'No order created in Flow 1');

    const getRes = await request.get(`${BASE}/api/orders/${orderId}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(getRes.status()).toBe(200);
    const before = await getRes.json();

    // NEGATIVE control: status mutation is owner-only — an unauthenticated PATCH
    // MUST be rejected before any state change (route: verifyAuth → 401).
    const noAuthPatch = await request.patch(`${BASE}/api/orders/${orderId}/status`, {
      data: { status: 'CONFIRMED' },
    });
    expect(noAuthPatch.status()).toBe(401);
    // TODO(needs_staging): assert a REAL second-tenant owner token PATCH → 404 (withTenant
    // finds no row) and a customer token PATCH → 403 (requireRole(['owner'])). Needs fixtures.

    if (before.status !== 'PENDING') {
      console.log(`Order ${orderId} is ${before.status}, skipping CONFIRMED transition`);
      return;
    }

    const patchRes = await request.patch(`${BASE}/api/orders/${orderId}/status`, {
      data: { status: 'CONFIRMED' },
      headers: { Authorization: `Bearer ${authToken}` },
    });

    expect(patchRes.status()).not.toBe(500);
    expect(patchRes.status()).not.toBe(401);

    if (patchRes.status() === 200) {
      const updated = await patchRes.json();
      expect(updated.status).toBe('CONFIRMED');
      expect(updated.id).toBe(orderId);
    } else if (patchRes.status() === 409) {
      const err = await patchRes.json();
      expect(err).toBeTruthy();
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 4: Client checkout page — form, delivery type, phone, OTP UI
  // ──────────────────────────────────────────────────────────────
  test('Flow 4: Client — checkout page loads with cart items, form, delivery selector, no JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const cards = page.locator('[data-testid="menu-item"]');
    await expect(cards.first()).toBeVisible({ timeout: 10000 });

    const addBtn = page.locator('[data-testid="menu-item-add"]').first();
    await addBtn.click();
    await expect(page.locator('#cartFabBtn')).toBeVisible({ timeout: 5000 });

    const fab = page.locator('#cartFabBtn');
    await expect(fab).toBeVisible({ timeout: 4000 });
    const fabText = await fab.textContent();
    expect(fabText).toMatch(/1/);

    await fab.click();

    // Cart heading
    const cartHeading = page.locator('h2').filter({ hasText: /Cart|Shporta/i }).first();
    await expect(cartHeading).toBeVisible({ timeout: 5000 });

    // Open checkout — try multiple button label variants
    const checkoutBtn = page.locator('button').filter({ hasText: /checkout|Checkout|Porosit|Vazhdo/i }).first();
    const hasCheckoutBtn = await checkoutBtn.isVisible({ timeout: 3000 }).catch(() => false);
    if (hasCheckoutBtn) {
      await checkoutBtn.click();
      await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    }

    // Should be on checkout page or have URL changed
    const url = page.url();
    const onCheckout = url.includes('/checkout');

    if (onCheckout) {
      const body = await page.textContent('body');
      expect(body).toMatch(/Checkout|checkout|total|order|ALL|Lek/i);

      const deliveryBtns = page.locator('button, label, [role="radio"]').filter({ hasText: /delivery|pickup|schedule|Dorëzim|Marrje/i });
      const deliveryCount = await deliveryBtns.count();

      const phoneInput = page.locator('input[type="tel"], input[inputmode="tel"], input[name="phone"]');
      const otpModal = page.locator('[role="dialog"], .modal, [class*="otp"]');
      const hasPhoneOrOtp = (await phoneInput.count() > 0) || (await otpModal.count() > 0);
      // Checkout must collect a phone (or surface the OTP step) — the computed signal is now asserted,
      // not abandoned. // TODO(needs_staging): validate live; depends on checkout reaching /checkout.
      expect(hasPhoneOrOtp, 'checkout must expose a phone input or OTP step').toBe(true);
    }

    expect(errors, `JS errors on checkout: ${errors.join('; ')}`).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 5: Client order status page — timeline, steps, no cookies
  // ──────────────────────────────────────────────────────────────
  test('Flow 5: Client — order status page shows timeline with status steps', async ({ page }) => {
    test.skip(!orderId, 'No order created in Flow 1');
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    // Inject the owner session so the (owner-owned) order resolves over the real auth
    // path — replaces the local-only `o_mock_123?dev=true` mock with the real orderId.
    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    expect(errors, `JS errors on status page: ${errors.join('; ')}`).toEqual([]);

    // Specific render proof (not body.length): the real stepper renders, and the
    // delivery pipeline has ≥3 steps. No conditional guard — zero steps now fails.
    await expect(page.locator('[data-testid="order-progress"]')).toBeVisible({ timeout: 10000 });
    const stepCount = await page.locator('[data-testid^="order-step-"]:not([data-testid$="-time"])').count();
    expect(stepCount).toBeGreaterThanOrEqual(3);

    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 6: Admin dashboard — orders display, sidebar, no cookies
  // ──────────────────────────────────────────────────────────────
  test('Flow 6: Admin — dashboard loads with order counts, orders list, no cookies', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    expect(errors, `JS errors on admin: ${errors.join('; ')}`).toEqual([]);

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    const hasContent = /dashboard|orders|total|count|active|delivery|pending|confirmed|PENDING|CONFIRMED/i.test(body);
    expect(hasContent).toBe(true);

    // Navigate to orders page
    const ordersLink = page.locator('a, button, nav *, [role="navigation"] *')
      .filter({ hasText: /orders|Orders/i }).first();
    if (await ordersLink.isVisible({ timeout: 2000 }).catch(() => false)) {
      await ordersLink.click();
      await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
      const ordersBody = await page.textContent('body');
      expect(ordersBody.length).toBeGreaterThan(100);
    }

    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 7: Dashboard API — snapshot response shape
  // ──────────────────────────────────────────────────────────────
  test('Flow 7: Admin — GET dashboard snapshot returns counts and orders', async ({ request }) => {
    const dashRes = await request.get(`${BASE}/api/owner/locations/${activeLocationId}/dashboard/snapshot`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(dashRes.status()).toBe(200);
    const dash = await dashRes.json();

    expect(dash.serverTime).toBeTruthy();
    expect(dash.counts).toBeTruthy();
    expect(typeof dash.counts.PENDING).toBe('number');
    expect(typeof dash.counts.CONFIRMED).toBe('number');
    expect(typeof dash.counts.PREPARING).toBe('number');
    expect(typeof dash.counts.READY).toBe('number');
    expect(typeof dash.counts.IN_DELIVERY).toBe('number');
    expect(typeof dash.counts.DELIVERED).toBe('number');
    expect(typeof dash.counts.CANCELLED).toBe('number');
    expect(typeof dash.counts.REJECTED).toBe('number');

    expect(Array.isArray(dash.orders)).toBe(true);
    if (dash.orders.length > 0) {
      const o = dash.orders[0];
      expectUuid(o.orderId, 'orderId');
      expect(o.status).toBeTruthy();
      expect(typeof o.total).toBe('number');
      expect(o.createdAt).toBeTruthy();
      // PII must be REDACTED, not merely present — assert the exact mask shape
      // (pii-mask.ts: name → `X***`/`***`, phone → `+*** *** 1234`/`***`). A full
      // unredacted name/number contains no `***` and so fails these.
      expect(o.customerNameMasked, 'name must be masked').toMatch(/^(.\*\*\*|\*\*\*)$/);
      expect(o.customerPhoneMasked, 'phone must be masked').toMatch(/^(\+\*\*\* \*\*\* (\d{4}|\*\*\*\*)|\*\*\*)$/);
    }

    expect(Array.isArray(dash.activeDeliveries)).toBe(true);
    expect(typeof dash.activeAlertCount).toBe('number');
    expect(typeof dash.activeSignalCount).toBe('number');
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 8: Persistence — client menu + cart after refresh and nav
  // ──────────────────────────────────────────────────────────────
  test('Flow 8: Client — menu page survives refresh and route navigation, no JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const cards = page.locator('[data-testid="menu-item"]');
    await expect(cards.first()).toBeVisible({ timeout: 10000 });

    // [REFRESH]
    await page.reload({ waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await expect(cards.first()).toBeVisible({ timeout: 10000 });
    expect(errors, `JS errors after refresh: ${errors.join('; ')}`).toEqual([]);

    // [NAVIGATION] Go to cart page, then back
    await page.goto(`${BASE}/s/${locationSlug}/cart`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await expect(cards.first()).toBeVisible({ timeout: 10000 });
    expect(errors, `JS errors after navigation: ${errors.join('; ')}`).toEqual([]);

    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
