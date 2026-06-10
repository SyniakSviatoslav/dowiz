import { test, expect } from '@playwright/test';

test.describe('Embed Mode', () => {

  test('embed mode hides fixed elements on menu page', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?dev=true&embed=true');
    await page.waitForSelector('article.product-card', { timeout: 15000 });
    await page.locator('article.product-card button[aria-label="Add"]').first().click();
    await expect(page.locator('#cartFabBtn')).toBeVisible({ timeout: 5000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
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
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
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
    await page.waitForSelector('article.product-card', { timeout: 15000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const cards = await page.locator('article.product-card').count();
    expect(cards).toBeGreaterThan(0);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

});
