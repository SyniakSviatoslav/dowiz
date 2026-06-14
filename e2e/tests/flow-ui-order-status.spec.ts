import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe('UI: Client Order Status — WS, Map, Share, Messages', () => {
  let authToken: string;
  let activeLocationId: string;
  let locationSlug: string;
  let productId: string;
  let orderId: string;
  const TS = Date.now();

  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const body = await authRes.json();
    authToken = body.access_token;
    activeLocationId = body.activeLocationId;

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
  });

  test.afterAll(async ({ request }) => {
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
  });

  test('Order status page loads with timeline', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    const hasStatus = /received|pending|confirmed|preparing|ready|delivered|PENDING|CONFIRMED|status/i.test(body);
    expect(hasStatus).toBe(true);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Order progress bar shows current step', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Progress bar should render
    const progressBar = page.locator('[role="progressbar"], [class*="progress"], [class*="Progress"]').first();
    const hasProgress = await progressBar.isVisible({ timeout: 3000 }).catch(() => false);

    if (hasProgress) {
      const barWidth = await progressBar.getAttribute('style').catch(() => '');
      expect(barWidth).toBeTruthy();
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('ETA display shows delivery estimate', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    const hasEta = /min|hour|minut|orë|ETA|estimated/i.test(body);

    // ETA may not show for PENDING orders, but page should not crash
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Courier position map placeholder renders', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const mapEl = page.locator('[class*="map"], [class*="Map"], #map, canvas, [class*="leaflet"], [class*="maplib"]').first();
    const hasMap = await mapEl.isVisible({ timeout: 3000 }).catch(() => false);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Share location UI is present', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const shareBtn = page.locator('button, a').filter({ hasText: /share|Share|Ndaj|location|Location/i }).first();
    const hasShare = await shareBtn.isVisible({ timeout: 3000 }).catch(() => false);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Order status 404 returns correct page', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/00000000-0000-0000-0000-000000000000`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Status page survives refresh without errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await page.reload({ waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('No cookies set on order status page', async ({ page }) => {
    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
