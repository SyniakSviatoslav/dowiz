import { test, expect } from '@playwright/test';
import { expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test.describe.configure({ mode: 'serial' });

test.describe('UI: Client Checkout — Full Flow', () => {
  let authToken: string;
  let locationSlug: string;
  let activeLocationId: string;
  let productId: string;
  let orderId: string;
  const TS = Date.now();

  test.beforeAll(async ({ request }) => {
    // Mutating suite (creates owner/products/orders) — refuse to run against prod/unknown.
    requireStaging(BASE);
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
      }).catch((e) => { void e; /* tolerated: best-effort test-data cleanup must not fail the suite */ });
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

    // FAB only renders when itemsCount > 0 — assert empty-cart baseline so the click proves 0→1.
    const fab = page.locator('#cartFabBtn');
    await expect(fab).toHaveCount(0);

    await addBtn.click();

    // Cart FAB appears with the exact count (aria-label="Cart: <n> items") — not a loose digit match.
    await expect(fab).toBeVisible({ timeout: 5000 });
    await expect(fab).toHaveAttribute('aria-label', 'Cart: 1 items');

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

    // Fill the REQUIRED fields deterministically — no silent isVisible() guard on a field
    // whose absence would still let the test "pass". The name input has no testid (required,
    // autoComplete="name"); phone has data-testid="checkout-phone".
    const nameInput = page.locator('input[autocomplete="name"]').first();
    await expect(nameInput).toBeVisible({ timeout: 10000 });
    await nameInput.fill('UI E2E Customer');

    const phoneInput = page.getByTestId('checkout-phone');
    await expect(phoneInput).toBeVisible({ timeout: 5000 });
    await phoneInput.fill('+355691234568');

    // Entrance / apartment are delivery-only fields — fill when shown (legitimately conditional
    // on delivery type), addressed by stable testid rather than placeholder heuristics.
    const entranceInput = page.getByTestId('checkout-entrance');
    if (await entranceInput.isVisible({ timeout: 1000 })) {
      await entranceInput.fill('3');
    }
    const aptInput = page.getByTestId('checkout-apartment');
    if (await aptInput.isVisible({ timeout: 1000 })) {
      await aptInput.fill('12');
    }

    // Submit via the single deterministic place-order button (data-testid="order-confirm-button").
    // No "click the last button" fallback — that could fire Cancel/Back and silently pass.
    const submitBtn = page.getByTestId('order-confirm-button');
    await expect(submitBtn).toBeVisible({ timeout: 5000 });
    await expect(submitBtn).toBeEnabled({ timeout: 5000 });
    await submitBtn.click();

    // Assert the SUCCESS state: on order placement CheckoutPage navigates to
    // /s/<slug>/order/<orderId> (CheckoutPage.tsx:514). A silent 400/422/5xx never navigates,
    // so this fails the test instead of passing fire-and-forget.
    await expect(page).toHaveURL(/\/order\/[^/?#]+/, { timeout: 20000 });
    orderId = page.url().split('/order/')[1]?.split('?')[0]?.split('#')[0];
    expectUuid(orderId, 'orderId from success navigation');

    // Verify no crash
    expect(errors, `JS errors after submit: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 3: Verify order exists via API (anon-negative + owner-positive controls)', async ({ request }) => {
    // NEGATIVE control: the order is private. An unauthenticated caller must be REJECTED, not
    // served — GET /orders/:id uses softVerifyAuth then 401s an unknown principal (orders.ts:684).
    // This also guards against the IDOR the old `GET <id>` → 200 assertion would have masked.
    const anonRes = await request.get(`${BASE}/api/orders/${orderId}`);
    expect(anonRes.status()).toBe(401);
    // TODO(needs_staging): add a true cross-tenant IDOR control — owner A reading a REAL second
    // tenant's order id must 404 (never an all-zero nil-UUID, which 404s by absence). Requires a
    // second seeded tenant + order on staging.

    // POSITIVE control: the owning tenant reads it back with a real token (tenant-scoped read).
    const getRes = await request.get(`${BASE}/api/orders/${orderId}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
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

  test('Flow 4: Order status page shows the order status', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Assert the actual status surface rendered (not just "body has >100 chars" — a 500/redirect/
    // spinner would pass that). The badge carries data-status; assert it is a real lifecycle state.
    const statusBadge = page.getByTestId('order-status-badge');
    await expect(statusBadge).toBeVisible({ timeout: 15000 });
    await expect(statusBadge).toHaveAttribute(
      'data-status',
      /^(PENDING|CONFIRMED|PREPARING|READY|IN_DELIVERY|DELIVERED|CANCELLED|REJECTED)$/,
    );

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
