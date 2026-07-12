import { test, expect } from '@playwright/test';

// Hardened against false-greens (AGENTS.md "Test Integrity"): item count is asserted
// on a semantic [data-testid="cart-item"] (not a CSS layout path that passes on empty
// wrapper divs); quantity is read via [data-testid="cart-item-qty"] with toHaveText
// auto-waiting (no sleep); the total asserts a real non-zero numeric value; persistence
// re-opens the drawer and asserts the same item count.
// NEEDS-STAGING: assertions require a live run against deployed staging carrying the new
// data-testid anchors (cart-item / cart-item-qty / cart-total) added to ClientLayout.tsx.

test.describe('Client Cart', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('article.product-card', { timeout: 15000 });
  });

  async function addItemsToCart(page: any, count: number = 2) {
    const addButtons = page.locator('article.product-card button[aria-label="Add"]');
    for (let i = 0; i < count; i++) {
      await addButtons.nth(i).click();
    }
    await expect(page.locator('[data-testid="cart-open"]')).toBeVisible({ timeout: 3000 });
  }

  async function openCart(page: any) {
    await page.locator('[data-testid="cart-open"]').click();
    // checkout button only renders inside the open drawer when items.length > 0
    await expect(page.locator('[data-testid="cart-checkout"]')).toBeVisible({ timeout: 3000 });
  }

  test('cart drawer shows items with names and prices', async ({ page }) => {
    await addItemsToCart(page, 2);
    await openCart(page);

    // Two distinct products → exactly two semantic cart items (not just any wrapper div)
    await expect(page.locator('[data-testid="cart-item"]')).toHaveCount(2);
    // Each item carries a non-empty name and a quantity readout
    await expect(page.locator('[data-testid="cart-item"]').first()).toContainText(/\S/);
    await expect(page.locator('[data-testid="cart-item-qty"]')).toHaveCount(2);
  });

  test('quantity stepper increases quantity', async ({ page }) => {
    await addItemsToCart(page, 1);
    await openCart(page);

    const item = page.locator('[data-testid="cart-item"]').first();
    const qty = item.locator('[data-testid="cart-item-qty"]');
    await expect(qty).toHaveText('1');

    // buttons order inside an item: [decrease, qty, increase]
    await item.locator('button').nth(1).click();
    await expect(qty).toHaveText('2');
  });

  test('quantity stepper decreases quantity', async ({ page }) => {
    await addItemsToCart(page, 1);
    await openCart(page);

    const item = page.locator('[data-testid="cart-item"]').first();
    const qty = item.locator('[data-testid="cart-item-qty"]');
    await expect(qty).toHaveText('1');

    // First increase to 2, then decrease back to 1
    await item.locator('button').nth(1).click();
    await expect(qty).toHaveText('2');

    await item.locator('button').nth(0).click();
    await expect(qty).toHaveText('1');
  });

  test('remove item when quantity reaches 0', async ({ page }) => {
    await addItemsToCart(page, 1);
    await openCart(page);

    const item = page.locator('[data-testid="cart-item"]').first();
    // decrease from 1 to 0 removes the item
    await item.locator('button').nth(0).click();

    // Should show empty cart message, and no cart items remain
    await expect(page.locator('text=Your cart is empty')).toBeVisible({ timeout: 3000 });
    await expect(page.locator('[data-testid="cart-item"]')).toHaveCount(0);
  });

  test('cart shows total correctly', async ({ page }) => {
    await addItemsToCart(page, 2);
    await openCart(page);

    // Assert a real, non-zero formatted price value — not just the "Total" label
    const totalEl = page.locator('[data-testid="cart-total"]');
    await expect(totalEl).toBeVisible();
    await expect(totalEl).toHaveText(/[1-9]/);
  });

  test('cart persist across page navigation', async ({ page }) => {
    await addItemsToCart(page, 2);

    // Navigate away and back
    await page.goto('/');
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('article.product-card', { timeout: 15000 });

    // FAB should still show, and the same items must persist in the drawer
    await expect(page.locator('[data-testid="cart-open"]')).toBeVisible({ timeout: 5000 });
    await openCart(page);
    await expect(page.locator('[data-testid="cart-item"]')).toHaveCount(2);
  });

  test('checkout button navigates to checkout page', async ({ page }) => {
    await addItemsToCart(page, 1);
    await openCart(page);

    // Click Checkout button
    await page.locator('[data-testid="cart-checkout"]').click();

    // Should navigate to checkout page
    await expect(page).toHaveURL(/\/checkout/, { timeout: 5000 });
  });

});
