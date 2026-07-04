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

test('?ch=qr is captured per-slug into sessionStorage on /s/:slug', async ({ page }) => {
  const resp = await page.goto(`/s/${SLUG}?ch=qr`);
  expect(resp?.status(), `GET /s/${SLUG}?ch=qr`).toBeLessThan(400);
  await expect(page.locator('#root')).toBeVisible();

  const stored = await page.evaluate((slug) => sessionStorage.getItem(`dos_channel:${slug}`), SLUG);
  expect(stored, 'sessionStorage dos_channel:<slug> captures the allowlisted value').toBe('qr');
});

test('an unrecognized ?ch= value normalizes to "other", not silently dropped', async ({ page }) => {
  await page.goto(`/s/${SLUG}?ch=some-random-billboard`);
  await expect(page.locator('#root')).toBeVisible();

  const stored = await page.evaluate((slug) => sessionStorage.getItem(`dos_channel:${slug}`), SLUG);
  expect(stored, 'unknown channel -> other (never blocks capture, never passes through raw)').toBe('other');
});

test('no ?ch= param leaves no captured channel (order creation then defaults to web-direct server-side)', async ({ page }) => {
  await page.goto(`/s/${SLUG}`);
  await expect(page.locator('#root')).toBeVisible();

  const stored = await page.evaluate((slug) => sessionStorage.getItem(`dos_channel:${slug}`), SLUG);
  expect(stored, 'nothing captured -> getOrderChannel falls back to web-direct at send time').toBeNull();
});

test('?ch=qr on /s/:slug -> the POST /orders request carries x-channel: qr (order-creation propagation)', async ({ page }) => {
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

  await page.goto(`/s/${SLUG}?ch=qr`);
  await expect(page.locator('#root')).toBeVisible();

  const add = page.locator('[data-testid=menu-item-add]').first();
  await expect(add).toBeVisible({ timeout: 20_000 });
  await add.click();
  const modalAdd = page.locator('[data-testid=modal-add-to-cart], [data-testid=product-add]');
  if (await modalAdd.first().isVisible().catch(() => false)) await modalAdd.first().click();

  await page.getByTestId('cart-open').click();
  await page.getByTestId('cart-checkout').click();

  const phone = page.getByTestId('checkout-phone');
  await expect(phone).toBeVisible({ timeout: 10_000 });
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
