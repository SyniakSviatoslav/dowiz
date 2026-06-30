import { test, expect } from '@playwright/test';

// Proof for the two QA-loop fixes (2026-06-30):
//  Loop 1 — storefront a11y/i18n seams (taste labels, cart qty aria, free-delivery
//           progressbar name, geolocate button label)
//  Loop 2 — notification sound asset restored (/sounds/ping.wav 200)
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test qa-loops-2026-06-30 --reporter=list

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
const SLUG = 'demo';

test.describe('QA loops 2026-06-30 — fixes are live', () => {
  test('Loop2: notification sound asset is served (was 404)', async ({ request }) => {
    const resp = await request.get(`${BASE}/sounds/ping.wav`);
    expect(resp.status()).toBe(200);
    expect(resp.headers()['content-type'] || '').toMatch(/audio|octet-stream|wav/i);
  });

  test('Loop1: product modal taste axis has a visible label + accessible name', async ({ page }) => {
    await page.goto(`${BASE}/s/${SLUG}`, { waitUntil: 'networkidle' });
    // Margherita carries a taste profile (salty/etc).
    const card = page.locator('[data-testid="menu-item"]').filter({ hasText: 'Margherita' }).first();
    await card.click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible({ timeout: 5000 });
    // Each taste axis is a span aria-labelled "<Label>: <level>" — was icon-only before.
    const tasteChip = dialog.locator('span[aria-label*=": "]').first();
    await expect(tasteChip).toBeVisible();
    // and it surfaces a human-readable axis word (EN or SQ), not just icons.
    await expect(dialog.getByText(/Salty|Sweet|Sour|Rich|Spicy|Kripur|Ëmbël|Thartë|Pasur|Nxehtë/i).first()).toBeVisible();
  });

  test('Loop1: cart qty buttons + free-delivery progressbar have accessible names', async ({ page }) => {
    await page.goto(`${BASE}/s/${SLUG}`, { waitUntil: 'networkidle' });
    await page.locator('[data-testid="menu-item-add"], [data-testid="menu-item"] button').first().click();
    await page.getByTestId('cart-open').click();
    // qty buttons were nameless icon buttons before.
    await expect(page.getByRole('button', { name: /Decrease quantity|Ulni sasinë/i })).toBeVisible({ timeout: 5000 });
    await expect(page.getByRole('button', { name: /Increase quantity|Rritni sasinë/i })).toBeVisible();
    // progressbar had aria-valuenow but no accessible name.
    await expect(page.getByRole('progressbar', { name: /Free delivery progress|Progresi i dorëzimit falas/i })).toBeVisible();
  });

  test('Loop1: checkout geolocate button has a translated accessible name', async ({ page }) => {
    await page.goto(`${BASE}/s/${SLUG}`, { waitUntil: 'networkidle' });
    await page.locator('[data-testid="menu-item-add"], [data-testid="menu-item"] button').first().click();
    await page.getByTestId('cart-open').click();
    await page.getByTestId('cart-checkout').click();
    // hardcoded English title + no aria-label before; now aria-labelled & translated.
    await expect(page.getByRole('button', { name: /My location|Vendndodhja ime/i })).toBeVisible({ timeout: 8000 });
  });
});
