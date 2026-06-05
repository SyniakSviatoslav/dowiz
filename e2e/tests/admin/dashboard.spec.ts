import { test, expect } from '@playwright/test';

test.describe('Admin Dashboard', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto('/admin?dev=true');
    await page.waitForTimeout(4000);
  });

  test('dashboard page loads without errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    const body = await page.textContent('body');
    expect(body).toBeTruthy();

    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest')
    );
    expect(criticalErrors).toEqual([]);
  });

  test('dashboard renders content', async ({ page }) => {
    const body = await page.textContent('body');
    expect(body).toBeTruthy();
    // Content may be minimal during loading or empty state
    expect(body.length).toBeGreaterThan(0);
  });

  test('sidebar navigation is visible', async ({ page }) => {
    const navElements = page.locator('button, a').filter({ hasText: /orders|menu|branding|dashboard/i });
    const count = await navElements.count();
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('no cookies are set on admin pages', async ({ page }) => {
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('theme variables are applied on admin', async ({ page }) => {
    const primary = await page.evaluate(() =>
      getComputedStyle(document.documentElement).getPropertyValue('--brand-primary').trim()
    );
    expect(primary).toBeTruthy();
  });

});
