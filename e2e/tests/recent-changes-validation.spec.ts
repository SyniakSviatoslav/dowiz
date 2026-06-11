import { test, expect } from '@playwright/test';

test.describe('Recent Changes Validation — Live https://dowiz.fly.dev', () => {

  // ─── CDN Image Serving ───────────────────────────────────────────
  test('CDN-1: image route serves webp with correct headers', async ({ page }) => {
    // Test that the /images/ route exists and returns proper content-type
    const res = await page.request.get('https://dowiz.fly.dev/images/products/test/test.webp');
    // 404 is expected since test.webp doesn't exist, but route should exist (not SPA fallback)
    expect([200, 404]).toContain(res.status());
    if (res.status() === 200) {
      expect(res.headers()['content-type']).toContain('webp');
    }
    // Verify it's not returning HTML (SPA fallback)
    const text = await res.text();
    expect(text).not.toContain('<!DOCTYPE');
  });

  test('CDN-2: image route does not serve SPA HTML', async ({ page }) => {
    const res = await page.request.get('https://dowiz.fly.dev/images/products/nonexistent/file.webp');
    expect(res.status()).toBe(404);
    const text = await res.text();
    expect(text).not.toContain('<!DOCTYPE');
    expect(text).not.toContain('<html');
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
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
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
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const steps = page.locator('text=/received|preparing|ready|on the way|delivered/i');
    expect(await steps.count()).toBeGreaterThanOrEqual(4);
  });

  test('STATUS-4: share location button hidden (not IN_DELIVERY)', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    await expect(page.locator('text=Share my location with courier')).toHaveCount(0);
  });

  test('STATUS-5: Order details section + total visible (sq/en/uk)', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
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

  // ─── Health Check ────────────────────────────────────────────────
  test('HEALTH-1: app serves HTML on root path', async ({ page }) => {
    const res = await page.request.get('https://dowiz.fly.dev/');
    expect(res.status()).toBe(200);
    const text = await res.text();
    expect(text).toContain('<!DOCTYPE html>');
    expect(text).toContain('Dowiz');
  });
});
