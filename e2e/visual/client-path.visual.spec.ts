/**
 * CLIENT critical-path visual snapshot spec — the eater journey (menu → product → cart → checkout
 * → order status), every screen × state × locale, frozen as baselines.
 *
 * One spec, three breakpoints: the viewport is supplied by the PROJECT (mobile-390 / tablet-768 /
 * desktop-1280 in playwright.visual.config.ts) — we DO NOT loop breakpoints here. We loop LOCALES.
 *
 * Forced states (loading / empty / error) are driven by `page.route(...)` network mocks so the
 * baseline is deterministic regardless of seed timing. Real states (closed / busy / stoplist /
 * status) ride the seeded fixtures. Every snapshot masks the dynamic zone via MASK(page).
 *
 * Selectors are READ from the live components — never invented:
 *   - MenuPage.tsx       product card [data-testid="menu-item"] (click → opens [role="dialog"]),
 *                        add button [data-testid="menu-item-add"], venue chip/banners.
 *   - ClientLayout.tsx   cart trigger [data-testid="cart-open"] (renders only when items>0),
 *                        cart checkout [data-testid="cart-checkout"], empty = "Cart is empty".
 *   - CheckoutPage.tsx   single-page form; sections map to the "3 steps": Contact ([data-testid=
 *                        "checkout-phone"]) · Delivery map (MapWithPin) · Payment ([data-testid=
 *                        "checkout-total"]); submit [data-testid="order-confirm-button"]; error
 *                        banner role="alert" + [data-testid="checkout-call-restaurant"].
 *   - OrderStatusPage.tsx success [data-testid="order-status-badge"]; not-found / dead link →
 *                        [data-testid="order-back-to-menu"].
 *
 * Canonical routes (ClientRoutes.tsx, mounted at /s/:slug/*):
 *   menu     /s/:slug
 *   checkout /s/:slug/checkout
 *   status   /s/:slug/order/:id
 */
import { readFileSync } from 'node:fs';
import { test, expect, type Page } from '@playwright/test';
import { setLocale, MASK, settle, LOCALES } from './harness.js';
import type { VisualFixtures } from './harness.js';

const FIXTURES_PATH = 'e2e/visual/.fixtures.json';

// Read the fixtures seeded by globalSetup. We read the JSON directly (ESM `fs`) rather than via
// global-setup's readFixtures(), which uses `require` and throws under ESM. At collection time the
// file may not exist yet (globalSetup runs only for an actual run, not for --list) — fall back to
// inert placeholders so the spec always parses; real values are present whenever tests execute.
function loadFixtures(): VisualFixtures {
  try {
    return JSON.parse(readFileSync(FIXTURES_PATH, 'utf8')) as VisualFixtures;
  } catch {
    return {
      open: { slug: 'visual-open', locationId: '00000000-0000-0000-0000-000000000001' },
      closed: { slug: 'visual-closed', locationId: '00000000-0000-0000-0000-000000000002' },
      busy: { slug: 'visual-busy', locationId: '00000000-0000-0000-0000-000000000003' },
      stoplistProductId: '00000000-0000-0000-0000-0000000000aa',
      orderId: 'o_visual_seed',
      courierId: '00000000-0000-0000-0000-0000000000bb',
    };
  }
}

const FX = loadFixtures();

/** A deterministic menu payload (one category, one available product) for forced states. */
function mockMenu(extra: Record<string, unknown> = {}) {
  return {
    menu_version: 1,
    default_locale: 'sq',
    supported_locales: ['sq', 'en'],
    currency: { code: 'ALL', minor_unit: 0 },
    location_name: 'Visual Fixture',
    categories: [
      {
        id: 'cat-1',
        name: 'Pizza',
        sort_order: 0,
        products: [
          { id: 'p-1', name: 'Margherita', description: 'Tomato, mozzarella, basil', price: 800, available: true },
        ],
      },
    ],
    ...extra,
  };
}

/** Snapshot a full-page screenshot with the standard mask. */
async function shoot(page: Page, name: string) {
  await settle(page);
  await expect(page).toHaveScreenshot(name, { mask: MASK(page), fullPage: true });
}

