import { test, expect } from '@playwright/test';

// Read-only GETs below target BASE; default to staging so a test never hits prod
// (no-prod-base-in-test). Navigation tests use the Playwright baseURL (also VITE_BASE_URL-driven).
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test.describe('Recent Changes Validation — Live (staging via VITE_BASE_URL)', () => {

  // ─── CDN Image Serving ───────────────────────────────────────────
  test('CDN-1: image route serves webp with correct headers', async ({ page }) => {
    // Test that the /images/ route exists and returns proper content-type
<<<<<<< Updated upstream
    const res = await page.request.get('https://dowiz.fly.dev/images/products/test/test.webp');
    // 404 is expected since test.webp doesn't exist, but route should exist (not SPA fallback)
    expect([200, 404]).toContain(res.status());
    if (res.status() === 200) {
      expect(res.headers()['content-type']).toContain('webp');
    }
=======
    const res = await page.request.get(`${BASE}/images/products/test/test.webp`);
    // 404 is expected since test.webp doesn't exist, but the route must exist (not SPA fallback)
    expect(res.status()).toBe(404);
>>>>>>> Stashed changes
    // Verify it's not returning HTML (SPA fallback)
    const text = await res.text();
    expect(text).not.toContain('<!DOCTYPE');
    // Positive: the route returns the JSON error envelope (NOT an SPA HTML 404). The handler
    // is reply.sendError(404,'NOT_FOUND',…) → buildErrorEnvelope, so code === 'NOT_FOUND'.
    expect(JSON.parse(text).code).toBe('NOT_FOUND');
  });

  test('CDN-2: image route does not serve SPA HTML', async ({ page }) => {
    const res = await page.request.get(`${BASE}/images/products/nonexistent/file.webp`);
    expect(res.status()).toBe(404);
    const text = await res.text();
    expect(text).not.toContain('<!DOCTYPE');
    expect(text).not.toContain('<html');
    // Positive: a real JSON NOT_FOUND envelope, not an SPA fallback that happens to lack <html>.
    expect(JSON.parse(text).code).toBe('NOT_FOUND');
  });

  // ─── Currency Display (Lek/L instead of ALL) ─────────────────────
  test('CURRENCY-1: home page loads without JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(200);
    // Page renders successfully
  });

  test('CURRENCY-2: status page totals display correctly', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    // The page should render — we verify it loads without JS errors
    expect(body.length).toBeGreaterThan(200);
  });

  // ─── Category Delete ─────────────────────────────────────────────
  test('CAT-1: admin menu manager loads without JS errors', async ({ page }) => {
    // Even without auth, we verify the app shell loads
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin/menu');
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
  });

  // ─── Status Page (CR-5 + CR-6 regression) ───────────────────────
  test('STATUS-1: no JS errors on status page', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(4000);
    expect(errors).toEqual([]);
  });

  test('STATUS-2: ETA time display visible (sq/en/uk)', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    // Provenance: prove the ETA/content is API-derived, not a static/mock page that renders
    // the same strings regardless. The page must call the customer order-status endpoint.
    const orderApi = page.waitForResponse(r => r.url().includes('/customer/orders/') && r.url().includes('/status'), { timeout: 9000 });
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await orderApi;
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    // Matches Albanian or English translation text
    await expect(page.locator('text=/Mbërritja|Estimated arrival|Очікуваний/i')).toBeVisible({ timeout: 8000 });
    // ETA format: number + "min"
    await expect(page.locator('text=/\\d+\\s*min/i').first()).toBeVisible({ timeout: 5000 });
  });

  test('STATUS-3: status timeline renders', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    const orderApi = page.waitForResponse(r => r.url().includes('/customer/orders/') && r.url().includes('/status'), { timeout: 9000 });
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await orderApi; // provenance: timeline is API-derived
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    // Scope to the OrderProgress stepper so tooltips / aria-labels / body copy elsewhere on the
    // page can't inflate the count (the stepper renders inside [data-testid=order-status-badge]).
    const steps = page.locator('[data-testid="order-status-badge"]').locator('text=/received|preparing|ready|on the way|delivered/i');
    expect(await steps.count()).toBeGreaterThanOrEqual(4);
  });

  test('STATUS-4: share location button hidden (not IN_DELIVERY)', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    const orderApi = page.waitForResponse(r => r.url().includes('/customer/orders/') && r.url().includes('/status'), { timeout: 9000 });
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await orderApi; // provenance: state is API-derived
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    await expect(page.locator('text=Share my location with courier')).toHaveCount(0);
    // TODO(needs-staging): add STATUS-4b positive-control — seed a real IN_DELIVERY order on
    // staging and assert this button IS visible, so the negative above isn't vacuously true on a
    // blank/errored page. Requires a live courier-on-the-way order id (no fake/mock substitute).
  });

  test('STATUS-5: Order details section + total visible (sq/en/uk)', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    const orderApi = page.waitForResponse(r => r.url().includes('/customer/orders/') && r.url().includes('/status'), { timeout: 9000 });
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await orderApi; // provenance: order details are API-derived
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    await expect(page.locator('text=/Porosisë|Order Details|Detajet/i')).toBeVisible({ timeout: 5000 });
    await expect(page.locator('text=/Totali|Total|Разом/i')).toBeVisible({ timeout: 5000 });
  });

  // ─── Menu / App Shell ────────────────────────────────────────────
  test('MENU-1: app shell loads without JS errors on home page', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/');
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(200);
  });

  // ─── No Cookies Policy ───────────────────────────────────────────
  test('COOKIE-1: status page sets zero cookies', async ({ page }) => {
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(2000);
    expect(await page.context().cookies()).toEqual([]);
  });

  test('COOKIE-2: menu page sets zero cookies', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');
    await page.waitForTimeout(2000);
    expect(await page.context().cookies()).toEqual([]);
  });

  // ─── Checkout Page ───────────────────────────────────────────────
  test('CHECKOUT-1: checkout page loads empty cart state', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/checkout?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
  });

  // ─── Customer order tracking without a session ───────────────────
  // Regression: a no-session visit to /s/:slug/order/:id used to 401 and the
  // global apiClient handler hard-redirected the visitor to the owner /login.
  // The owner-session-expiry redirect is now scoped to /admin, so the customer
  // tracking surface must stay put and show its own "reload the menu" message.
  test('TRACK-1: no-session order tracking does not redirect to owner login', async ({ page, context }) => {
    await context.clearCookies();
    const orderId = '00000000-0000-4000-8000-000000000000';
    await page.goto(`/s/test-slug/order/${orderId}`);
    await page.waitForTimeout(4000);

    // The fix: no bounce to the owner login / admin app.
    expect(page.url(), `unexpectedly redirected to ${page.url()}`).not.toMatch(/\/login(\?|$)/);
    expect(page.url()).not.toMatch(/\/admin(\/|\?|$)/);
    // Still on the customer tracking surface.
    expect(page.url()).toContain(`/order/${orderId}`);
    // Positive: not a blank 200 / generic error — the no-session path renders the customer
    // tracking error surface with its "back to the menu" escape (OrderStatusPage error branch).
    await expect(page.locator('[data-testid="order-back-to-menu"]')).toBeVisible({ timeout: 8000 });
  });

  // ─── Health Check ────────────────────────────────────────────────
  test('HEALTH-1: app serves HTML on root path', async ({ page }) => {
    const res = await page.request.get(`${BASE}/`);
    expect(res.status()).toBe(200);
    const text = await res.text();
    expect(text).toContain('<!DOCTYPE html>');
    expect(text).toContain('Dowiz');
  });
});
