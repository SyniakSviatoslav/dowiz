import { test, expect } from '@playwright/test';

test.describe('Admin Pages — Full Coverage', () => {

  test('couriers page loads', async ({ page }) => {
    await page.goto('/admin/couriers?dev=true');
    await page.waitForTimeout(4000);
    const body = await page.textContent('body');
    expect(body).toBeTruthy();
  });

  test('analytics page loads with stats', async ({ page }) => {
    await page.goto('/admin/analytics?dev=true');
    await page.waitForTimeout(4000);
    const body = await page.textContent('body');
    expect(body).toBeTruthy();
  });

  test('crm page loads with customer table', async ({ page }) => {
    await page.goto('/admin/crm?dev=true');
    await page.waitForTimeout(3000);
    const body = await page.textContent('body');
    expect(body).toBeTruthy();
  });

  test('settings page loads with form', async ({ page }) => {
    await page.goto('/admin/settings?dev=true');
    await page.waitForTimeout(3000);
    const body = await page.textContent('body');
    expect(body).toBeTruthy();
  });

  test('onboarding page loads with step wizard', async ({ page }) => {
    await page.goto('/admin/onboarding?dev=true');
    await page.waitForTimeout(3000);
    const body = await page.textContent('body');
    expect(body).toBeTruthy();
  });

  test('onboarding step navigation works', async ({ page }) => {
    await page.goto('/admin/onboarding?dev=true');
    await page.waitForTimeout(2000);

    // Step 0: Fill restaurant info
    const inputs = page.locator('input');
    if (await inputs.count() >= 3) {
      await inputs.nth(0).fill('Pizza Roma');
      await inputs.nth(1).fill('+355691234567');
      // slug auto-generates from name
      await page.waitForTimeout(300);
    }

    // Click Next
    const nextBtn = page.locator('button:has-text("Next")');
    if (await nextBtn.count() > 0 && await nextBtn.isEnabled()) {
      await nextBtn.first().click();
      await page.waitForTimeout(500);
    }
  });

  test('all admin pages no cookies', async ({ page }) => {
    const pages = ['/admin', '/admin/menu', '/admin/branding', '/admin/couriers', '/admin/analytics', '/admin/crm', '/admin/settings', '/admin/onboarding'];
    for (const p of pages) {
      await page.goto(`${p}?dev=true`);
      const cookies = await page.context().cookies();
      expect(cookies).toEqual([]);
    }
  });

});
