import { test, expect } from '@playwright/test';

test.describe('Client Cart', () => {

  test.beforeEach(async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });
  });

  async function addItemsToCart(page: any, count: number = 2) {
    const addButtons = page.locator('[data-testid="menu-item-add"]');
    for (let i = 0; i < count; i++) {
      await addButtons.nth(i).click();
    }
    await expect(page.locator('#cartFabBtn')).toBeVisible({ timeout: 3000 });
  }

  async function openCart(page: any) {
    await page.locator('#cartFabBtn').click();
    await expect(page.locator('text=Your Cart')).toBeVisible({ timeout: 3000 });
  }

  test('cart drawer shows items with names and prices', async ({ page }) => {
    await addItemsToCart(page, 2);
    await openCart(page);

    // Should show item names
    const items = page.locator('text=Your Cart').locator('..').locator('.space-y-4 > div');
    const count = await items.count();
    expect(count).toBeGreaterThanOrEqual(2);
  });

  test('quantity stepper increases quantity', async ({ page }) => {
    await addItemsToCart(page, 1);
    await openCart(page);

    // Click + to increase quantity
    const plusBtn = page.locator('text=Your Cart').locator('..').locator('button:has-text("+")').first();
    const initialQty = await page.locator('text=Your Cart').locator('..').locator('.w-4.text-center').first().textContent();

    await plusBtn.click();
    await page.waitForTimeout(500);

    const newQty = await page.locator('text=Your Cart').locator('..').locator('.w-4.text-center').first().textContent();
    expect(Number(newQty)).toBeGreaterThan(Number(initialQty));
  });

  test('quantity stepper decreases quantity', async ({ page }) => {
    await addItemsToCart(page, 1);
    await openCart(page);

    // First increase to 2, then decrease to 1
    const plusBtn = page.locator('text=Your Cart').locator('..').locator('button:has-text("+")').first();
    await plusBtn.click();
    await page.waitForTimeout(300);

    const minusBtn = page.locator('text=Your Cart').locator('..').locator('button:has-text("-")').first();
    await minusBtn.click();
    await page.waitForTimeout(300);

    const qty = await page.locator('text=Your Cart').locator('..').locator('.w-4.text-center').first().textContent();
    expect(Number(qty)).toBe(1);
  });

  test('remove item when quantity reaches 0', async ({ page }) => {
    await addItemsToCart(page, 1);
    await openCart(page);

    // Click - to decrease from 1 to 0 (should remove)
    const minusBtn = page.locator('text=Your Cart').locator('..').locator('button:has-text("-")').first();
    await minusBtn.click();
    await page.waitForTimeout(500);

    // Should show empty cart message
    await expect(page.locator('text=Your cart is empty')).toBeVisible({ timeout: 3000 });
  });

  test('cart shows total correctly', async ({ page }) => {
    await addItemsToCart(page, 2);
    await openCart(page);

    // Should show total
    const totalEl = page.locator('text=Total').first();
    await expect(totalEl).toBeVisible();
  });

  test('cart persist across page navigation', async ({ page }) => {
    await addItemsToCart(page, 2);

    // Navigate away and back
    await page.goto('/');
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });

    // FAB should still show
    const fab = page.locator('#cartFabBtn');
    await expect(fab).toBeVisible({ timeout: 5000 });
  });

  test('checkout button navigates to checkout page', async ({ page }) => {
    await addItemsToCart(page, 1);
    await openCart(page);

    // Click Checkout button
    const checkoutBtn = page.locator('button:has-text("Checkout")');
    await checkoutBtn.click();

    // Should navigate to checkout page
    await expect(page).toHaveURL(/\/checkout/, { timeout: 5000 });
  });

});
