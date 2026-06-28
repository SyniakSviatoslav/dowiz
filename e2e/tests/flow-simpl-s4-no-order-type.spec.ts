import { test, expect } from '@playwright/test';

// flow-simplification §4 — the order-type switch is removed from checkout (delivery is the only live type;
// pickup/scheduled deferred). Proves the UI surface: no delivery-type tablist, and the delivery fields the
// (now sole) delivery flow needs are present. The order CONTRACT is unchanged (the payload still sends
// type:'delivery'), so the existing order-create E2E coverage still holds.
const SLUG = process.env.E2E_LOCATION_SLUG || 'demo';

test('§4 · checkout has no order-type (delivery/pickup) switch — delivery-only', async ({ page }) => {
  await page.goto(`/s/${SLUG}`);

  // Add the first available menu item to the cart (storefront add button).
  const add = page.locator('[data-testid=menu-item-add]').first();
  await expect(add).toBeVisible({ timeout: 20_000 });
  await add.click();
  // A product modal may open with its own add-to-cart; click it if present (otherwise the tap added directly).
  const modalAdd = page.locator('[data-testid=modal-add-to-cart], [data-testid=product-add]');
  if (await modalAdd.first().isVisible().catch(() => false)) await modalAdd.first().click();

  await page.goto(`/s/${SLUG}/checkout`);

  // The delivery-type tablist (aria-label "Delivery type", with deliver/pickup tabs) must be GONE.
  await expect(page.getByRole('tablist', { name: /delivery type/i })).toHaveCount(0);
  await expect(page.getByRole('tab', { name: /pickup/i })).toHaveCount(0);

  // The delivery flow is the only one and renders directly (entrance field is a delivery-only field).
  await expect(page.getByTestId('checkout-entrance')).toBeVisible();
});
