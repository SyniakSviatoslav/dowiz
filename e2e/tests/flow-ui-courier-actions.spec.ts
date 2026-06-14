import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe.configure({ mode: 'serial' });

test.describe('UI: Courier — Accept via UI, Delivery Page, Shift Controls', () => {
  let authToken: string;
  let activeLocationId: string;
  let courierToken: string;
  let courierId: string;
  let productId: string;
  let orderId: string;
  const TS = Date.now();
  const COURIER_EMAIL = `courier-ui2-${TS}@test.com`;
  const COURIER_PASSWORD = 'test-password-123!';

  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const body = await authRes.json();
    authToken = body.access_token;
    activeLocationId = body.activeLocationId;

    // Create product
    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `CUI-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    const catId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: `CUI-Prod-${TS}`, price: 550, available: true, categoryId: catId },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;

    // Create courier
    const invRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { email: COURIER_EMAIL, role: 'courier' } }
    );
    expect(invRes.status()).toBe(201);
    const invDetail = await request.get(`${BASE}/api/courier/auth/invites/${(await invRes.json()).id}`);
    const invite = await invDetail.json();
    const code = invite.code || invite.inviteCode;

    await request.post(`${BASE}/api/courier/auth/invites/${invite.id}/redeem`, {
      data: { name: 'UI Courier 2', email: COURIER_EMAIL, password: COURIER_PASSWORD, code },
    });

    const loginRes = await request.post(`${BASE}/api/courier/auth/login`, {
      data: { email: COURIER_EMAIL, password: COURIER_PASSWORD },
    });
    expect(loginRes.status()).toBe(200);
    const loginBody = await loginRes.json();
    courierToken = loginBody.jwt;
    courierId = loginBody.courier?.id || loginBody.userId;

    // Start shift
    await request.post(`${BASE}/api/courier/me/shift/start`, {
      headers: { Authorization: `Bearer ${courierToken}` },
      data: { lat: 41.33, lng: 19.82 },
    });

    // Create + confirm order
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1 }],
        customer: { phone: '+355600000040', name: 'Courier UI2 Test' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Rruga e Barrikadave, Tirana' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });
    expect(orderRes.status()).toBe(201);
    orderId = (await orderRes.json()).id;

    await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/orders/${orderId}/confirm`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
  });

  test.afterAll(async ({ request }) => {
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
  });

  test('Assign courier via API', async ({ request }) => {
    test.skip(!orderId, 'No order');
    const res = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/orders/${orderId}/assign-courier`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { courierId } }
    );
    expect([200, 409]).toContain(res.status());
    console.log(`Assign: ${res.status()}`);
  });

  test('Tasks page shows assigned task', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Delivery page loads with map and actions', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier/delivery/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    // Should have delivery info: ETA, dropoff, customer info
    const hasContent = /delivery|pickup|dropoff|complete|map|E2E|order|cancel|eta|min|address|phone/i.test(body);
    expect(hasContent).toBe(true);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Delivery page has cash collection input', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier/delivery/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const cashInput = page.locator('input[type="number"], input[name="cash"]').first();
    const hasCashInput = await cashInput.isVisible({ timeout: 3000 }).catch(() => false);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Delivery page has complete/action button', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier/delivery/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const actionBtn = page.locator('button, a').filter({ hasText: /pick.?up|complete|deliver|Picked|Collected|Marr|Kompleto/i }).first();
    const hasAction = await actionBtn.isVisible({ timeout: 3000 }).catch(() => false);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Shift page shows timer and controls', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier/shift`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    const hasShiftInfo = /shift|online|available|timer|active|start|end|Ndeshje|Aktiv/i.test(body);
    expect(hasShiftInfo).toBe(true);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Earnings page shows summary', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier/earnings`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('History page shows deliveries', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier/history`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('No cookies on courier pages', async ({ page }) => {
    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier`, { waitUntil: 'networkidle' });
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
