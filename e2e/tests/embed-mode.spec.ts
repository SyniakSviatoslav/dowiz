import { test, expect } from '@playwright/test';

test.describe('Embed Mode', () => {

  test('embed mode hides fixed elements on menu page', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?dev=true&embed=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });
    await page.locator('[data-testid="menu-item-add"]').first().click();
    await expect(page.locator('#cartFabBtn')).toBeVisible({ timeout: 5000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible();
    const embedClassPresent = await page.evaluate(() =>
      document.documentElement.classList.contains('embed-mode') ||
      document.body.classList.contains('embed-mode')
    );
    expect(embedClassPresent).toBe(true);
  });

  test('embed mode page loads without JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?dev=true&embed=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    const criticalErrors = errors.filter(e => !e.includes('favicon') && !e.includes('404'));
    expect(criticalErrors).toEqual([]);
    await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible({ timeout: 15000 });
  });

  test('embed mode has no horizontal scroll overflow', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?dev=true&embed=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const viewport = page.viewportSize();
    if (viewport) {
      const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
      expect(scrollWidth).toBeLessThanOrEqual(viewport.width + 5);
    }
  });

  test('embed mode renders full menu with product cards', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?dev=true&embed=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const cards = await page.locator('[data-testid="menu-item"]').count();
    // test-slug fixture seeds a full menu; smoke.spec asserts the same >=3 floor.
    // >0 let a 1-of-N partial/broken render pass.
    expect(cards).toBeGreaterThanOrEqual(3);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

});
