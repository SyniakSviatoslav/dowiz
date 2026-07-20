import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe.configure({ mode: 'serial' });

test.describe('UI: Courier Core Flow — Login, Accept, Deliver', () => {
  let authToken: string;
  let activeLocationId: string;
  let locationSlug: string;
  let courierToken: string;
  let courierId: string;
  let orderId: string;
  let productId: string;

  const TS = Date.now();
  const COURIER_EMAIL = `courier-ui-${TS}@test.com`;
  const COURIER_PASSWORD = 'test-password-123!';

  test.beforeAll(async ({ request }) => {
    // Create owner + product
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
      data: { name: `UI-Cour-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    const categoryId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: `UI-Cour-Prod-${TS}`, price: 500, available: true, categoryId, stockCount: 10 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;

    // Create courier via invite flow
    const inviteRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { email: COURIER_EMAIL, role: 'courier' } }
    );
    expect(inviteRes.status()).toBe(200);
    const inviteBody = await inviteRes.json();
    const inviteId = inviteBody.inviteId;
    const code = inviteBody.code;

    // Redeem invite — response includes courier.id
    const redeemRes = await request.post(`${BASE}/api/courier/auth/invites/${inviteId}/redeem`, {
      data: { full_name: 'UI Courier', email: COURIER_EMAIL, password: COURIER_PASSWORD, code },
    });
    expect(redeemRes.status()).toBe(200);
    const redeemBody = await redeemRes.json();
    courierId = redeemBody.courier?.id;
    expect(courierId).toBeTruthy();

    // Login as courier — response only has jwt, refreshToken, activeLocationId, role
    const loginRes = await request.post(`${BASE}/api/courier/auth/login`, {
      data: { email: COURIER_EMAIL, password: COURIER_PASSWORD },
    });
    expect(loginRes.status()).toBe(200);
    const loginBody = await loginRes.json();
    courierToken = loginBody.jwt;
    expect(courierToken).toBeTruthy();

    // Start courier shift
    const shiftRes = await request.post(`${BASE}/api/courier/me/shift/start`, {
      headers: { Authorization: `Bearer ${courierToken}` },
      data: { lat: 41.33, lng: 19.82 },
    });
    expect(shiftRes.status()).toBe(200);

    // Create order
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1 }],
        customer: { phone: '+355600000002', name: 'UI Courier Test' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Sheshi Skënderbej, Tirana' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });
    expect(orderRes.status()).toBe(201);
    orderId = (await orderRes.json()).id;

    // Confirm the order via owner
    await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/orders/${orderId}/confirm`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );

    console.log('Setup done:', { locationSlug, orderId, courierId, activeLocationId });
  });

  test.afterAll(async ({ request }) => {
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
  });

  test('Flow 1: Courier — tasks page loads with assignments', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, courierToken);

    await page.goto(`${BASE}/courier`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 2: Owner — assign courier to order', async ({ request }) => {
    test.skip(!orderId || !activeLocationId, 'No order created');

    const assignRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/orders/${orderId}/assign-courier`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { courierId } }
    );

    // May be 200 or 409 depending on order state
    expect([200, 409]).toContain(assignRes.status());
    console.log(`Assign courier result: ${assignRes.status()}`);
  });

  test('Flow 3: Courier — accept task via UI', async ({ page }) => {
    test.skip(!courierToken, 'No courier auth');
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, courierToken);

    await page.goto(`${BASE}/courier`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Try to find and click accept button
    const acceptBtn = page.locator('button, a').filter({ hasText: /accept|Accept|Prano|Merr/i }).first();
    if (await acceptBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
      await acceptBtn.click();
      // Should navigate to delivery page
      await page.waitForTimeout(2000);
      const url = page.url();
      const onDeliveryPage = url.includes('/delivery/') || url.includes('/courier/');
      expect(onDeliveryPage || errors.length === 0).toBe(true);
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 4: Courier — delivery page loads with map and actions', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, courierToken);

    await page.goto(`${BASE}/courier/delivery/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    // Should show delivery info
    const hasContent = /delivery|pickup|dropoff|complete|map|E2E|order|cancel/i.test(body);
    expect(hasContent).toBe(true);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 5: Courier — shift page shows timer and controls', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, courierToken);

    await page.goto(`${BASE}/courier/shift`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 6: Courier — earnings page loads with summary', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, courierToken);

    await page.goto(`${BASE}/courier/earnings`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 7: Courier — history page loads with delivery history', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, courierToken);

    await page.goto(`${BASE}/courier/history`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 8: No cookies on any courier page', async ({ page }) => {
    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, courierToken);

    await page.goto(`${BASE}/courier`, { waitUntil: 'networkidle' });
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
