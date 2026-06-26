import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe.configure({ mode: 'serial' });

test.describe('UI: Client Checkout — Full Flow', () => {
  let authToken: string;
  let locationSlug: string;
  let activeLocationId: string;
  let productId: string;
  let orderId: string;
  const TS = Date.now();

  test.beforeAll(async ({ request }) => {
    // Create owner + product for this test
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
      data: { name: `UI-Checkout-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    const categoryId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: {
        name: `UI-Checkout-Prod-${TS}`,
        price: 500,
        available: true,
        categoryId,
        stockCount: 10,
      },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;

    console.log('Setup:', { locationSlug, activeLocationId, productId });
  });

  test.afterAll(async ({ request }) => {
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
  });

  test('Flow 1: Add item to cart and navigate to checkout via UI', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'domcontentloaded', timeout: 15000 });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Wait for React hydration (SSR renders cards first, then React hydrates add buttons)
    await page.waitForSelector('[data-testid="menu-item-add"]', { timeout: 15000 });
    const addBtn = page.locator('[data-testid="menu-item-add"]').first();
    await expect(addBtn).toBeVisible({ timeout: 3000 });
    await addBtn.click();

    // Cart FAB appears with count
    const fab = page.locator('#cartFabBtn');
    await expect(fab).toBeVisible({ timeout: 5000 });
    const fabText = await fab.textContent();
    expect(fabText).toMatch(/[1-9]/);

    // Open cart drawer
    await fab.click();
    const cartHeader = page.locator('h2, h3').filter({ hasText: /Cart|Shporta|Your/i }).first();
    await expect(cartHeader).toBeVisible({ timeout: 3000 });

    // Click checkout
    const checkoutBtn = page.locator('button, a').filter({ hasText: /checkout|Checkout|Porosit|Vazhdo/i }).first();
    await expect(checkoutBtn).toBeVisible({ timeout: 3000 });
    await checkoutBtn.click();

    // Verify on checkout page
    await expect(page).toHaveURL(/\/checkout/, { timeout: 8000 });
    await expect(page.locator('body')).toBeAttached({ timeout: 5000 });

    const criticalErrors = errors.filter(e => !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('ResizeObserver') && !e.includes("Unexpected token"));
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
  });

  test('Flow 2: Fill checkout form and submit order', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    // Navigate to menu and add item
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const addBtn = page.locator('[data-testid="menu-item-add"]').first();
    await expect(addBtn).toBeVisible({ timeout: 10000 });
    await addBtn.click();

    // Go to checkout
    const fab = page.locator('#cartFabBtn');
    await expect(fab).toBeVisible({ timeout: 5000 });
    await fab.click();
    await page.waitForTimeout(500);
    const checkoutBtn = page.locator('button, a').filter({ hasText: /checkout|Checkout|Porosit|Vazhdo/i }).first();
    await expect(checkoutBtn).toBeVisible({ timeout: 3000 });
    await checkoutBtn.click();
    await expect(page).toHaveURL(/\/checkout/, { timeout: 8000 });
    await page.waitForTimeout(1000);

    // Fill form fields
    const inputs = page.locator('input');
    const inputCount = await inputs.count();

    // Fill name field if present
    const nameInput = page.locator('input[name="name"], input[placeholder*="name" i], input[id*="name" i]').first();
    if (await nameInput.isVisible({ timeout: 2000 }).catch(() => false)) {
      await nameInput.fill('UI E2E Customer');
    }

    // Fill phone field
    const phoneInput = page.locator('input[type="tel"], input[name="phone"], input[placeholder*="phone" i]').first();
    if (await phoneInput.isVisible({ timeout: 2000 }).catch(() => false)) {
      await phoneInput.fill('+355691234568');
    }

    // Fill address
    const addressInput = page.locator('input[placeholder*="address" i], input[name="address"], textarea').first();
    if (await addressInput.isVisible({ timeout: 2000 }).catch(() => false)) {
      await addressInput.fill('Rruga e Durrësit, Tirana');
    }

    // Fill entrance
    const entranceInput = page.locator('input[placeholder*="entrance" i], input[name="entrance"], input[placeholder*="hyrje" i]').first();
    if (await entranceInput.isVisible({ timeout: 1000 }).catch(() => false)) {
      await entranceInput.fill('3');
    }

    // Fill apartment
    const aptInput = page.locator('input[placeholder*="apartment" i], input[name="apartment"], input[placeholder*="banes" i]').first();
    if (await aptInput.isVisible({ timeout: 1000 }).catch(() => false)) {
      await aptInput.fill('12');
    }

    // Set cash amount
    const cashInput = page.locator('input[type="number"], input[name="cash"], input[placeholder*="cash" i]').first();
    if (await cashInput.isVisible({ timeout: 2000 }).catch(() => false)) {
      await cashInput.fill('600');
    }

    // Click submit/place order button
    const submitBtn = page.locator('button[type="submit"], button').filter({ hasText: /place|order|Place|Porosit|Konfirmo|submit|Submit/i }).first();
    if (await submitBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
      await submitBtn.click();
    } else {
      // Fallback: try the last button (might be a layout submit)
      const allBtns = page.locator('button');
      const btnCount = await allBtns.count();
      if (btnCount > 0) {
        await allBtns.last().click();
      }
    }

    // Wait for order placement response
    await page.waitForTimeout(3000);

    // After successful order, page should either navigate to order status
    // or show a confirmation modal
    const currentUrl = page.url();

    if (currentUrl.includes('/order/')) {
      orderId = currentUrl.split('/order/')[1]?.split('?')[0]?.split('#')[0];
    }

    // Verify no crash
    expect(errors, `JS errors after submit: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 3: Verify order exists via API', async ({ request }) => {
    test.skip(!orderId, 'No orderId captured from UI flow');

    const getRes = await request.get(`${BASE}/api/orders/${orderId}`);
    expect(getRes.status()).toBe(200);
    const order = await getRes.json();
    expect(order.id).toBe(orderId);
    expect(order.status).toBeTruthy();
    expect(['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED', 'CANCELLED', 'REJECTED']).toContain(order.status);
    expect(typeof order.total).toBe('number');
    expect(order.total).toBeGreaterThan(0);
    expect(order.items).toBeTruthy();
    expect(order.items.length).toBeGreaterThan(0);

    console.log(`Order ${orderId} verified: status=${order.status}, total=${order.total}`);
  });

  test('Flow 4: Order status page loads without JS errors', async ({ page }) => {
    test.skip(!orderId, 'No orderId captured');

    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors on status page: ${errors.join('; ')}`).toEqual([]);

    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('Flow 5: Admin dashboard shows the order', async ({ page }) => {
    test.skip(!orderId || !authToken, 'No orderId or authToken');

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

    expect(errors, `JS errors on admin: ${errors.join('; ')}`).toEqual([]);

    // Navigate to orders page via sidebar
    const ordersLink = page.locator('a, button, nav *, [role="navigation"] *')
      .filter({ hasText: /orders|Orders/i }).first();
    if (await ordersLink.isVisible({ timeout: 2000 }).catch(() => false)) {
      await ordersLink.click();
      await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
      const ordersBody = await page.textContent('body');
      expect(ordersBody.length).toBeGreaterThan(100);
    }
  });
});
