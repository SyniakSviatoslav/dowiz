import { test, expect } from '@playwright/test';

test.describe('Admin Orders Page', () => {

  test('orders page is accessible from sidebar', async ({ page }) => {
    await page.goto('/admin?dev=true');
    await page.waitForTimeout(2000);

    // Click on "Orders" or similar in sidebar
    const ordersLink = page.locator('a:has-text("Orders"), a:has-text("orders")');
    if (await ordersLink.count() > 0) {
      await ordersLink.first().click();
      await page.waitForTimeout(1000);
    }
  });

  test('menu manager page loads', async ({ page }) => {
    await page.goto('/admin/menu?dev=true');
    await page.waitForTimeout(3000);

    // Should render without crashing
    const body = await page.textContent('body');
    expect(body!.length).toBeGreaterThan(0);
  });

  test('branding page loads', async ({ page }) => {
    await page.goto('/admin/branding?dev=true');
    await page.waitForTimeout(3000);

    // Should render without crashing  
    const body = await page.textContent('body');
    expect(body!.length).toBeGreaterThan(0);
  });

});
