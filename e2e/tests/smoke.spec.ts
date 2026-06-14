import { test, expect } from '@playwright/test';

test.describe('Smoke Tests — App Loads', () => {

  test('React app boots without errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));
    page.on('console', (msg) => {
      if (msg.type() === 'error') errors.push(msg.text());
    });

    await page.goto('/');

    // Verify the app mounted (root element has content)
    const root = page.locator('#root');
    await expect(root).toBeVisible({ timeout: 15000 });

    // Filter benign errors
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') &&
      !e.includes('404 Not Found') &&
      !e.includes('manifest') &&
      !e.includes('Failed to load resource') &&
      !e.includes('serviceWorker')
    );
    expect(criticalErrors).toEqual([]);
  });

  test('Menu page loads with mock data', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');

    // Wait for product cards to render (uses class `product-card`)
    await page.waitForSelector('article.product-card', {
      timeout: 20000,
    });

    // Verify products are visible
    const cards = page.locator('article.product-card');
    const count = await cards.count();
    expect(count).toBeGreaterThan(0);

    // Verify category nav is visible
    const nav = page.locator('nav.sticky');
    await expect(nav).toBeVisible({ timeout: 5000 });
  });

  test('Cart FAB hidden when cart is empty (correct behavior)', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');

    // Wait for page to load
    await page.waitForSelector('article.product-card', { timeout: 15000 });

    // Cart FAB should NOT be visible when cart is empty (0 items)
    const fab = page.locator('#cartFabBtn');
    await expect(fab).not.toBeVisible({ timeout: 5000 });
  });

  test('Add to cart shows CartFAB', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('article.product-card', { timeout: 15000 });

    // Click the "+" button on first available product
    const addBtn = page.locator('article.product-card button[aria-label="Add"]').first();
    await addBtn.click();

    // Cart FAB should appear with count
    const fab = page.locator('#cartFabBtn');
    await expect(fab).toBeVisible({ timeout: 5000 });
  });

  test('Theme variables are applied', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');

    // Check CSS custom properties are set
    const primary = await page.evaluate(() => {
      return getComputedStyle(document.documentElement).getPropertyValue('--brand-primary').trim();
    });
    expect(primary).toBeTruthy();
    expect(primary).not.toBe('');
  });

  test('No cookies are set', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

});
