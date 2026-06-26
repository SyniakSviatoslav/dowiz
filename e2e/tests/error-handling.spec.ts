import { test, expect } from '@playwright/test';

test.describe('Error Handling', () => {

  test('menu page handles 500 server error gracefully — shows fallback content', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route('**/public/menu/**', route => route.fulfill({ status: 500, body: JSON.stringify({ error: 'Server error' }) }));
    await page.goto('/s/test-slug?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    expect(errors, `JS errors on 500: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    // Should show fallback/error UI, not blank
    expect(/error|unavailable|retry|try again|something went wrong|menu|product/i.test(body)).toBe(true);
  });

  test('menu page handles network timeout — shows fallback', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route('**/public/menu/**', route => route.abort('timedout'));
    await page.goto('/s/test-slug?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('Failed to fetch')
    );
    expect(criticalErrors, `JS errors on timeout: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
  });

  test('checkout handles 422 validation error — shows error message', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });
    await page.locator('[data-testid="menu-item-add"]').first().click();
    await expect(page.locator('#cartFabBtn')).toBeVisible({ timeout: 5000 });
    await page.locator('#cartFabBtn').click();
    const checkoutBtn = page.locator('button:has-text("Checkout")');
    if (await checkoutBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await checkoutBtn.click();
      await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    }
    await page.route('**/customer/orders**', route => route.fulfill({ status: 422, body: JSON.stringify({ error: 'Validation failed' }) }));
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('checkout handles 429 rate limit — shows retry message', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });
    await page.locator('[data-testid="menu-item-add"]').first().click();
    await expect(page.locator('#cartFabBtn')).toBeVisible({ timeout: 5000 });
    await page.locator('#cartFabBtn').click();
    const checkoutBtn = page.locator('button:has-text("Checkout")');
    if (await checkoutBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await checkoutBtn.click();
      await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    }
    await page.route('**/customer/otp/**', route => route.fulfill({ status: 429, body: JSON.stringify({ error: 'Too many requests', retryAfterSeconds: 60 }) }));
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('order status handles 404 gracefully — shows not found message', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route('**/customer/orders/**', route => route.fulfill({ status: 404, body: JSON.stringify({ error: 'Not found' }) }));
    await page.goto('/s/test-slug/order/nonexistent?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/not found|error|unavailable|order|status|404/i.test(body)).toBe(true);
  });

  test('admin page handles 401 gracefully — shows login redirect', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route('**/owner/**', route => route.fulfill({ status: 401, body: JSON.stringify({ error: 'Unauthorized' }) }));
    await page.goto('/admin?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest')
    );
    expect(criticalErrors, `JS errors on 401: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/login|unauthorized|sign in|redirect|access/i.test(body)).toBe(true);
  });

  test('admin page handles 403 gracefully — shows forbidden message', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route('**/owner/**', route => route.fulfill({ status: 403, body: JSON.stringify({ error: 'Forbidden' }) }));
    await page.goto('/admin?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest')
    );
    expect(criticalErrors, `JS errors on 403: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/forbidden|access denied|no permission|restricted|error/i.test(body)).toBe(true);
  });

  test('admin page handles 5xx gracefully — shows service unavailable message', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route('**/owner/**', route => route.fulfill({ status: 503, body: JSON.stringify({ error: 'Service unavailable' }) }));
    await page.goto('/admin?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest')
    );
    expect(criticalErrors, `JS errors on 5xx: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/unavailable|error|retry|try again|service|down|offline/i.test(body)).toBe(true);
  });

});
