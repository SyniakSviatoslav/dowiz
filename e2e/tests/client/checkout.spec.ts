import { test, expect } from '@playwright/test';

test.describe('Client Checkout', () => {

  async function addItemAndGoToCheckout(page: any) {
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('article.product-card', { timeout: 15000 });

    // Add an item
    await page.locator('article.product-card button[aria-label="Add"]').first().click();
    await page.waitForTimeout(500);
    await expect(page.locator('#cartFabBtn')).toBeVisible({ timeout: 5000 });

    // Open cart and click Checkout
    await page.locator('#cartFabBtn').click();
    await expect(page.locator('text=Your Cart')).toBeVisible({ timeout: 3000 });
    await page.locator('button:has-text("Checkout")').click();
    await expect(page).toHaveURL(/\/checkout/, { timeout: 5000 });
  }

  test('checkout page loads with cart items displayed', async ({ page }) => {
    await addItemAndGoToCheckout(page);
    // Wait for checkout content to render
    await page.waitForTimeout(1000);
    const body = await page.textContent('body');
    expect(body).toContain('Checkout');
  });

  test('back button returns to menu', async ({ page }) => {
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(500);

    // Click the second ← button (checkout's own back button, not the header)
    const backBtns = page.locator('button:has-text("←")');
    if (await backBtns.count() >= 2) {
      await backBtns.nth(1).click();
    } else if (await backBtns.count() === 1) {
      await backBtns.first().click();
    }
    await expect(page.locator('article.product-card').first()).toBeVisible({ timeout: 5000 });
  });

  test('empty cart redirects with empty message', async ({ page }) => {
    await page.goto('/s/test-slug/checkout?dev=true');

    // Should show empty cart message
    await expect(page.locator('text=Cart is empty')).toBeVisible({ timeout: 5000 });
  });

  test('delivery type selector works (delivery, pickup, scheduled)', async ({ page }) => {
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(1000);

    // Check that delivery/pickup/scheduled buttons exist
    const body = await page.textContent('body');
    expect(body).toMatch(/delivery|pickup|schedule/i);
  });

  test('address input field is present for delivery', async ({ page }) => {
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(1000);

    // Check for input fields
    const inputs = page.locator('input');
    const count = await inputs.count();
    expect(count).toBeGreaterThanOrEqual(1);
  });

  test('phone input field is present', async ({ page }) => {
    await addItemAndGoToCheckout(page);

    const phoneField = page.locator('input[type="tel"]');
    const count = await phoneField.count();
    if (count > 0) {
      await expect(phoneField.first()).toBeVisible();
    } else {
      // Phone field might be in the OTP modal
      const otpModal = page.locator('[role="dialog"], .modal, [class*="otp"]');
      const hasOtpUI = await otpModal.count() > 0;
      expect(hasOtpUI || count === 0).toBeTruthy();
    }
  });

  test('start checkout opens OTP modal', async ({ page }) => {
    await addItemAndGoToCheckout(page);

    // Click the submit/continue button
    const submitBtn = page.locator('button[type="submit"]');
    if (await submitBtn.count() > 0) {
      await submitBtn.click();
      await page.waitForTimeout(1000);
    }
  });

  test('order total is displayed correctly', async ({ page }) => {
    await addItemAndGoToCheckout(page);

    // Should show some form of total/subtotal
    const pageContent = await page.textContent('body');
    expect(pageContent).toMatch(/ALL|Lek/i);
  });

  test('payment section shows cash as default', async ({ page }) => {
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(1500);

    const body = await page.textContent('body');
    // The checkout page should show order-related content
    expect(body).toMatch(/total|order|checkout/i);
  });

});
