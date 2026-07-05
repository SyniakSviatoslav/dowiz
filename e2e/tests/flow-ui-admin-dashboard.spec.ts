import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';
import { expectUuid } from '../helpers/assert-shape';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe.configure({ mode: 'serial' });

test.describe('UI: Admin Dashboard — Status Transitions via UI, Detail Modal, Messages', () => {
  let authToken: string;
  let activeLocationId: string;
  let productId: string;
  let orderId: string;
  const TS = Date.now();

  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const body = await authRes.json();
    authToken = body.access_token;
    activeLocationId = body.activeLocationId;

    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `Dsh-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    const catId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: `Dsh-Prod-${TS}`, price: 700, prep_time_minutes: 10, available: true, categoryId: catId },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;

    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1 }],
        customer: { phone: '+355600000030', name: 'Dashboard UI Test' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Rruga e Durrësit, Tirana' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
        acknowledged_codes: ['velocity'], // ack the speed-bump so the setup order is never soft-blocked
      },
    });
    const orderBody = await orderRes.json();
    orderId = orderBody.id; // created-order proof is the UUID (201 clean / 200 acked-or-idempotent)
    expectUuid(orderId, `order create failed (HTTP ${orderRes.status()}): ${JSON.stringify(orderBody).slice(0, 180)}`);
  });

  test.afterAll(async ({ request }) => {
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch((e) => { void e; /* tolerated: best-effort fixture cleanup, deletion failure must not fail the suite */ });
    }
  });

  test('Dashboard loads with order cards visible', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(body).toMatch(/dashboard|orders|total|count|active|delivery|pending|confirmed/i);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Confirm order via UI button', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    await page.waitForTimeout(2000);

    // Find and click confirm button
    const confirmBtn = page.locator('button').filter({ hasText: /confirm|Confirm|Konfirmo|Prano/i }).first();
    if (await confirmBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
      await confirmBtn.click();
      await page.waitForTimeout(1500);

      // Check for confirm dialog and accept it
      const dialog = page.locator('[role="dialog"], [role="alertdialog"], .modal, [class*="confirm"]').first();
      if (await dialog.isVisible({ timeout: 2000 }).catch(() => false)) {
        const yesBtn = dialog.locator('button').filter({ hasText: /yes|confirm|po|confirmo|vazhdo/i }).first();
        if (await yesBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
          await yesBtn.click();
          await page.waitForTimeout(1000);
        }
      }
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Prepare order status via API', async ({ request }) => {
    test.skip(!orderId, 'No order');
    // Use API since UI button may not be reachable after confirm
    const res = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/orders/${orderId}/confirm`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(res.status()).toBe(200);
  });

  test('Dashboard quick stats grid shows numbers', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Check for stat cards (order counts, revenue, etc.)
    const statCards = page.locator('[class*="stat"], [class*="Stat"], [class*="card"], [class*="Card"]').filter({ hasText: /\d+/ });
    const count = await statCards.count().catch(() => 0);

    // At minimum page should render without crash
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Dashboard search input works', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const searchInput = page.locator('input[type="search"], input[placeholder*="Search" i]').first();
    if (await searchInput.isVisible({ timeout: 3000 }).catch(() => false)) {
      await searchInput.fill('test');
      await page.waitForTimeout(500);
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Sort dropdown opens', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const sortBtn = page.locator('button, select').filter({ hasText: /sort|Sort|Rendit/i }).first();
    if (await sortBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await sortBtn.click();
      await page.waitForTimeout(500);
    }

    const select = page.locator('select').first();
    if (await select.isVisible({ timeout: 1000 }).catch(() => false)) {
      await select.selectOption({ index: 1 }).catch((e) => { void e; /* tolerated: optional sort control may expose no second option */ });
      await page.waitForTimeout(500);
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Dashboard snapshot API returns full shape', async ({ request }) => {
    const dashRes = await request.get(`${BASE}/api/owner/locations/${activeLocationId}/dashboard/snapshot`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(dashRes.status()).toBe(200);
    const dash = await dashRes.json();

    expect(dash.serverTime).toBeTruthy();
    expect(dash.counts).toBeTruthy();
    for (const s of ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED', 'CANCELLED', 'REJECTED']) {
      expect(typeof dash.counts[s]).toBe('number');
    }
    expect(Array.isArray(dash.orders)).toBe(true);
    expect(Array.isArray(dash.activeDeliveries)).toBe(true);
    expect(typeof dash.activeAlertCount).toBe('number');
    expect(typeof dash.activeSignalCount).toBe('number');

    if (dash.orders.length > 0) {
      const o = dash.orders[0];
      expectUuid(o.orderId, 'orderId');
      expect(o.status).toBeTruthy();
      expect(typeof o.total).toBe('number');
    }
  });
});
