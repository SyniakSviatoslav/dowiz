import { test, expect } from '@playwright/test';

test.describe('Courier Tasks', () => {

  test('tasks page loads without errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    await page.goto('/courier?dev=true');
    await page.waitForTimeout(3000);

    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest')
    );
    expect(criticalErrors).toEqual([]);
  });

  test('tasks page renders content', async ({ page }) => {
    await page.goto('/courier?dev=true');
    await page.waitForTimeout(3000);

    const body = await page.textContent('body');
    expect(body!.length).toBeGreaterThan(0);
  });

  test('no cookies on courier pages', async ({ page }) => {
    await page.goto('/courier?dev=true');
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('delivery page loads', async ({ page }) => {
    await page.goto('/courier/delivery/test-id?dev=true');
    await page.waitForTimeout(3000);

    const body = await page.textContent('body');
    expect(body!.length).toBeGreaterThan(0);
  });

});
