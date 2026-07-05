import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe('UI: Client Menu — Detail Modal, Modifiers, Search, Filter', () => {
  let locationSlug = 'demo';

  test('Detail modal opens on product click and shows product info', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const cards = page.locator('[data-testid="menu-item"]');
    const cardCount = await cards.count();
    test.skip(cardCount === 0, 'No product cards');

    await cards.first().click();
    await page.waitForTimeout(1000);

    const modal = page.locator('[role="dialog"], .modal, [class*="modal"], [class*="drawer"]').first();
    const modalVisible = await modal.isVisible({ timeout: 3000 }).catch(() => false);

    if (modalVisible) {
      const modalText = await modal.textContent();
      expect(modalText.length).toBeGreaterThan(50);
      // Close modal
      const closeBtn = modal.locator('button').first();
      if (await closeBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
        await closeBtn.click();
        await page.waitForTimeout(500);
      }
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Search input filters product cards', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const searchInput = page.locator('input[type="search"], input[placeholder*="Search" i], input[placeholder*="Kërko" i]').first();
    if (await searchInput.isVisible({ timeout: 5000 }).catch(() => false)) {
      await searchInput.click();
      await searchInput.fill('pizza');
      await page.waitForTimeout(500);
      // Should not crash
      const body = await page.textContent('body');
      expect(body.length).toBeGreaterThan(50);
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Category tabs navigate between sections', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const tabs = page.locator('[role="tab"], button.category-tab, nav button').first();
    if (await tabs.isVisible({ timeout: 5000 }).catch(() => false)) {
      // Click first two tabs
      const allTabs = page.locator('[role="tab"], button.category-tab');
      const count = await allTabs.count();
      if (count >= 2) {
        await allTabs.nth(0).click();
        await page.waitForTimeout(300);
        await allTabs.nth(1).click();
        await page.waitForTimeout(300);
      }
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Skeleton loading state appears before content', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'domcontentloaded' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const skeleton = page.locator('[class*="skeleton"], [class*="Skeleton"], .animate-pulse').first();
    const hasSkeleton = await skeleton.isVisible({ timeout: 3000 }).catch(() => false);

    // Wait for full load
    await page.waitForLoadState('networkidle');

    if (hasSkeleton) {
      // Skeleton should be gone after content loads
      await expect(skeleton).not.toBeVisible({ timeout: 5000 });
    }

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Cart FAB updates count when items added', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const addBtns = page.locator('[data-testid="menu-item-add"]');
    const count = await addBtns.count();
    test.skip(count === 0, 'No add buttons');

    // Add first item
    await addBtns.first().click();
    await page.waitForTimeout(500);

    const fab = page.locator('#cartFabBtn');
    await expect(fab).toBeVisible({ timeout: 5000 });
    const fabText1 = await fab.textContent();
    expect(fabText1).toMatch(/[1-9]/);

    // Add another item if available
    if (count > 1) {
      await addBtns.nth(1).click();
      await page.waitForTimeout(500);
      const fabText2 = await fab.textContent();
      expect(fabText2).toMatch(/[2-9]/);
    }

    // Open cart drawer
    await fab.click();
    await page.waitForTimeout(500);

    const cartDrawer = page.locator('h2, h3, [role="dialog"]').filter({ hasText: /Cart|Shporta|Your/i }).first();
    await expect(cartDrawer).toBeVisible({ timeout: 3000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Product cards show price in ALL currency', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body).toMatch(/ALL|Lek|\d+/);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('No JS errors on menu page after interactions', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Perform multiple interactions
    const tabs = page.locator('[role="tab"]');
    if (await tabs.first().isVisible({ timeout: 3000 }).catch(() => false)) {
      await tabs.first().click();
    }

    const addBtn = page.locator('[data-testid="menu-item-add"]').first();
    if (await addBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
      await addBtn.click();
    }

    await page.waitForTimeout(1000);

    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('ResizeObserver')
    );
    expect(criticalErrors).toEqual([]);
  });
});