// ───────────────────────────── MENU ─────────────────────────────
test.describe('menu', () => {
  for (const loc of LOCALES) {
    test(`menu-success-${loc}`, async ({ page }) => {
      await setLocale(page, loc);
      await page.goto(`/s/${FX.open.slug}`);
      await shoot(page, `menu-success-${loc}.png`);
    });

    test(`menu-loading-${loc}`, async ({ page }) => {
      await setLocale(page, loc);
      // Delay the menu fetch so the skeleton grid is the rendered state at snapshot time.
      await page.route('**/public/locations/*/menu*', async (route) => {
        await new Promise((r) => setTimeout(r, 3000));
        await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(mockMenu()) });
      });
      await page.goto(`/s/${FX.open.slug}`, { waitUntil: 'commit' });
      // Don't settle to networkidle (the route is intentionally hung) — wait for the skeleton.
      await page.locator('.skeleton-block').first().waitFor({ state: 'visible' });
      await page.waitForTimeout(400);
      await expect(page).toHaveScreenshot(`menu-loading-${loc}.png`, { mask: MASK(page), fullPage: true });
    });

    test(`menu-empty-${loc}`, async ({ page }) => {
      await setLocale(page, loc);
      // Published venue with zero categories → the "menu unavailable / not published yet" state.
      await page.route('**/public/locations/*/menu*', (route) =>
        route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(mockMenu({ categories: [] })) }),
      );
      await page.goto(`/s/${FX.open.slug}`);
      await shoot(page, `menu-empty-${loc}.png`);
    });

    test(`menu-closed-${loc}`, async ({ page }) => {
      await setLocale(page, loc);
      await page.goto(`/s/${FX.closed.slug}`);
      await shoot(page, `menu-closed-${loc}.png`);
    });

    test(`menu-busy-${loc}`, async ({ page }) => {
      await setLocale(page, loc);
      await page.goto(`/s/${FX.busy.slug}`);
      await shoot(page, `menu-busy-${loc}.png`);
    });

    test(`menu-stoplist-${loc}`, async ({ page }) => {
      await setLocale(page, loc);
      // Open venue carrying an 86'd product (stoplistProductId) — the unavailable card variant.
      await page.goto(`/s/${FX.open.slug}`);
      await shoot(page, `menu-stoplist-${loc}.png`);
    });
  }
});

// ─────────────────────────── PRODUCT MODAL ───────────────────────────
test.describe('product-modal', () => {
  for (const loc of LOCALES) {
    test(`product-modal-open-${loc}`, async ({ page }) => {
      await setLocale(page, loc);
      await page.goto(`/s/${FX.open.slug}`);
      await settle(page);
      // Click a product card (not its add button) → opens the detail bottom-sheet [role="dialog"].
      await page.locator('[data-testid="menu-item"]').first().click();
      await page.locator('[role="dialog"]').waitFor({ state: 'visible' });
      await page.waitForTimeout(400);
      await expect(page).toHaveScreenshot(`product-modal-open-${loc}.png`, { mask: MASK(page), fullPage: true });
    });

    test(`product-modal-with-modifiers-${loc}`, async ({ page }) => {
      await setLocale(page, loc);
      // Force a product that carries a required modifier group so the modal renders the
      // option controls (radio/checkbox) deterministically.
      await page.route('**/public/locations/*/menu*', (route) =>
        route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(
            mockMenu({
              categories: [
                {
                  id: 'cat-1',
                  name: 'Pizza',
                  sort_order: 0,
                  products: [
                    {
                      id: 'p-mod',
                      name: 'Build-your-own',
                      description: 'Pick a size',
                      price: 900,
                      available: true,
                      modifier_groups: [
                        {
                          id: 'g-size',
                          name: 'Size',
                          min_select: 1,
                          max_select: 1,
                          required: true,
                          sort_order: 0,
                          display_type: 'radio',
                          modifiers: [
                            { id: 'm-s', name: 'Small', price_delta: 0, available: true, sort_order: 0 },
                            { id: 'm-l', name: 'Large', price_delta: 200, available: true, sort_order: 1 },
                          ],
                        },
                      ],
                    },
                  ],
                },
              ],
            }),
          ),
        }),
      );
      await page.goto(`/s/${FX.open.slug}`);
      await settle(page);
      await page.locator('[data-testid="menu-item"]').first().click();
      await page.locator('[role="dialog"]').waitFor({ state: 'visible' });
      await page.waitForTimeout(400);
      await expect(page).toHaveScreenshot(`product-modal-with-modifiers-${loc}.png`, { mask: MASK(page), fullPage: true });
    });

    test(`product-modal-unavailable-${loc}`, async ({ page }) => {
      await setLocale(page, loc);
      // The stoplist product, surfaced as available:false → modal shows the "Unavailable" badge
      // and a disabled add. Forced via mock so the unavailable product is guaranteed present.
      await page.route('**/public/locations/*/menu*', (route) =>
        route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(
            mockMenu({
              categories: [
                {
                  id: 'cat-1',
                  name: 'Pizza',
                  sort_order: 0,
                  products: [
                    { id: FX.stoplistProductId || 'p-86', name: 'Sold-out Special', description: 'Currently 86’d', price: 700, available: false },
                  ],
                },
              ],
            }),
          ),
        }),
      );
      await page.goto(`/s/${FX.open.slug}`);
      await settle(page);
      await page.locator('[data-testid="menu-item"]').first().click();
      await page.locator('[role="dialog"]').waitFor({ state: 'visible' });
      await page.waitForTimeout(400);
      await expect(page).toHaveScreenshot(`product-modal-unavailable-${loc}.png`, { mask: MASK(page), fullPage: true });
    });
  }
});

