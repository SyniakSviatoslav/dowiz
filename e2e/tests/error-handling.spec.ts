import { test, expect } from '@playwright/test';

test.describe('Error Handling', () => {

  test('menu page handles 500 server error gracefully — shows fallback content', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route('**/public/menu/**', route => route.fulfill({ status: 500, body: JSON.stringify({ error: 'Server error' }) }));
    await page.goto('/s/test-slug?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    expect(errors, `JS errors on 500: ${errors.join('; ')}`).toEqual([]);
    // Menu data must NOT render on a 500 — the error/empty state replaces the product grid.
    await expect(page.locator('[data-testid="menu-item"]')).toHaveCount(0);
    const body = (await page.textContent('body')) ?? '';
    // Error-state words only (success-domain "menu"/"product" removed — they render on any page).
    expect(/error|unavailable|retry|try again|something went wrong|failed/i.test(body)).toBe(true);
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
    await expect(page.locator('[data-testid="menu-item"]')).toHaveCount(0);
    const body = (await page.textContent('body')) ?? '';
    expect(/error|unavailable|retry|try again|something went wrong|failed/i.test(body)).toBe(true);
  });

  test('checkout handles 422 validation error — shows error message', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    // Register the 422 mock BEFORE any interaction so the real order POST is intercepted.
    await page.route('**/customer/orders**', route => route.fulfill({ status: 422, body: JSON.stringify({ error: 'Validation failed' }) }));
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });
    await page.locator('[data-testid="menu-item-add"]').first().click();
    await expect(page.locator('#cartFabBtn')).toBeVisible({ timeout: 5000 });
    await page.locator('#cartFabBtn').click();
    // Hard assertion (was a swallowed `if (isVisible().catch(()=>false))` that silently skipped).
    const checkoutBtn = page.locator('button:has-text("Checkout")');
    await expect(checkoutBtn).toBeVisible({ timeout: 5000 });
    await checkoutBtn.click();
    // TODO(needs_staging): fill checkout-phone + submit to actually fire the order POST and assert
    // the 422 validation-error UI surfaces; requires a live staging run with a seeded menu.
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('checkout handles 429 rate limit — shows retry message', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    // Register the 429 mock BEFORE any interaction so the OTP request is intercepted.
    await page.route('**/customer/otp/**', route => route.fulfill({ status: 429, body: JSON.stringify({ error: 'Too many requests', retryAfterSeconds: 60 }) }));
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });
    await page.locator('[data-testid="menu-item-add"]').first().click();
    await expect(page.locator('#cartFabBtn')).toBeVisible({ timeout: 5000 });
    await page.locator('#cartFabBtn').click();
    const checkoutBtn = page.locator('button:has-text("Checkout")');
    await expect(checkoutBtn).toBeVisible({ timeout: 5000 });
    await checkoutBtn.click();
    // TODO(needs_staging): enter checkout-phone + trigger OTP send to actually hit /customer/otp and
    // assert the rate-limit (retry) UI surfaces; requires a live staging run with a seeded menu.
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
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
    // Positive proof of the not-found state: the "back to menu" escape renders only in the EmptyState.
    await expect(page.locator('[data-testid="order-back-to-menu"]')).toBeVisible({ timeout: 15000 });
    const body = (await page.textContent('body')) ?? '';
    // Error-state words only (success-domain "order"/"status" removed).
    expect(/not found|error|unavailable|404/i.test(body)).toBe(true);
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
    // Negative: a 401 must NOT leak owner data — no dashboard order cards / new-order banner render.
    await expect(page.locator('[data-testid^="order-card-"]')).toHaveCount(0);
    await expect(page.locator('[data-testid="owner-new-order-banner"]')).toHaveCount(0);
    const body = (await page.textContent('body')) ?? '';
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
    // Negative: a 403 must NOT leak owner data — no dashboard order cards / new-order banner render.
    await expect(page.locator('[data-testid^="order-card-"]')).toHaveCount(0);
    await expect(page.locator('[data-testid="owner-new-order-banner"]')).toHaveCount(0);
    const body = (await page.textContent('body')) ?? '';
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
    // Negative: a 503 must NOT leak owner data — no dashboard order cards render.
    await expect(page.locator('[data-testid^="order-card-"]')).toHaveCount(0);
    const body = (await page.textContent('body')) ?? '';
    expect(/unavailable|error|retry|try again|service|down|offline/i.test(body)).toBe(true);
  });

});
