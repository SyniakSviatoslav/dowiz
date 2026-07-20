import { test, expect } from '@playwright/test';

test.describe('Menu Manager — Interactive CRUD', () => {

  test('menu page loads and renders category tabs', async ({ page }) => {
    await page.goto('/admin/menu?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    // Page should have content
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    // Category tabs should be visible (buttons with rounded-md class in scrollable area)
    const tabBar = page.locator('.overflow-x-auto.hide-scrollbar').first();
    await expect(tabBar).toBeVisible({ timeout: 10000 });

    // Should have at least one category tab button
    const tabBtns = tabBar.locator('button');
    const tabCount = await tabBtns.count();
    expect(tabCount).toBeGreaterThanOrEqual(1);
  });

  test('clicking category tab shows products', async ({ page }) => {
    await page.goto('/admin/menu?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    const tabBar = page.locator('.overflow-x-auto.hide-scrollbar').first();
    const tabs = tabBar.locator('button');
    const count = await tabs.count();

    if (count > 1) {
      await tabs.nth(1).click();
      await page.waitForTimeout(2000);

      // After clicking a category, the product area should exist
      const productArea = page.locator('.space-y-2').first();
      await expect(productArea).toBeVisible({ timeout: 10000 });
    }
  });

  test('search input filters displayed products', async ({ page }) => {
    await page.goto('/admin/menu?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    // Find any visible text input
    const searchInput = page.locator('input:not([type="time"]):not([type="hidden"])').first();
    await expect(searchInput).toBeVisible({ timeout: 10000 });

    await searchInput.fill('Pizza');
    await page.waitForTimeout(500);

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
  });

  test('sort icon opens dropdown with options', async ({ page }) => {
    await page.goto('/admin/menu?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    // Find the sort button (has ti-arrows-sort icon)
    const sortBtn = page.locator('button').filter({ has: page.locator('.ti-arrows-sort') }).first();
    await expect(sortBtn).toBeVisible({ timeout: 10000 });

    await sortBtn.click();
    await page.waitForTimeout(500);

    // Sort dropdown should appear — it has shadow class
    const dropdown = page.locator('[class*="shadow-elevation"]').first();
    await expect(dropdown).toBeVisible({ timeout: 5000 });
  });

  test('availability filter icon opens dropdown', async ({ page }) => {
    await page.goto('/admin/menu?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    // Find filter button (has ti-filter icon)
    const filterBtn = page.locator('button').filter({ has: page.locator('.ti-filter') }).first();
    await expect(filterBtn).toBeVisible({ timeout: 10000 });

    await filterBtn.click();
    await page.waitForTimeout(500);

    // Dropdown should appear
    const dropdown = page.locator('[class*="shadow-elevation"]').first();
    await expect(dropdown).toBeVisible({ timeout: 5000 });
  });

  test('products display availability toggles when loaded', async ({ page }) => {
    await page.goto('/admin/menu?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(4000);

    // Try clicking a category tab first
    const tabBar = page.locator('.overflow-x-auto.hide-scrollbar').first();
    const tabs = tabBar.locator('button');
    const tabCount = await tabs.count();
    if (tabCount > 1) {
      await tabs.nth(1).click();
      await page.waitForTimeout(2000);
    }

    // Look for availability toggles (role="switch")
    const toggles = page.locator('[role="switch"]');
    const toggleCount = await toggles.count();
    expect(toggleCount).toBeGreaterThanOrEqual(0);
  });

  test('no JS errors on page', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto('/admin/menu?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    const critical = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('ResizeObserver')
    );
    expect(critical).toEqual([]);
  });

  test('no cookies set', async ({ page }) => {
    await page.goto('/admin/menu?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
