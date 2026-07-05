import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';

// flow-simplification §1 — a SOFT velocity confirm must NOT add customer friction. The anti-fake-signals
// gate returns 200 { outcome:'soft_confirm', reasons:[{code:'velocity'}], requiresConfirmation:true } for a
// frequent device (verified live on staging). The checkout must auto-acknowledge that soft reason and
// resubmit ONCE silently, landing the customer on the order page — never dead-end on the generic
// "order failed / call the restaurant" banner. The /api/orders contract is stubbed so the proof is
// deterministic (the live soft_confirm shape is the one captured from staging); the FE code under test is real.
const SLUG = process.env.E2E_LOCATION_SLUG || 'demo';

test('§1 · a velocity soft-confirm auto-acknowledges and places the order (no added friction)', async ({ page }) => {
  let orderPosts = 0;
  let secondBody: any = null;

  await page.route('**/api/orders', async (route) => {
    orderPosts += 1;
    if (orderPosts === 1) {
      // First attempt → the live velocity soft-confirm body.
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          outcome: 'soft_confirm',
          reasons: [{ code: 'velocity', severity: 'soft', message: 'Unusually many orders from this device.' }],
          requiresOtp: false,
          requiresConfirmation: true,
        }),
      });
      return;
    }
    // Second attempt → must carry the acknowledged code; respond with a created order.
    secondBody = route.request().postDataJSON();
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ id: 'velo-ack-order-1', status: 'PENDING', total: 1700 }),
    });
  });

  await page.goto(`/s/${SLUG}`);
  const add = page.locator('[data-testid=menu-item-add]').first();
  await expect(add).toBeVisible({ timeout: 20_000 });
  await add.click();
  const modalAdd = page.locator('[data-testid=modal-add-to-cart], [data-testid=product-add]');
  if (await modalAdd.first().isVisible().catch(() => false)) await modalAdd.first().click();

  await page.getByTestId('cart-open').click();
  await page.getByTestId('cart-checkout').click();

  const phone = page.getByTestId('checkout-phone');
  await expect(phone).toBeVisible({ timeout: 10_000 });
  await page.locator('input[autocomplete="name"]').first().fill('Velocity Regular');
  await phone.fill(`+3556${crypto.randomInt(10_000_000, 99_999_999)}`);
  await page.getByTestId('checkout-address').fill('Rruga Test 1');
  await page.getByTestId('checkout-entrance').fill('1');
  await page.getByTestId('checkout-apartment').fill('2');
  await page.locator('#checkout-form textarea').first().fill('Blue gate by the square');

  await page.getByTestId('order-confirm-button').click();

  // The frictionless outcome: lands on the order status page (the silent retry succeeded).
  await expect(page).toHaveURL(/\/s\/.+\/order\/velo-ack-order-1/, { timeout: 25_000 });

  // Proof of mechanism: exactly two POSTs, and the second acknowledged the velocity soft-reason.
  expect(orderPosts, 'auto-retry must fire exactly once (no loop)').toBe(2);
  expect(secondBody?.acknowledged_codes, 'second submit must acknowledge the soft reason').toContain('velocity');

  // The generic failure banner must NEVER have shown.
  await expect(page.getByText(/dështoi|could not be placed|telefono restorantin/i)).toHaveCount(0);
});
