import { test, expect } from '@playwright/test';

test.describe('Client Checkout', () => {

  async function addItemAndGoToCheckout(page: any) {
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });
    await page.locator('[data-testid="menu-item-add"]').first().click();
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
    await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible({ timeout: 5000 });
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
    // Assert the actual delivery-type toggle widget (role=tablist), not loose body text
    // that also appears in nav/labels/title. Switching a tab must flip aria-selected.
    const tablist = page.locator('[role="tablist"]');
    await expect(tablist).toBeVisible({ timeout: 5000 });
    const tabs = tablist.locator('[role="tab"]');
    await expect(tabs).toHaveCount(2); // delivery + pickup (scheduled is hidden scaffold)
    await tabs.nth(1).click();
    await expect(tabs.nth(1)).toHaveAttribute('aria-selected', 'true');
  });

  test('address input field is present for delivery', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(1000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    // Assert delivery-specific address fields (rendered only when deliveryType==='delivery'),
    // not any input — the phone field alone would satisfy a generic count>=1.
    await expect(page.locator('[data-testid="checkout-entrance"]')).toBeVisible({ timeout: 5000 });
    await expect(page.locator('[data-testid="checkout-apartment"]')).toBeVisible();
  });

  test('phone input or OTP modal is present on checkout', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    // Require a VISIBLE phone field or a VISIBLE OTP dialog — not a class-name match
    // (`[class*="otp"]`) that hits display:none elements. Both branches assert visibility.
    const phoneField = page.locator('[data-testid="checkout-phone"]');
    const otpModal = page.locator('[role="dialog"]');
    const hasPhone = await phoneField.isVisible();
    const hasOtp = await otpModal.isVisible();
    expect(hasPhone || hasOtp).toBe(true);
  });

  test('start checkout opens OTP modal', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    // The place-order/start-checkout control must exist and be visible (no conditional
    // guard that lets the test pass when the button is absent). The full OTP-open flow
    // requires a valid filled form + OTP_ENABLED on the target — see needs_staging TODO.
    const submitBtn = page.locator('[data-testid="order-confirm-button"]');
    await expect(submitBtn).toBeVisible({ timeout: 5000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    // TODO(needs_staging): fill name/phone/address, click submit, assert the OTP [role="dialog"]
    // becomes visible (or the order is created) against staging with OTP_ENABLED=true.
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
    // Cash is the default method → its amount input (#cash-amount) renders. Targeted locator,
    // not /total|order|checkout/ which appears on every checkout page regardless of payment state.
    await expect(page.locator('#cash-amount')).toBeVisible({ timeout: 5000 });
  });

  test('no cookies set on checkout page', async ({ page }) => {
    await addItemAndGoToCheckout(page);
    // Positive control: prove the checkout page actually rendered before asserting empty cookies —
    // otherwise an error page / failed nav would also yield [] and give a false security green.
    await expect(page.locator('[data-testid="checkout-total"]')).toBeVisible({ timeout: 5000 });
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('CR-1: dropoff instruction buttons present on delivery checkout', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(2000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const dropoffBtns = page.locator('button:has-text("Leave at door"),button:has-text("Hand to me"),button:has-text("Call on arrival"),button:has-text("Ring bell"),button:has-text("Text on arrival")');
    const count = await dropoffBtns.count();
    expect(count).toBeGreaterThanOrEqual(1);
  });

  test('CR-3: cash payment amount field renders with total minimum', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(2000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const cashInput = page.locator('input[type="number"]');
    await expect(cashInput).toBeVisible({ timeout: 5000 });
  });

  test('CR-3: payment section renders cash as default', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(2000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body).toMatch(/Cash|ALL/i);
  });

  test('checkout page renders order summary with ALL currency', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await addItemAndGoToCheckout(page);
    await page.waitForTimeout(2000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const totalEl = page.locator('text=/[0-9]+[\\s]*ALL/');
    await expect(totalEl.first()).toBeVisible({ timeout: 5000 });
  });

  // TODO(needs_staging): cross-tenant isolation. Add a test that places/loads a cart under
  // tenant A's slug then attempts checkout under a REAL second tenant's slug (not a nil/all-zero
  // id — that 404s by absence and proves nothing) and asserts tenant A's cart/order is neither
  // visible nor submittable under tenant B. Requires a real second seeded tenant on staging.

  // TODO(needs_staging): error-matrix on order-create. With the full form filled, intercept the
  // create call (page.route('**/api/orders', r => r.fulfill({ status: 422 | 429 | 503 })) and
  // r.abort() for network failure) and assert the role="alert" error banner renders for each —
  // not a silent no-op. Requires the full valid-form submit flow against staging.

});
