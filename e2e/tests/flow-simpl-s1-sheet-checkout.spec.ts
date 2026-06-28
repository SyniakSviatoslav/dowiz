import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';

// flow-simplification §1 — checkout is a bottom-sheet OVER the menu (no page navigation). Proves the full
// money path through the sheet: add item → cart → Checkout opens the sheet (URL stays /s/:slug) → fill the
// delivery fields → place the order → land on the order status page. Back/close keeps the cart (no trap).
const SLUG = process.env.E2E_LOCATION_SLUG || 'demo';

test('§1 · place an order through the checkout sheet (no page nav, lands on order status)', async ({ page }) => {
  await page.goto(`/s/${SLUG}`);

  // add the first item
  const add = page.locator('[data-testid=menu-item-add]').first();
  await expect(add).toBeVisible({ timeout: 20_000 });
  await add.click();
  const modalAdd = page.locator('[data-testid=modal-add-to-cart], [data-testid=product-add]');
  if (await modalAdd.first().isVisible().catch(() => false)) await modalAdd.first().click();

  // open the cart, then Checkout → the checkout SHEET rises over the menu (no route change)
  await page.getByTestId('cart-open').click();
  await page.getByTestId('cart-checkout').click();

  // §1 core: we never left the menu — the URL is still /s/:slug (NOT /s/:slug/checkout)
  await expect(page).toHaveURL(new RegExp(`/s/${SLUG}(\\?|$)`));
  const phone = page.getByTestId('checkout-phone');
  await expect(phone).toBeVisible({ timeout: 10_000 });

  // fill the delivery fields (no precise pin placed → entrance/apartment required, §3)
  await page.locator('input[autocomplete="name"]').first().fill('E2E Sheet');
  await phone.fill(`+3556${crypto.randomInt(10_000_000, 99_999_999)}`);
  await page.getByTestId('checkout-address').fill('Rruga Test 1');
  await page.getByTestId('checkout-entrance').fill('1');
  await page.getByTestId('checkout-apartment').fill('2');
  await page.locator('#checkout-form textarea').first().fill('Blue gate by the square'); // the required "how to find you" notes (locale-independent)

  // place the order through the sheet
  await page.getByTestId('order-confirm-button').click();

  // success → the order status page (the sheet closes on the route change)
  await expect(page).toHaveURL(/\/s\/.+\/order\/.+/, { timeout: 25_000 });
});
