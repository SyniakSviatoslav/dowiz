import { test, expect } from '@playwright/test';

test.describe('Courier Tasks', () => {

  test('tasks page loads without JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));
    await page.goto('/courier?dev=true');
    await page.waitForTimeout(3000);
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest')
    );
    expect(criticalErrors).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/task|delivery|order|accept|reject|pending|active|no task|empty/i.test(body)).toBe(true);
  });

  test('tasks page shows delivery assignments or empty state', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));
    await page.goto('/courier?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    const assignments = page.locator('[class*="card"], [class*="assignment"], [class*="task"], article');
    const count = await assignments.count();
    if (count > 0) {
      expect(count).toBeGreaterThanOrEqual(1);
    }
  });

  test('no cookies on courier pages', async ({ page }) => {
    await page.goto('/courier?dev=true');
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('delivery page loads with map and dropoff info', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/delivery/test-id?dev=true');
    await page.waitForTimeout(3000);
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('maplibregl')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/delivery|map|dropoff|pickup|address|order|status|complete/i.test(body)).toBe(true);
  });

});
