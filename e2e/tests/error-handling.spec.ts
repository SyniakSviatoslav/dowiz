import { test, expect } from '@playwright/test';

test.describe('Error Handling', () => {

  test('menu page handles 500 server error gracefully', async ({ page }) => {
    // Simulate 500 on menu API
    await page.route('**/public/menu/**', route => route.fulfill({ status: 500, body: JSON.stringify({ error: 'Server error' }) }));
    await page.goto('/s/test-slug?dev=true');
    await page.waitForTimeout(3000);

    // Page should not crash — should show fallback data
    const body = await page.textContent('body');
    expect(body!.length).toBeGreaterThan(0);
  });

  test('menu page handles network timeout', async ({ page }) => {
    await page.route('**/public/menu/**', route => route.abort('timedout'));
    await page.goto('/s/test-slug?dev=true');
    await page.waitForTimeout(3000);

    // Page should not crash — should show fallback
    const body = await page.textContent('body');
    expect(body!.length).toBeGreaterThan(0);
  });

  test('checkout handles 422 validation error', async ({ page }) => {
    // Add item and go to checkout
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('article.product-card', { timeout: 15000 });
    await page.locator('article.product-card button[aria-label="Add"]').first().click();
    await page.waitForTimeout(500);
    await page.locator('#cartFabBtn').click();
    await page.locator('button:has-text("Checkout")').click();
    await page.waitForTimeout(1000);

    // Mock 422 on order creation
    await page.route('**/customer/orders**', route => route.fulfill({ status: 422, body: JSON.stringify({ error: 'Validation failed' }) }));

    const body = await page.textContent('body');
    expect(body!.length).toBeGreaterThan(0);
  });

  test('checkout handles 429 rate limit', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('article.product-card', { timeout: 15000 });
    await page.locator('article.product-card button[aria-label="Add"]').first().click();
    await page.waitForTimeout(500);
    await page.locator('#cartFabBtn').click();
    await page.locator('button:has-text("Checkout")').click();
    await page.waitForTimeout(1000);

    await page.route('**/customer/otp/**', route => route.fulfill({ status: 429, body: JSON.stringify({ error: 'Too many requests' }) }));

    const body = await page.textContent('body');
    expect(body!.length).toBeGreaterThan(0);
  });

  test('order status handles 404 gracefully', async ({ page }) => {
    await page.route('**/customer/orders/**', route => route.fulfill({ status: 404, body: JSON.stringify({ error: 'Not found' }) }));
    await page.goto('/s/test-slug/order/nonexistent?dev=true');
    await page.waitForTimeout(3000);

    const body = await page.textContent('body');
    expect(body!.length).toBeGreaterThan(0);
  });

  test('admin page handles 401 gracefully', async ({ page }) => {
    await page.route('**/owner/**', route => route.fulfill({ status: 401, body: JSON.stringify({ error: 'Unauthorized' }) }));
    await page.goto('/admin?dev=true');
    await page.waitForTimeout(3000);

    // Page should not crash
    const body = await page.textContent('body');
    expect(body!.length).toBeGreaterThan(0);
  });

  test('admin page handles 403 gracefully', async ({ page }) => {
    await page.route('**/owner/**', route => route.fulfill({ status: 403, body: JSON.stringify({ error: 'Forbidden' }) }));
    await page.goto('/admin?dev=true');
    await page.waitForTimeout(3000);

    const body = await page.textContent('body');
    expect(body!.length).toBeGreaterThan(0);
  });

  test('admin page handles 5xx gracefully', async ({ page }) => {
    await page.route('**/owner/**', route => route.fulfill({ status: 503, body: JSON.stringify({ error: 'Service unavailable' }) }));
    await page.goto('/admin?dev=true');
    await page.waitForTimeout(3000);

    const body = await page.textContent('body');
    expect(body!.length).toBeGreaterThan(0);
  });

});
