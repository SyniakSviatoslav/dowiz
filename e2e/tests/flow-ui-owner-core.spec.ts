import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test.describe.configure({ mode: 'serial' });

test.describe('UI: Owner Core Flow — Dashboard Status Transitions', () => {
  let authToken: string;
  let activeLocationId: string;
  let locationSlug: string;
  let orderId: string;
  let productId: string;
  const TS = Date.now();

  test.beforeAll(async ({ request }) => {
    // Mutating suite (creates category/product/order, confirms): never let it run against prod.
    requireStaging(BASE);

    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const authBody = await authRes.json();
    authToken = authBody.access_token;
    activeLocationId = authBody.activeLocationId;
    expectJwt(authToken, 'mock-auth access_token');
    expectUuid(activeLocationId, 'activeLocationId');

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
    expectUuid(categoryId, 'categoryId');
    expectUuid(productId, 'productId');

    // Create order
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1 }],
        customer: { phone: '+355600000003', name: 'UI Owner Test' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Rruga Myslym Shyri, Tirana' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });
    expect(orderRes.status()).toBe(201);
    orderId = (await orderRes.json()).id;
    expectUuid(orderId, 'orderId');

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
    // A 401 bounces to /login; assert we stayed in the authenticated admin shell.
    await expect(page).not.toHaveURL(/\/login/);
    // Specific authenticated dashboard landmark — a 500 page / login wall / spinner has no WS dot.
    await expect(page.locator('[data-testid="ws-status-dot"]')).toBeVisible({ timeout: 15000 });

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

    // The view-mode tablist (Live / History) must exist — don't silently skip if missing.
    const tabs = page.locator('[role="tablist"] [role="tab"]');
    await expect(tabs).toHaveCount(2);
    const liveTab = tabs.first();
    const historyTab = tabs.nth(1);

    await expect(historyTab).toBeVisible();
    await historyTab.click();
    // Toggle must actually switch the active view (aria-selected reflects viewMode state).
    await expect(historyTab).toHaveAttribute('aria-selected', 'true');

    await expect(liveTab).toBeVisible();
    await liveTab.click();
    await expect(liveTab).toHaveAttribute('aria-selected', 'true');

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

    // Status filter buttons carry aria-pressed; they MUST be present (not silently skipped).
    const filterBtns = page.locator('button[aria-pressed]');
    await expect(filterBtns.first()).toBeVisible();
    expect(await filterBtns.count()).toBeGreaterThan(1);
    // Clicking a filter must actually select it (aria-pressed flips true).
    await filterBtns.nth(1).click();
    await expect(filterBtns.nth(1)).toHaveAttribute('aria-pressed', 'true');

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
    await expect(page).not.toHaveURL(/\/login/);

    // The just-confirmed order must actually appear on the dashboard, and its card must
    // carry the new CONFIRMED status — not merely "the page rendered some bytes".
    const card = page.locator(`[data-testid="order-card-${orderId}"]`);
    await expect(card).toBeVisible({ timeout: 15000 });
    await expect(card).toHaveAttribute('data-status', 'CONFIRMED');

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

  test('Flow 7: Admin — auth is Bearer-only (no cookies, no-token → 401)', async ({ page, request }) => {
    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);

    // Absence of cookies isn't proof of a correct boundary: prove the protected resource
    // actually rejects a request with NO Authorization header (auth.ts:47 → 401).
    const noAuthRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/dashboard/snapshot`,
    );
    expect(noAuthRes.status()).toBe(401);

    // TODO(needs-staging): mock-auth only ever mints the single dev owner, so a TRUE
    // cross-tenant IDOR check (owner B's token must 403/404 on owner A's activeLocationId)
    // requires a real second seeded tenant — cannot be faked with a nil/random UUID.
  });

  test('Flow 8: Admin — menu manager page loads', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page).not.toHaveURL(/\/login/);
    // Menu-manager specific control rendered (not a 500/redirect satisfying body.length).
    await expect(page.locator('[data-testid="kitchen-busy-toggle"]')).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 9: Admin — settings page loads', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle' });
    await expect(page).not.toHaveURL(/\/login/);
    // Settings page renders its section heading (SettingsPage.tsx:363) — a login wall / 500 has none.
    await expect(page.getByRole('heading', { level: 2 }).first()).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 10: Admin — branding page loads', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin/branding`, { waitUntil: 'networkidle' });
    await expect(page).not.toHaveURL(/\/login/);
    // Branding page root testid (BrandingPage.tsx:277) — proves the real page, not an error wall.
    await expect(page.locator('[data-testid="branding-page"]')).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 11: Admin — couriers page loads', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin/couriers`, { waitUntil: 'networkidle' });
    await expect(page).not.toHaveURL(/\/login/);
    // Couriers page renders its section heading (CouriersPage.tsx:228) — absent on a 500/redirect.
    await expect(page.getByRole('heading', { level: 2 }).first()).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });
});
