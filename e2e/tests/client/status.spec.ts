import { test, expect } from '@playwright/test';

test.describe('Client Order Status', () => {

  test('order status page loads with order content', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(200);
    expect(/order|status|received|preparing|ready|delivered|total/i.test(body)).toBe(true);
  });

  test('status timeline shows progression steps', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(2000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const statusSteps = page.locator('text=/received|preparing|ready|on the way|delivered/i');
    const stepCount = await statusSteps.count();
    expect(stepCount).toBeGreaterThanOrEqual(1);
    expect(stepCount).toBeLessThanOrEqual(5);
  });

  test('order not found shows appropriate message', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/nonexistent?dev=true');
    await page.waitForTimeout(2000);
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/not found|error|unavailable|order|status/i.test(body)).toBe(true);
  });

  test('no cookies are set on status page', async ({ page }) => {
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

});
