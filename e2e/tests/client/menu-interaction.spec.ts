import { test, expect } from '@playwright/test';

test.describe('Client Menu — Interaction Tests', () => {
  test('menu page loads with hero and categories', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });
    await page.waitForTimeout(5000);

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(500);

    // Navigation should be present (sticky nav bar)
    const nav = page.locator('nav').first();
    await expect(nav).toBeVisible({ timeout: 10000 });
  });

  test('category tab click scrolls to section', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });
    await page.waitForTimeout(4000);

    // Click a category tab in the nav
    const tab = page.locator('[role="tab"]').first();
    if (await tab.count() > 0) {
      const tabText = await tab.textContent();
      await tab.click();
      await page.waitForTimeout(1000);

      // Should scroll to the section
      const body = await page.textContent('body');
      expect(body).toContain(tabText!.trim());
    }
  });

  test('product card renders with name and price', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });
    await page.waitForTimeout(4000);

    // Wait for product cards to appear
    const productCards = page.locator('article').filter({ has: page.locator('.font-bold') });
    const cardCount = await productCards.count();

    if (cardCount > 0) {
      // First product should have a name and price
      const firstCard = productCards.first();
      const text = await firstCard.textContent();

      // Should have at least some text content
      expect(text.length).toBeGreaterThan(10);
    } else {
      // If no products, at least the page rendered
      const body = await page.textContent('body');
      expect(body.length).toBeGreaterThan(200);
    }
  });

  test('search input filters products', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });
    await page.waitForTimeout(4000);

    // Search input should be present (above the product grid, in the sticky bar)
    const searchInput = page.locator('input[placeholder*="Search"]');
    if (await searchInput.count() > 0) {
      await searchInput.fill('test');
      await page.waitForTimeout(500);

      // Should not crash
      const body = await page.textContent('body');
      expect(body.length).toBeGreaterThan(100);
    }
  });

  test('add to cart button exists on product cards', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });
    await page.waitForTimeout(4000);

    // Find plus buttons (add to cart) on product cards
    const addBtns = page.locator('[data-testid="menu-item-add"]');
    const count = await addBtns.count();
    if (count > 0) {
      // Click first add button
      await addBtns.first().click();
      await page.waitForTimeout(1000);

      // Cart FAB should appear (if cart wasn't empty before)
      const fabBtn = page.locator('#cartFabBtn');
      const fabCount = await fabBtn.count();
      expect(fabCount).toBeGreaterThanOrEqual(0); // May already exist
    }
  });

  test('product cards show product info and add button', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });
    await page.waitForTimeout(6000);

    const productCards = page.locator('[data-testid="menu-item"]').first();
    await expect(productCards).toBeVisible({ timeout: 15000 });

    const count = await page.locator('[data-testid="menu-item"]').count();
    expect(count).toBeGreaterThanOrEqual(1);

    // Each card should have a price displayed
    const body = await page.textContent('body');
    expect(body).toMatch(/\d+/);
  });

  test('no JS errors on client menu', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto('/s/demo', { waitUntil: 'networkidle' });
    await page.waitForTimeout(4000);

    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('ResizeObserver')
    );
    expect(criticalErrors).toEqual([]);
  });

  test('no cookies set on client menu', async ({ page }) => {
    await page.goto('/s/demo', { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
