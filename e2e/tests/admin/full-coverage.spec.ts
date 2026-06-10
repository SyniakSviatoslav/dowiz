import { test, expect } from '@playwright/test';

test.describe('Admin Pages — Full Coverage', () => {

  test('couriers page loads with courier list or empty state', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin/couriers?dev=true');
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/courier|couriers|invite|list|empty|no courier/i.test(body)).toBe(true);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('analytics page loads with stats and charts', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin/analytics?dev=true');
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/analytics|stats|revenue|orders|chart|graph|count/i.test(body)).toBe(true);
  });

  test('crm page loads with customer table', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin/crm?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/customer|crm|phone|order|name|table|list/i.test(body)).toBe(true);
  });

  test('settings page loads with form fields', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin/settings?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/setting|config|location|name|phone|email|save|update/i.test(body)).toBe(true);
  });

  test('onboarding page loads with step wizard', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin/onboarding?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/onboarding|welcome|step|restaurant|profile|setup|wizard/i.test(body)).toBe(true);
  });

  test('onboarding step navigation fills form and advances', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin/onboarding?dev=true');
    await page.waitForTimeout(2000);

    const inputs = page.locator('input');
    const inputCount = await inputs.count();
    expect(inputCount).toBeGreaterThanOrEqual(1);

    if (inputCount >= 3) {
      await inputs.nth(0).fill('Pizza Roma');
      await inputs.nth(1).fill('+355691234567');
      await page.waitForTimeout(300);
    }

    const nextBtn = page.locator('button:has-text("Next")');
    const nextVisible = await nextBtn.count() > 0 && await nextBtn.isEnabled();
    expect(nextVisible).toBe(true);
    if (nextVisible) {
      await nextBtn.first().click();
      await page.waitForTimeout(500);
      const bodyAfter = await page.textContent('body');
      expect(bodyAfter.length).toBeGreaterThan(100);
    }
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('all admin pages set no cookies', async ({ page }) => {
    const pages = ['/admin', '/admin/menu', '/admin/branding', '/admin/couriers', '/admin/analytics', '/admin/crm', '/admin/settings', '/admin/onboarding'];
    for (const p of pages) {
      await page.goto(`${p}?dev=true`);
      const cookies = await page.context().cookies();
      expect(cookies).toEqual([]);
    }
  });

});
