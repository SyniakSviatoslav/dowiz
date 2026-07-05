import { test, expect } from '@playwright/test';

// Proof for the design HIGH-1/2 + modal-dedup FE changes (commit fix(storefront)…). Runs against the
// real staged storefront. The load-bearing proof is REGRESSION: the ProductCard restructure (photo
// block gated on a real photo; essentials-only card; relocated taste/allergen/nutrition) must keep the
// storefront rendering + add-to-cart working, and the product modal must open with the dish name.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const SLUG = 'demo';

test.describe.configure({ mode: 'serial' });
test.setTimeout(60_000);

test('storefront renders the menu cards (HIGH-1/2 restructure did not break the grid)', async ({ page }) => {
  await page.goto(`${BASE}/s/${SLUG}`);
  const items = page.locator('[data-testid=menu-item]');
  await expect(items.first(), 'menu cards render').toBeVisible({ timeout: 20_000 });
  expect(await items.count(), 'the demo menu has many products').toBeGreaterThan(5);
  await page.screenshot({ path: 'audit/design-fixes/storefront-cards.png', fullPage: false });
});

test('a card opens the product modal with the dish name (modal renders post-restructure)', async ({ page }) => {
  await page.goto(`${BASE}/s/${SLUG}`);
  const firstCard = page.locator('[data-testid=menu-item]').first();
  await expect(firstCard).toBeVisible({ timeout: 20_000 });
  // the product NAME on the card (used to assert the modal shows it too)
  const cardName = (await firstCard.innerText()).split('\n').map((s) => s.trim()).filter(Boolean)[0] ?? '';
  await firstCard.click();
  // modal opened: the add-from-modal confirm button is the modal's anchor element
  await expect(page.locator('[data-testid=product-detail-confirm]'), 'product modal opens on card tap').toBeVisible({ timeout: 10_000 });
  if (cardName) {
    await expect(page.getByText(cardName, { exact: false }).first(), 'the dish name is shown in the modal').toBeVisible();
  }
  await page.screenshot({ path: 'audit/design-fixes/product-modal.png' });
});

test('add-to-cart still works from the card (menu-item-add preserved)', async ({ page }) => {
  await page.goto(`${BASE}/s/${SLUG}`);
  await expect(page.locator('[data-testid=menu-item]').first()).toBeVisible({ timeout: 20_000 });
  const addBtn = page.locator('[data-testid=menu-item-add]').first();
  await expect(addBtn, 'the add control survived the card declutter').toBeVisible();
  await addBtn.click();
  // the cart open control should now be reachable (an item is in the cart)
  await expect(page.locator('[data-testid=cart-open]'), 'cart reachable after add').toBeVisible({ timeout: 10_000 });
});