// ───────────────────────────── CART ─────────────────────────────
test.describe('cart', () => {
  for (const loc of LOCALES) {
    test(`cart-success-${loc}`, async ({ page }) => {
      await setLocale(page, loc);
      await page.goto(`/s/${FX.open.slug}`);
      await settle(page);
      // Add the first item via the card's add button, then open the cart dialog. The cart
      // trigger only mounts once items>0 (ClientLayout) — adding first is required.
      await page.locator('[data-testid="menu-item-add"]').first().click();
      await page.locator('[data-testid="cart-open"]').waitFor({ state: 'visible' });
      await page.locator('[data-testid="cart-open"]').click();
      await page.locator('[data-testid="cart-checkout"]').waitFor({ state: 'visible' });
      await page.waitForTimeout(400);
      await expect(page).toHaveScreenshot(`cart-success-${loc}.png`, { mask: MASK(page), fullPage: true });
    });

    test(`cart-empty-${loc}`, async ({ page }) => {
      await setLocale(page, loc);
      // The cart trigger is hidden with an empty cart, so force-open the dialog by adding then
      // removing — instead we render the empty-cart dialog via the menu's own empty state. The
      // empty cart body ("Cart is empty") lives in the ResponsiveDialog; open it after a single
      // add+remove so the dialog is mounted with zero items.
      await page.goto(`/s/${FX.open.slug}`);
      await settle(page);
      await page.locator('[data-testid="menu-item-add"]').first().click();
      await page.locator('[data-testid="cart-open"]').click();
      // Decrement to empty (qty 1 → 0 removes the line) using the minus control inside the dialog.
      const dialog = page.locator('[role="dialog"]');
      await dialog.locator('button:has(.ti-minus)').first().click();
      await expect(dialog.getByText(/Cart is empty|Shporta është bosh/i)).toBeVisible();
      await page.waitForTimeout(400);
      await expect(page).toHaveScreenshot(`cart-empty-${loc}.png`, { mask: MASK(page), fullPage: true });
    });
  }
});

// ─────────────────────────── CHECKOUT ───────────────────────────
// CheckoutPage is a single scrolling form; the "3 steps" are its three cards. We seed the cart
// (via menu add) then navigate to /checkout for every checkout snapshot.
async function gotoCheckoutWithCart(page: Page, loc: 'al' | 'en') {
  await setLocale(page, loc);
  await page.goto(`/s/${FX.open.slug}`);
  await settle(page);
  await page.locator('[data-testid="menu-item-add"]').first().click();
  await page.locator('[data-testid="cart-open"]').waitFor({ state: 'visible' });
  await page.goto(`/s/${FX.open.slug}/checkout`);
  await settle(page);
}

