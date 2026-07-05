import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';

// QR/ATTRIBUTION build lane. `?ch=<value>` on `/s/:slug` is captured into sessionStorage
// (apps/web/src/lib/channel.ts) and travels write-only into order creation as the
// `x-channel` request header (apps/api/src/lib/channel.ts folds it into the existing
// orders.metadata jsonb — see docs/design/order-channel-attribution/). It is never read
// for pricing/status/dispatch, so there's no DOM-visible price/behavior difference to
// assert on — the proof is (a) the captured sessionStorage value and (b) the actual
// `x-channel` header on the live POST /orders request, intercepted so no real order is
// created against the deployed backend (same pattern as flow-simpl-s1-velocity-frictionless.spec.ts).
const SLUG = process.env.E2E_LOCATION_SLUG || 'demo';

/**
 * Staging's global limiter is 100 req/min per client IP (server.ts fastifyRateLimit) — a
 * multi-project run of this file alone exceeds it (12 page loads × ~25 asset/API requests).
 * That 429 is environmental (self-inflicted test load), not the feature under test, so
 * navigation retries with the server-provided Retry-After — bounded, explicit, and the
 * feature assertions themselves stay untouched.
 */
async function gotoThroughRateLimit(page: import('@playwright/test').Page, url: string) {
  for (let attempt = 1; ; attempt++) {
    const resp = await page.goto(url);
    if (!resp || resp.status() !== 429) return resp;
    if (attempt >= 4) return resp;
    let waitMs = 8_000;
    try {
      waitMs = Math.max(JSON.parse(await resp.text()).retryAfterMs ?? 0, 5_000) + 2_000;
    } catch { /* keep default */ }
    await page.waitForTimeout(waitMs);
  }
}

// NOTE on waiting: `#root` is the SPA-shell div and is "visible" BEFORE React mounts, so an
// immediate sessionStorage read races the capture effect (proven live: the value appears once
// the app hydrates). Capture assertions therefore POLL; the negative case first waits for a
// React-rendered element (menu add button) so "still null" means "app ran and captured nothing",
// not "app hasn't run yet".
test('?ch=qr is captured per-slug into sessionStorage on /s/:slug', async ({ page }) => {
  const resp = await gotoThroughRateLimit(page, `/s/${SLUG}?ch=qr`);
  expect(resp?.status(), `GET /s/${SLUG}?ch=qr`).toBeLessThan(400);

  await expect
    .poll(() => page.evaluate((slug) => sessionStorage.getItem(`dos_channel:${slug}`), SLUG), {
      message: 'sessionStorage dos_channel:<slug> captures the allowlisted value',
      timeout: 20_000,
    })
    .toBe('qr');
});

test('an unrecognized ?ch= value normalizes to "other", not silently dropped', async ({ page }) => {
  await gotoThroughRateLimit(page, `/s/${SLUG}?ch=some-random-billboard`);

  await expect
    .poll(() => page.evaluate((slug) => sessionStorage.getItem(`dos_channel:${slug}`), SLUG), {
      message: 'unknown channel -> other (never blocks capture, never passes through raw)',
      timeout: 20_000,
    })
    .toBe('other');
});

test('no ?ch= param leaves no captured channel (order creation then defaults to web-direct server-side)', async ({ page }) => {
  await gotoThroughRateLimit(page, `/s/${SLUG}`);
  // App fully mounted + menu rendered — the capture effect has definitely run by now.
  await expect(page.locator('[data-testid=menu-item-add]').first()).toBeVisible({ timeout: 20_000 });

  const stored = await page.evaluate((slug) => sessionStorage.getItem(`dos_channel:${slug}`), SLUG);
  expect(stored, 'nothing captured -> getOrderChannel falls back to web-direct at send time').toBeNull();
});

test('?ch=qr on /s/:slug -> the POST /orders request carries x-channel: qr (order-creation propagation)', async ({ page }) => {
  // Full checkout flow + possible 429 backoffs (see gotoThroughRateLimit) — the default 30s
  // test budget expires inside the environmental wait, not the assertions.
  test.setTimeout(120_000);
  let channelHeader: string | null | undefined;
  let orderPosted = false;

  await page.route('**/api/orders', async (route) => {
    orderPosted = true;
    channelHeader = await route.request().headerValue('x-channel');
    // Fulfill locally — no real order/DB row is created against the deployed backend.
    await route.fulfill({
      status: 201,
      contentType: 'application/json',
      body: JSON.stringify({ id: 'e2e-channel-attribution-order', status: 'PENDING', total: 1700 }),
    });
  });

  await gotoThroughRateLimit(page, `/s/${SLUG}?ch=qr`);
  await expect(page.locator('#root')).toBeVisible();

  const add = page.locator('[data-testid=menu-item-add]').first();
  await expect(add).toBeVisible({ timeout: 20_000 });
  await add.click();
  // The product modal animates in — a same-tick isVisible() misses it and the item never
  // lands in the cart (then checkout has nothing to submit). Give it a short explicit window.
  const modalAdd = page.locator('[data-testid=modal-add-to-cart], [data-testid=product-add]');
  await modalAdd
    .first()
    .waitFor({ state: 'visible', timeout: 3_000 })
    .then(() => modalAdd.first().click())
    .catch(() => {}); // no modal for simple items — direct add already happened

  await page.getByTestId('cart-open').click();
  await page.getByTestId('cart-checkout').click();

  const phone = page.getByTestId('checkout-phone');
  await expect(phone).toBeVisible({ timeout: 15_000 });
  await page.locator('input[autocomplete="name"]').first().fill('Channel Attribution E2E');
  await phone.fill(`+3556${crypto.randomInt(10_000_000, 99_999_999)}`);
  await page.getByTestId('checkout-address').fill('Rruga Test 1');
  await page.getByTestId('checkout-entrance').fill('1');
  await page.getByTestId('checkout-apartment').fill('2');
  await page.locator('#checkout-form textarea').first().fill('Blue gate by the square');

  await page.getByTestId('order-confirm-button').click();

  await expect(page).toHaveURL(/\/s\/.+\/order\/e2e-channel-attribution-order/, { timeout: 25_000 });

  expect(orderPosted, 'POST /orders must have fired').toBe(true);
  expect(channelHeader, 'x-channel header must carry the captured qr attribution').toBe('qr');
});
