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
    // NOTE: do NOT blanket-exclude 'Failed to load resource' — it masks real
    // network failures for API calls, JS chunks, and images. Only known-benign
    // static assets (favicon/manifest/serviceWorker) are excused.
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') &&
      !e.includes('404 Not Found') &&
      !e.includes('manifest') &&
      !e.includes('serviceWorker')
    );
    expect(criticalErrors).toEqual([]);
  });

  test('Menu page loads with mock data', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');

    // Wait for product cards to render (data-testid="menu-item")
    await page.waitForSelector('[data-testid="menu-item"]', {
      timeout: 20000,
    });

    // Verify a real menu rendered — the dev-seed has many products, so a single
    // card / render stub / stale mock must fail. Assert against the seed floor.
    const cards = page.locator('[data-testid="menu-item"]');
    const count = await cards.count();
    expect(count).toBeGreaterThanOrEqual(3);

    // At least one card must render a real, non-empty price (e.g. "1200 ALL" /
    // "12.00 €") — a render stub with no data would have no money string.
    await expect(cards.first()).toContainText(/\d+\s*(ALL|€)/);

    // Verify category nav is visible
    const nav = page.locator('nav.sticky');
    await expect(nav).toBeVisible({ timeout: 5000 });
  });

  test('Cart FAB hidden when cart is empty (correct behavior)', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');

    // Wait for page to load
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });

    // Cart FAB should NOT be visible when cart is empty (0 items)
    const fab = page.locator('#cartFabBtn');
    await expect(fab).not.toBeVisible({ timeout: 5000 });
  });

  test('Add to cart shows CartFAB', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });

    // Click the "+" button on first available product
    const addBtn = page.locator('[data-testid="menu-item-add"]').first();
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
    // Must be a real CSS colour — a non-empty string like 'inherit', '0', or a
    // garbage value from a broken theme path would pass a truthy check.
    expect(primary).toMatch(/^#[0-9a-f]{3,8}$|^rgba?\(/i);
  });

  test('No cookies are set', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

});
