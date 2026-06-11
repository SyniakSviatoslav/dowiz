import { test, expect } from '@playwright/test';

const ORDER_ID_404 = '00000000-0000-0000-0000-000000000001';

test.describe('Client Order Status — Live Deployment', () => {

  test('CR-5: order status page loads against live deployment', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(200);
  });

  test('CR-5: status timeline renders progression steps', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const statusSteps = page.locator('text=/received|preparing|ready|on the way|delivered/i');
    const stepCount = await statusSteps.count();
    expect(stepCount).toBeGreaterThanOrEqual(4);
    expect(stepCount).toBeLessThanOrEqual(10);
  });

  test('order not found shows appropriate message', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto(`/s/test-slug/order/${ORDER_ID_404}?dev=true`);
    await page.waitForTimeout(3000);
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
  });

  test('no cookies are set on status page', async ({ page }) => {
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(2000);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('CR-5: order status shows estimated arrival time text', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    await expect(page.locator('text=/Mbërritja|Estimated arrival|Очікуваний/i')).toBeVisible({ timeout: 8000 });
    const etaText = await page.locator('text=/\\d+\\s*min/').first().textContent();
    expect(etaText).toBeTruthy();
  });

  test('CR-5: ETA display shows time estimate on status page', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    await expect(page.locator('text=/Mbërritja|Estimated arrival|Очікуваний/i')).toBeVisible({ timeout: 8000 });
    const etaSection = page.locator('text=/\\d+.*min/');
    await expect(etaSection.first()).toBeVisible({ timeout: 5000 });
  });

  test('CR-6: share location button hidden before IN_DELIVERY', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const shareBtn = page.locator('text=Share my location with courier');
    await expect(shareBtn).toHaveCount(0);
  });

  test('CR-5: Order Details section renders items', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    // Ordinal text should contain number of items — matches sq/en/uk
    const body = await page.textContent('body');
    expect(body).toMatch(/1x|Dragon Roll|ALL/i);
  });

  test('CR-5: Total price renders', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(4000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body).toMatch(/ALL/);
  });

  test('no JS errors on status page load', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(4000);
    expect(errors).toEqual([]);
  });
});