test.describe('checkout', () => {
  for (const loc of LOCALES) {
    test(`checkout-step1-phone-${loc}`, async ({ page }) => {
      await gotoCheckoutWithCart(page, loc);
      // Step 1 — Contact Info card with the phone field.
      await page.locator('[data-testid="checkout-phone"]').waitFor({ state: 'visible' });
      await expect(page).toHaveScreenshot(`checkout-step1-phone-${loc}.png`, { mask: MASK(page), fullPage: true });
    });

    test(`checkout-step2-map-${loc}`, async ({ page }) => {
      await gotoCheckoutWithCart(page, loc);
      // Step 2 — the delivery map-pin card (MapWithPin). Scroll it into view, then snapshot the page.
      await page.locator('[data-testid="checkout-entrance"]').scrollIntoViewIfNeeded();
      await page.waitForTimeout(400);
      await expect(page).toHaveScreenshot(`checkout-step2-map-${loc}.png`, { mask: MASK(page), fullPage: true });
    });

    test(`checkout-step3-payment-${loc}`, async ({ page }) => {
      await gotoCheckoutWithCart(page, loc);
      // Step 3 — payment / order-summary card with the total + submit button.
      await page.locator('[data-testid="checkout-total"]').scrollIntoViewIfNeeded();
      await page.waitForTimeout(400);
      await expect(page).toHaveScreenshot(`checkout-step3-payment-${loc}.png`, { mask: MASK(page), fullPage: true });
    });

    test(`checkout-error-${loc}`, async ({ page }) => {
      // POST /api/orders → 500: assert the designed fallback (error banner) + the cart is preserved
      // (the menu add survives because clearCart only runs on success).
      await page.route('**/api/orders', (route) =>
        route.fulfill({ status: 500, contentType: 'application/json', body: JSON.stringify({ message: 'boom' }) }),
      );
      await gotoCheckoutWithCart(page, loc);
      // Fill the minimum required fields for a delivery order so submit reaches the POST.
      await page.locator('[data-testid="checkout-phone"]').fill('+355691234567');
      await page.getByPlaceholder(/Your name|Emri juaj/i).first().fill('Test Eater');
      await page.locator('[data-testid="checkout-entrance"]').fill('A');
      await page.locator('[data-testid="checkout-apartment"]').fill('3');
      await page.locator('textarea').first().fill('Blue gate by the bakery');
      await page.locator('[data-testid="order-confirm-button"]').click();
      // The error banner is the proof of the fallback state.
      await page.locator('[role="alert"]').first().waitFor({ state: 'visible' });
      await page.waitForTimeout(400);
      await expect(page).toHaveScreenshot(`checkout-error-${loc}.png`, { mask: MASK(page), fullPage: true });
    });
  }
});

// ──────────────────────────── ORDER STATUS ────────────────────────────
test.describe('status', () => {
  for (const loc of LOCALES) {
    test(`status-success-${loc}`, async ({ page }) => {
      await setLocale(page, loc);
      await page.goto(`/s/${FX.open.slug}/order/${FX.orderId}`);
      await settle(page);
      // The status badge is the load-bearing element of the live tracking screen.
      await page.locator('[data-testid="order-status-badge"]').waitFor({ state: 'visible' });
      await page.waitForTimeout(400);
      await expect(page).toHaveScreenshot(`status-success-${loc}.png`, { mask: MASK(page), fullPage: true });
    });

    test(`status-not-found-${loc}`, async ({ page }) => {
      await setLocale(page, loc);
      // A bad id resolves to the "Order not found" EmptyState with a back-to-menu escape.
      await page.goto(`/s/${FX.open.slug}/order/o_does_not_exist`);
      await settle(page);
      await page.locator('[data-testid="order-back-to-menu"]').waitFor({ state: 'visible' });
      await page.waitForTimeout(400);
      await expect(page).toHaveScreenshot(`status-not-found-${loc}.png`, { mask: MASK(page), fullPage: true });
    });
  }
});
