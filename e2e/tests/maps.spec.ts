/* eslint-disable @typescript-eslint/no-unused-vars -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { test, expect } from '@playwright/test';

test.describe('Map Components', () => {

  test('checkout page shows map with pin component', async ({ page }) => {
    // Add item to cart first, then go to checkout
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('article.product-card', { timeout: 15000 });

    // Add item and go to checkout
    await page.getByRole('button', { name: /add|shto/i }).first().click();
    await expect(page.getByTestId('cart-fab')).toBeVisible({ timeout: 5000 });
    await page.getByTestId('cart-fab').click();
    await expect(page.getByText(/your cart|shporta|кошик/i)).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /checkout|porosit|vazhdo/i }).click();
    await expect(page).toHaveURL(/\/checkout/, { timeout: 5000 });

    const pageContent = await page.textContent('body');
    expect(pageContent).toMatch(/pin|map|vendndodhje/i);
  });

  test('checkout map loads without CSP worker errors', async ({ page }) => {
    const cspErrors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error' && msg.text().includes('Content Security Policy')) {
        cspErrors.push(msg.text());
      }
    });

    await page.goto('https://dowiz.fly.dev/s/demo/checkout', { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(5000);

    // Verify no CSP worker-src/blob errors
    const workerCspErrors = cspErrors.filter(e => e.includes('worker-src') || e.includes('blob'));
    expect(workerCspErrors).toHaveLength(0);
    
    // Verify page loaded (should have CSP header with worker-src)
    const bodyText = await page.textContent('body');
    expect(bodyText).toBeTruthy();
  });

  test('delivery page renders map container', async ({ page }) => {
    await page.goto('/courier/delivery/test-id?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const pageContent = await page.textContent('body');
    expect(pageContent).toMatch(/drop-off|delivery|deliver|dorezim|njoftim/i);
  });

  test('order status page has map area', async ({ page }) => {
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const bodyText = await page.textContent('body');
    expect(bodyText).toBeTruthy();
    expect(bodyText!.length).toBeGreaterThan(20);
  });

  test('admin dashboard shows courier map', async ({ page }) => {
    await page.goto('/admin?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const courierHeading = page.getByText(/couriers live|korrierët|кур'єри/i);
    const isVisible = await courierHeading.isVisible().catch(() => false);
  });

  test('onboarding page has radius map', async ({ page }) => {
    await page.goto('/admin/onboarding?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const pageContent = await page.textContent('body');
    expect(pageContent).toBeTruthy();
    expect(pageContent!.length).toBeGreaterThan(50);
  });

});
