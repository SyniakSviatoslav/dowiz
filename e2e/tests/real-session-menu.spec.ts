import { test, expect } from '@playwright/test';

test.describe('Real Session — Menu Rebuild Verification', () => {

  test('Supply Library page loads', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    await page.goto('/admin/supplies?dev=true');

    // Specific functional-render proof: the page heading mounts (not body.length).
    // If the error boundary fired, this heading is absent → test fails.
    await expect(page.getByRole('heading', { name: 'Supplies' })).toBeVisible({ timeout: 15000 });
    await expect(page.getByText('Something went wrong')).toHaveCount(0);

    const criticalErrors = errors.filter(e => !e.includes('favicon') && !e.includes('manifest'));
    expect(criticalErrors).toEqual([]);
  });

  test('Menu Manager loads', async ({ page }) => {
    await page.goto('/admin/menu?dev=true');

    // Page-level control rendered only when the manager mounts functionally.
    await expect(page.locator('[data-testid="kitchen-busy-toggle"]')).toBeVisible({ timeout: 15000 });
    await expect(page.getByText('Something went wrong')).toHaveCount(0);
  });

  // NEGATIVE control: without the ?dev=true bypass and with no token, the admin
  // route must redirect unauthenticated users to /login (AdminRoutes guard).
  for (const route of ['/admin/supplies', '/admin/menu']) {
    test(`${route} redirects unauthenticated user to login`, async ({ page }) => {
      await page.goto(route);
      await page.waitForURL(/\/login/, { timeout: 15000 });
      await expect(page.locator('#login-email')).toBeVisible();
    });
  }

  test('Client menu page shows products with Tabler icons', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');
<<<<<<< Updated upstream
    await page.waitForSelector('article.product-card', { timeout: 15000 });
    await page.waitForTimeout(1000);
=======
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });
>>>>>>> Stashed changes

    // Product cards rendered
    const cards = page.locator('article.product-card');
    const count = await cards.count();
    expect(count).toBeGreaterThan(0);

    // Stars should be Tabler icons (not emoji ★)
    const starIcons = page.locator('.ti-star-filled');
    await expect(starIcons.first()).toBeVisible();

<<<<<<< Updated upstream
    // Cart FAB should use Tabler icon
    // Add item first
    const addBtn = page.locator('article.product-card button[aria-label="Add"]').first();
    if (await addBtn.isVisible().catch(() => false)) {
      await addBtn.click();
      await page.waitForTimeout(500);
      const fab = page.locator('#cartFabBtn');
      if (await fab.isVisible({ timeout: 3000 }).catch(() => false)) {
        // Should have Tabler shopping cart icon (not emoji)
        const fabIcon = fab.locator('.ti-shopping-cart');
        const fabIconCount = await fabIcon.count();
        expect(fabIconCount).toBeGreaterThanOrEqual(0);
      }
    }
=======
    // Cart FAB should use a Tabler icon — add an item, then verify the FAB + its icon.
    const addBtn = page.locator('[data-testid="menu-item-add"]').first();
    await expect(addBtn).toBeVisible();
    await addBtn.click();

    const fab = page.locator('#cartFabBtn');
    await expect(fab).toBeVisible({ timeout: 5000 });

    // The Tabler shopping-cart glyph must actually be present (not an emoji fallback).
    const fabIcon = fab.locator('.ti-shopping-cart');
    await expect(fabIcon).toHaveCount(1);
>>>>>>> Stashed changes
  });

  test('Language switcher visible on client header', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');
<<<<<<< Updated upstream
    await page.waitForSelector('article.product-card', { timeout: 15000 });
    await page.waitForTimeout(1000);
=======
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });
>>>>>>> Stashed changes

    // The switcher is a required affordance on the client header.
    const langBtn = page.locator('button[aria-label*="Switch language" i]');
    await expect(langBtn.first()).toBeVisible({ timeout: 5000 });
  });

});
