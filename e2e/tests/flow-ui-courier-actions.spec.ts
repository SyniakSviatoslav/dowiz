import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';
import { expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

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
    requireStaging(BASE); // mutating spec: never create couriers/orders against prod
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
    expect(invRes.status()).toBe(200);
    const invBody = await invRes.json();
    const inviteId = invBody.inviteId;
    const code = invBody.code;

    const redeemBody2 = await (await request.post(`${BASE}/api/courier/auth/invites/${inviteId}/redeem`, {
      data: { full_name: 'UI Courier 2', email: COURIER_EMAIL, password: COURIER_PASSWORD, code },
    })).json();
    courierId = redeemBody2.courier?.id;
    expectUuid(courierId, 'courierId'); // setup gate: a failed redeem must not silently yield undefined

    const loginRes = await request.post(`${BASE}/api/courier/auth/login`, {
      data: { email: COURIER_EMAIL, password: COURIER_PASSWORD },
    });
    expect(loginRes.status()).toBe(200);
    const loginBody = await loginRes.json();
    courierToken = loginBody.jwt;

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

    const confirmRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/orders/${orderId}/confirm`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(confirmRes.status()).toBe(200); // setup gate: an unconfirmed order can't be assigned
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
    await page.goto(`${BASE}/courier`, { waitUntil: 'load', timeout: 30000 });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Specific render proof: the Tasks page h1 (locale-tolerant) — a login redirect / error shell
    // / blank spinner has no such heading and must fail this assertion.
    await expect(
      page.getByRole('heading', { level: 1, name: /Tasks|Detyrat|Завдання/i })
    ).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Delivery page loads with map and actions', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier/delivery/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Order-specific proof: the assigned drop-off address this order was created with.
    // A loose word-regex passed on any nav bar / 404 / not-found shell; this asserts the
    // CORRECT task loaded for THIS courier (PII rendered only to the assigned courier).
    await expect(
      page.getByText('Rruga e Barrikadave', { exact: false })
    ).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Delivery page has cash collection input', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier/delivery/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Cash-collection UI: the cash-amount card renders for a cash order's task.
    // TODO(needs-staging): the editable number input is gated on picked_up state and the card on
    // ca.cash_amount being populated — verify against a live staged cash assignment.
    await expect(
      page.locator('[data-testid=task-cash-amount]')
    ).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Delivery page has complete/action button', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier/delivery/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const actionBtn = page.locator('button, a').filter({ hasText: /pick.?up|complete|deliver|Picked|Collected|Marr|Kompleto/i }).first();
    const hasAction = await actionBtn.isVisible({ timeout: 15000 }).catch(() => false);
    expect(hasAction, 'delivery page must expose a pickup/complete action button').toBe(true);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Shift page shows timer and controls', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier/shift`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await expect(
      page.getByRole('heading', { level: 1, name: /Shift|Turni|Зміна/i })
    ).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Earnings page shows summary', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier/earnings`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await expect(
      page.getByRole('heading', { level: 1, name: /Earnings|Fitimet|Заробіток/i })
    ).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('History page shows deliveries', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier/history`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await expect(
      page.getByRole('heading', { level: 1, name: /History|Historiku|Історія/i })
    ).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Isolation: a second (unassigned) courier cannot see the assigned task', async ({ page, request }) => {
    // IDOR control: a different courier at the same location must NOT load this order's task.
    // Server scopes GET /courier/assignments/:id by courier_id → 404 → "not found" soft state,
    // so the customer PII (drop-off address) must NOT render for the other courier.
    // TODO(needs-staging): requires a live staged second courier + assigned order to exercise.
    const ts2 = Date.now();
    const email2 = `courier-ui2-iso-${ts2}@test.com`;
    const inv = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { email: email2, role: 'courier' } }
    );
    expect(inv.status()).toBe(200);
    const { inviteId, code } = await inv.json();
    await request.post(`${BASE}/api/courier/auth/invites/${inviteId}/redeem`, {
      data: { full_name: 'UI Courier ISO', email: email2, password: COURIER_PASSWORD, code },
    });
    const login2 = await request.post(`${BASE}/api/courier/auth/login`, {
      data: { email: email2, password: COURIER_PASSWORD },
    });
    expect(login2.status()).toBe(200);
    const otherToken = (await login2.json()).jwt;
    expectUuid(orderId, 'orderId');

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), otherToken);
    await page.goto(`${BASE}/courier/delivery/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // The assigned courier's drop-off address must NOT leak to the unassigned courier.
    await expect(page.getByText('Rruga e Barrikadave', { exact: false })).toHaveCount(0);
  });

  test('No cookies on courier pages', async ({ page }) => {
    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), courierToken);
    await page.goto(`${BASE}/courier`, { waitUntil: 'networkidle' });
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
