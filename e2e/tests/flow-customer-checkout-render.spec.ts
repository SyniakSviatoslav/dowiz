import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

// Regression: the customer order flow must reach a rendered checkout, not an
// error. /s/:slug is the SSR menu (hydrated, .product-add); adding an item then
// visiting /s/:slug/checkout (the SSR checkout shell, #app) must render the cart
// — previously a schema mismatch threw inside renderApp and a silent catch
// disguised it as "Error loading menu", so a customer could add but never check
// out. Menu and checkout share the SSR cart, so the handoff works.
test.describe('Customer order flow — menu → checkout renders', () => {
  test('adding an item then visiting /checkout shows the cart, not an error', async ({ page }) => {
    await page.goto(`${BASE}/s/demo`);

    const addBtn = page.locator('.product-add').first();
    await expect(addBtn).toBeVisible({ timeout: 15_000 });
    await addBtn.click();

    await page.goto(`${BASE}/s/demo/checkout`);
    const app = page.locator('#app');
    await expect(app).toBeVisible({ timeout: 15_000 });

    await expect(app).not.toContainText('Error loading menu', { timeout: 12_000 });
    await expect(app).not.toContainText('went wrong');

    await expect(page.getByText(/Checkout/i).first()).toBeVisible({ timeout: 12_000 });
  });
});
