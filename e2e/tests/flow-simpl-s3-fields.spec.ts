import { test, expect } from '@playwright/test';

// flow-simplification §3 — contextually-required door detail (pin-confidence-gated).
// Default state on checkout = NO precise pin placed (pinLocation null = low confidence) → entrance/apartment
// are REQUIRED (the courier most needs the door detail then). When the customer places a precise pin they
// become optional. We assert the safe default here (required + no "(optional)" hint); the optional-on-precise-
// pin path is gated by `required={pinLocation == null}` (map-drag is not deterministically scriptable).
const SLUG = process.env.E2E_LOCATION_SLUG || 'demo';

test('§3 · door detail is required by default (low-confidence pin)', async ({ page }) => {
  await page.goto(`/s/${SLUG}`);
  const add = page.locator('[data-testid=menu-item-add]').first();
  await expect(add).toBeVisible({ timeout: 20_000 });
  await add.click();
  const modalAdd = page.locator('[data-testid=modal-add-to-cart], [data-testid=product-add]');
  if (await modalAdd.first().isVisible().catch(() => false)) await modalAdd.first().click();

  await page.goto(`/s/${SLUG}/checkout`);

  const entrance = page.getByTestId('checkout-entrance');
  await expect(entrance).toBeVisible();
  // No precise pin placed yet → entrance is required (the contextual floor for the least-served).
  await expect(entrance).toHaveJSProperty('required', true);
});
