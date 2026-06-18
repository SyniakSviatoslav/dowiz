import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

// Regression: the customer order flow must reach a rendered cart/checkout.
// Previously /checkout fetched /public/locations/:slug/menu (200) but threw
// inside renderApp on a schema mismatch — it read prod.available_names /
// location.address while the endpoint returns flat name/description — and a
// silent catch disguised it as "Error loading menu". A customer could add to
// cart but never see a cart or check out.
test.describe('Customer order flow — menu → checkout renders', () => {
  test('adding an item then visiting /checkout shows the cart, not an error', async ({ page }) => {
    await page.goto(`${BASE}/s/demo`);
    // SSR menu hydrates with add buttons
    const addBtn = page.locator('.product-add').first();
    await expect(addBtn).toBeVisible({ timeout: 15_000 });
    await addBtn.click();

    await page.goto(`${BASE}/s/demo/checkout`);
    const app = page.locator('#app');
    await expect(app).toBeVisible({ timeout: 15_000 });

    // The checkout must render the cart, not the swallowed-error message.
    await expect(app).not.toContainText('Error loading menu', { timeout: 12_000 });
    await expect(app).not.toContainText('went wrong');

    // A rendered cart shows the total/Checkout CTA and quantity controls.
    await expect(page.getByText(/Checkout/i).first()).toBeVisible({ timeout: 12_000 });
  });
});
