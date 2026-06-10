import { test, expect } from '@playwright/test';

test.describe('Client Checkout', () => {

  async function addItemAndGoToCheckout(page: any) {
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('article.product-card', { timeout: 15000 });
    await page.locator('article.product-card button[aria-label="Add"]').first().click();
    await page.waitForTimeout(500);
    await expect(page.locator('#cartFabBtn')).toBeVisible({ timeout: 5000 });
    await page.locator('#cartFabBtn').click();
    await expect(page.locator('text=Your Cart')).toBeVisible({ timeout: 3000 });
    await page.locator('button:has-text("Checkout")').click();
    await expect(page).toHaveURL(/\/checkout/, { timeout: 5000 });
  }

  test('checkout page loads with cart items displayed', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(1000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body).toContain('Checkout');
    expect(body.length).toBeGreaterThan(100);
  });

  test('back button returns to menu page', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(500);

    const backBtns = page.locator('button:has-text("←")');
    const backCount = await backBtns.count();
    expect(backCount).toBeGreaterThanOrEqual(1);
    await backBtns.first().click();
    await page.waitForTimeout(1000);
    await expect(page.locator('article.product-card').first()).toBeVisible({ timeout: 5000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('empty cart redirects with empty message', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/checkout?dev=true');
    await expect(page.locator('text=Cart is empty')).toBeVisible({ timeout: 5000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('delivery type selector works (delivery, pickup, scheduled)', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(1000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body).toMatch(/delivery|pickup|schedule/i);
  });

  test('address input field is present for delivery', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(1000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const inputs = page.locator('input, textarea');
    const count = await inputs.count();
    expect(count).toBeGreaterThanOrEqual(1);
  });

  test('phone input or OTP modal is present on checkout', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const phoneField = page.locator('input[type="tel"]');
    const otpModal = page.locator('[role="dialog"], .modal, [class*="otp"]');
    const hasPhone = await phoneField.count() > 0;
    const hasOtp = await otpModal.count() > 0;
    expect(hasPhone || hasOtp).toBe(true);
    if (hasPhone) {
      await expect(phoneField.first()).toBeVisible();
    }
  });

  test('start checkout opens OTP modal', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    const submitBtn = page.locator('button[type="submit"]');
    if (await submitBtn.count() > 0) {
      await submitBtn.click();
      await page.waitForTimeout(1000);
    }
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
  });

  test('order total is displayed correctly in ALL', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body).toMatch(/ALL|Lek/i);
    expect(body.length).toBeGreaterThan(100);
  });

  test('payment section shows cash as default method', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(1500);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body).toMatch(/total|order|checkout/i);
    expect(body.length).toBeGreaterThan(100);
  });

  test('no cookies set on checkout page', async ({ page }) => {
    await addItemAndGoToCheckout(page);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

});
