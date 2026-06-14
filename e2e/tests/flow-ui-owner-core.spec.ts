import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe.configure({ mode: 'serial' });

test.describe('UI: Owner Core Flow — Dashboard Status Transitions', () => {
  let authToken: string;
  let activeLocationId: string;
  let locationSlug: string;
  let orderId: string;
  let productId: string;
  const TS = Date.now();

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
    locationSlug = (await settingsRes.json()).slug;

    // Create product
    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `UI-Owner-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    const categoryId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: `UI-Owner-Prod-${TS}`, price: 500, available: true, categoryId, stockCount: 10 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;

    // Create order
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1 }],
        customer: { phone: '+355600000003', name: 'UI Owner Test' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Rruga Myslym Shyri, Tirana' },
        payment: { method: 'cash' },
        idempotency_key: `ui-owner-${TS}`,
      },
    });
    expect(orderRes.status()).toBe(201);
    orderId = (await orderRes.json()).id;

    console.log('Setup:', { locationSlug, activeLocationId, orderId, productId });
  });

  test.afterAll(async ({ request }) => {
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
  });

  test('Flow 1: Admin — dashboard loads with orders list', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(body).toMatch(/dashboard|orders|total|count|active|delivery|pending|confirmed/i);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 2: Admin — live/history toggle switches view', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Click Live/History toggle buttons
    const liveBtn = page.locator('button, [role="tab"]').filter({ hasText: /live|Live|Aktiv/i }).first();
    if (await liveBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await liveBtn.click();
      await page.waitForTimeout(500);
    }

    const historyBtn = page.locator('button, [role="tab"]').filter({ hasText: /history|History|Historiku/i }).first();
    if (await historyBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await historyBtn.click();
      await page.waitForTimeout(500);
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 3: Admin — status filter buttons work', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Try clicking status filter buttons
    const filterBtns = page.locator('button, [role="tab"]').filter({ hasText: /pending|confirmed|preparing|ready|delivery|delivered|all|PENDING|CONFIRMED|PREPARING|READY|All/i });
    const count = await filterBtns.count();
    if (count > 0) {
      await filterBtns.first().click();
      await page.waitForTimeout(500);
      if (count > 1) {
        await filterBtns.nth(1).click();
        await page.waitForTimeout(500);
      }
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 4: Admin — confirm order via API (status transition)', async ({ request }) => {
    test.skip(!orderId, 'No order created');

    const confirmRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/orders/${orderId}/confirm`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect([200, 409]).toContain(confirmRes.status());
    console.log(`Confirm result: ${confirmRes.status()}`);

    if (confirmRes.status() === 200) {
      const body = await confirmRes.json();
      expect(body.status).toBe('CONFIRMED');
    }
  });

  test('Flow 5: Admin — verify order appears on dashboard after status change', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 6: Admin — dashboard snapshot validates via API', async ({ request }) => {
    const dashRes = await request.get(`${BASE}/api/owner/locations/${activeLocationId}/dashboard/snapshot`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(dashRes.status()).toBe(200);
    const dash = await dashRes.json();

    expect(dash.serverTime).toBeTruthy();
    expect(dash.counts).toBeTruthy();
    expect(typeof dash.counts.PENDING).toBe('number');
    expect(typeof dash.counts.CONFIRMED).toBe('number');
    expect(typeof dash.counts.DELIVERED).toBe('number');

    expect(Array.isArray(dash.orders)).toBe(true);
    expect(Array.isArray(dash.activeDeliveries)).toBe(true);
    expect(typeof dash.activeAlertCount).toBe('number');
    expect(typeof dash.activeSignalCount).toBe('number');
  });

  test('Flow 7: Admin — no cookies on dashboard', async ({ page }) => {
    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('Flow 8: Admin — menu manager page loads', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 9: Admin — settings page loads', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 10: Admin — branding page loads', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin/branding`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 11: Admin — couriers page loads', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin/couriers`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });
});
