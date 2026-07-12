import { test, expect } from '@playwright/test';

// Non-prod default: this spec only reads + mutates client-side cart (localStorage),
// it never POSTs an order, so no requireStaging guard is needed — but the BASE
// default must not point at prod (Test Integrity §6).
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

// Regression: the customer order flow must reach a rendered checkout, not an
// error or an empty shell. /s/:slug (human UA) is the React SPA storefront; its
// add-to-cart button is [data-testid=menu-item-add] and the cart persists to
// localStorage (dos_cart_<locationId>, see CartProvider) across the full-page
// nav to /s/:slug/checkout, where CheckoutPage renders order-confirm-button for
// a non-empty, successfully-loaded cart. A prior schema mismatch threw inside
// the checkout and a silent catch disguised it, so a customer could add but
// never check out.
test.describe('Customer order flow — menu → checkout renders', () => {
  test('adding an item then visiting /checkout shows the cart, not an error', async ({ page }) => {
    await page.goto(`${BASE}/s/demo`);

    const addBtn = page.getByTestId('menu-item-add').first();
    await expect(addBtn).toBeVisible({ timeout: 15_000 });
    await addBtn.click();

    // CRITICAL: prove the click actually mutated the cart BEFORE navigating away.
    // The cart sticky-bar (cart-open) renders ONLY when itemsCount > 0
    // (ClientLayout) — a silently-failed/detached click leaves it absent → red,
    // instead of sailing on to the empty-cart shell and passing green.
    await expect(page.getByTestId('cart-open')).toBeVisible({ timeout: 10_000 });

    await page.goto(`${BASE}/s/demo/checkout`);
    const root = page.locator('#root');
    await expect(root).toBeVisible({ timeout: 15_000 });

    // Positive render proof: order-confirm-button renders only for a non-empty,
    // successfully-loaded checkout. The empty-cart shell (items.length===0 early
    // return) and the error boundary do NOT render it — so this goes red on both,
    // unlike a negative-absence text check on a broad container.
    await expect(page.getByTestId('order-confirm-button')).toBeVisible({ timeout: 15_000 });
    // Secondary tripwire (NOT the sole proof): no error-boundary text leaked in.
    await expect(root).not.toContainText('went wrong');
  });

  // Error matrix: when the checkout's /info load fails (5xx / network), the page
  // MUST surface its location-load-failed alert (role=alert), never a silent
  // blank or an infinite spinner that a loose body-text check would pass.
  test('checkout surfaces the location-load error state when /info fails', async ({ page }) => {
    await page.goto(`${BASE}/s/demo`);
    const addBtn = page.getByTestId('menu-item-add').first();
    await expect(addBtn).toBeVisible({ timeout: 15_000 });
    await addBtn.click();
    // Need a non-empty cart so checkout renders its body (the error block lives
    // past the items.length===0 early return), proving cart-state first.
    await expect(page.getByTestId('cart-open')).toBeVisible({ timeout: 10_000 });

    // Fail the location-info fetch the checkout depends on (CheckoutPage:264 →
    // .catch → setLocationLoadFailed(true)). abort() exercises the network-failure
    // dimension; the !r.ok throw covers the 5xx dimension identically.
    await page.route('**/public/locations/*/info', route => route.abort());

    await page.goto(`${BASE}/s/demo/checkout`);
    await expect(page.getByTestId('checkout-location-load-failed')).toBeVisible({ timeout: 15_000 });
  });

  // TODO(needs-staging): cross-tenant cart isolation. The cart key is
  // dos_cart_<locationId> (CartProvider) — isolation is per-location — but
  // asserting no bleed requires a REAL second tenant's slug with a distinct
  // locationId (a nil/guessed slug would 404 by absence and prove nothing, per
  // Test Integrity §5). Add a test: add to /s/demo, visit /s/<second-tenant>,
  // assert cart-open is NOT visible there. Blocked on a confirmed 2nd staging
  // tenant fixture. Unknown-slug 404 is already covered in api-real.spec.ts.
});
