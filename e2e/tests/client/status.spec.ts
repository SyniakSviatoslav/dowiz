import { test, expect } from '@playwright/test';

const API_GLOB = '**/api/customer/orders/*/status';

const STATUS_MOCK = {
  id: 'test-order-id',
  status: 'IN_DELIVERY',
  total: 650,
  items: [{ id: 'i1', productId: 'p1', name: 'Burger', price: 650, quantity: 1 }],
  createdAt: new Date().toISOString(),
  etaMinutes: 8,
  courierName: 'A***',
  courierPhoneMasked: '+*** *** 1234',
  courierPosition: { lat: 41.33, lng: 19.82 },
  deliveryLat: 41.335,
  deliveryLng: 19.825,
};

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
    expect(stepCount).toBeLessThanOrEqual(7);
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

  // CR-5: ETA display appears
  test('order status shows estimated arrival time', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    await expect(page.locator('text=Estimated arrival')).toBeVisible({ timeout: 5000 });
    const etaText = await page.locator('text=/\\d+\\s*min/').first().textContent();
    expect(etaText).toBeTruthy();
  });

  // CR-5: Server-computed ETA overrides client mock
  test('server eta minutes display when available', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route(API_GLOB, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(STATUS_MOCK),
      });
    });
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body).toContain('8 min');
  });

  // CR-5: Courier map renders when courier info present
  test('courier map shows on status page with courier assigned', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route(API_GLOB, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(STATUS_MOCK),
      });
    });
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body).toContain('A***');
  });

  // CR-6: Share location button NOT visible before IN_DELIVERY
  test('share location button hidden before in_delivery', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    // Mock returns PREPARING status — button should not appear
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const shareBtn = page.locator('text=Share my location with courier');
    await expect(shareBtn).toHaveCount(0);
  });

  // CR-6: Share location button visible during IN_DELIVERY
  test('share location button visible during in_delivery', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route(API_GLOB, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(STATUS_MOCK),
      });
    });
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const shareBtn = page.locator('text=Share my location with courier');
    await expect(shareBtn).toBeVisible({ timeout: 5000 });
  });

  // CR-6: Share location flow — click share shows banner, stop hides it
  test('share location toggle shows and hides sharing banner', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route(API_GLOB, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(STATUS_MOCK),
      });
    });
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);

    const ctx = page.context();
    await ctx.grantPermissions(['geolocation']);
    await ctx.setGeolocation({ latitude: 41.33, longitude: 19.82 });

    // Click share
    const shareBtn = page.locator('text=Share my location with courier');
    await shareBtn.click();
    await page.waitForTimeout(1000);

    // Banner should appear
    await expect(page.locator('text=Sharing your location')).toBeVisible({ timeout: 5000 });

    // Click stop
    const stopBtn = page.locator('text=Stop');
    await stopBtn.click();
    await page.waitForTimeout(500);

    // Share button should be back
    await expect(page.locator('text=Share my location with courier')).toBeVisible({ timeout: 5000 });
  });

});
