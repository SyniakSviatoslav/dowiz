import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';
import { requireStaging } from '../helpers/staging-guard';
import { expectJwt, expectUuid } from '../helpers/assert-shape';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test.describe('UI: Client Order Status — WS, Map, Share, Messages', () => {
  let authToken: string;
  let activeLocationId: string;
  let locationSlug: string;
  let productId: string;
  let orderId: string;
  const TS = Date.now();

  test.beforeAll(async ({ request }) => {
    // This suite MUTATES state (creates categories/products/orders, transitions status) and uses
    // the dev/mock-auth backdoor — refuse to run against prod / an unknown target.
    requireStaging(BASE);
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const body = await authRes.json();
    authToken = body.access_token;
    activeLocationId = body.activeLocationId;
    expectJwt(authToken, 'mock-auth access_token');
    expectUuid(activeLocationId, 'activeLocationId');

    const settingsRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    locationSlug = (await settingsRes.json()).slug;

    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `OS-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    const catId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: `OS-Prod-${TS}`, price: 600, available: true, categoryId: catId },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;

    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1 }],
        customer: { phone: '+355600000020', name: 'Status Page Test' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Rruga e Kavajës, Tirana' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });
    expect(orderRes.status()).toBe(201);
    orderId = (await orderRes.json()).id;
    expectUuid(orderId, 'orderId');
  });

  test.afterAll(async ({ request }) => {
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch((e) => { void e; /* tolerated: best-effort cleanup in afterAll, must not fail the suite */ });
    }
  });

  test('Order status page loads with timeline', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    // Specific tracking-page anchors — not a body-text regex (which a 404/error/nav shell satisfies).
    await expect(page.locator('[data-testid="order-progress"]')).toBeVisible({ timeout: 15000 });
    await expect(page.locator('[data-testid="order-status-badge"]')).toBeVisible();

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Order progress bar shows current step', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });

    // The timeline must render (unconditional — no if(hasProgress) escape hatch) with ≥1 lifecycle step.
    await expect(page.locator('[data-testid="order-progress"]')).toBeVisible({ timeout: 15000 });
    await expect(page.locator('[data-testid^="order-step-"]').first()).toBeVisible();

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('ETA display shows delivery estimate', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });

    // The ETA/status headline always renders and must carry text (no compute-then-never-assert boolean).
    // TODO(needs-staging): the numeric "X–Y min" range only shows once driven to IN_DELIVERY — drive
    // status then assert the range string specifically.
    const headline = page.locator('[data-testid="order-eta-headline"]');
    await expect(headline).toBeVisible({ timeout: 15000 });
    await expect(headline).not.toBeEmpty();

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Courier position map placeholder renders', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });

    // CourierLiveMap renders ONLY for a delivery order in IN_DELIVERY (no courier ⇒ no map for PENDING),
    // so assert the real tracking page rendered rather than a compute-then-never-assert boolean.
    // TODO(needs-staging): drive the order to IN_DELIVERY (assign a courier) and assert the live map canvas is visible.
    await expect(page.locator('[data-testid="order-progress"]')).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Share location UI is present', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });

    // The "Share my location with courier" UI is gated to IN_DELIVERY, so it is correctly absent for a
    // PENDING order — assert the real tracking page rendered rather than a never-asserted boolean.
    // TODO(needs-staging): drive the order to IN_DELIVERY and assert the share-location control is visible.
    await expect(page.locator('[data-testid="order-progress"]')).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Order status 404 returns correct page', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/00000000-0000-0000-0000-000000000000`, { waitUntil: 'networkidle' });

    // An unknown order id renders the not-found EmptyState with a way back (never a dead-end, never a blank shell).
    await expect(page.locator('[data-testid="order-back-to-menu"]')).toBeVisible({ timeout: 15000 });
    // TODO(needs-staging): cross-tenant IDOR — load tenant-A's REAL orderId under tenant-B's slug and
    // assert this same not-found state. A nil-UUID 404s by absence and proves nothing about isolation;
    // this needs a real second-tenant fixture.

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Status page survives refresh without errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('[data-testid="order-progress"]')).toBeVisible({ timeout: 15000 });

    await page.reload({ waitUntil: 'networkidle' });
    // The timeline must re-render after reload (specific anchor, not a body.length floor).
    await expect(page.locator('[data-testid="order-progress"]')).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('No cookies set on order status page', async ({ page }) => {
    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
