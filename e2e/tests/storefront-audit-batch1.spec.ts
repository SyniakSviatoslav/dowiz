import { test, expect } from '@playwright/test';

// Storefront owner-audit Batch 1 proof (against deployed staging /s/demo).
// Covers: closed-state gating + prominent banner, categories-as-filter, single price
// toggle, reveal/solar removal, footer contact rail (WhatsApp/call/maps, no fake socials),
// Ukrainian locale + real branding, and the simplified checkout privacy copy.
// Venue open/closed is wall-clock dependent, so the open/closed states are forced via
// route-mock on the /info endpoint (deterministic), preserving every other real field.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

async function forceVenue(page: import('@playwright/test').Page, status: 'open' | 'closed') {
  await page.route('**/public/locations/demo/info', async (route) => {
    const resp = await route.fetch();
    const json = await resp.json();
    await route.fulfill({ json: { ...json, status, isOpen: status === 'open' } });
  });
}

async function gotoMenu(page: import('@playwright/test').Page) {
  await page.goto(`${BASE}/s/demo`, { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('[data-testid=menu-item]', { timeout: 25000 });
}

test.describe('Storefront audit · Batch 1 · /s/demo', () => {
  test('no Solar/sunlight toggle on the storefront (one theme)', async ({ page }) => {
    await gotoMenu(page);
    await expect(page.getByTestId('sunlight-toggle')).toHaveCount(0);
  });

  test('Ukrainian available + real logo rendered', async ({ page }) => {
    await gotoMenu(page);
    await expect(page.getByRole('button', { name: 'UA' })).toBeVisible();
    // logo is a data: URI WebP set in the header
    const logo = page.locator('header img, [role=banner] img').first();
    await expect(logo).toBeVisible();
    const src = await logo.getAttribute('src');
    expect(src || '').toMatch(/^data:image\/webp/);
  });

  test('categories act as a single-select filter', async ({ page }) => {
    await gotoMenu(page);
    const before = await page.locator('main h2').count();
    expect(before).toBeGreaterThan(1);
    await page.getByTestId('category-nav').getByRole('button', { name: /^Pizzas/ }).click();
    await expect.poll(async () => page.locator('main h2').count()).toBeLessThan(before);
    await expect(page.locator('main h2').first()).toContainText(/Pizzas/);
    // "All" chip restores the full menu
    await page.getByTestId('category-nav').getByRole('button', { name: /^(All|Të gjitha|Усі)/ }).click();
    await expect.poll(async () => page.locator('main h2').count()).toBe(before);
  });

  test('single price-sort toggle (4-button row gone)', async ({ page }) => {
    await gotoMenu(page);
    await expect(page.getByRole('button', { name: /Sort by price|sipas çmimit|за ціною/i })).toHaveCount(1);
    await expect(page.getByRole('button', { name: 'A–Z' })).toHaveCount(0);
  });

  test('CLOSED: prominent banner + ordering blocked (card + modal)', async ({ page }) => {
    await forceVenue(page, 'closed');
    await gotoMenu(page);
    const banner = page.getByTestId('venue-closed-banner');
    await expect(banner).toBeVisible();
    await expect(banner).toContainText(/closed|mbyllur|зачинено/i);
    // Modal CTA is disabled and relabelled when closed.
    await page.locator('[data-testid=menu-item]').first().click();
    const cta = page.getByTestId('product-detail-confirm');
    await expect(cta).toBeVisible();
    await expect(cta).toBeDisabled();
    await expect(cta).toContainText(/closed|mbyllur|зачинено/i);
  });

  test('footer contact rail: WhatsApp + call + maps, no fake socials', async ({ page }) => {
    await gotoMenu(page);
    const footer = page.locator('footer');
    await footer.scrollIntoViewIfNeeded();
    await expect(footer.locator('a[href^="https://wa.me/"]')).toBeVisible();
    await expect(footer.locator('a[href^="tel:"]')).toBeVisible();
    await expect(footer.locator('a[href*="google.com/maps"]')).toBeVisible();
    await expect(footer.locator('a[href*="demo_resto"], a[href*="demo.resto"]')).toHaveCount(0);
  });

  test('OPEN: checkout privacy simplified, no "Did you know?" fact', async ({ page }) => {
    await forceVenue(page, 'open');
    await gotoMenu(page);
    // add a no-modifier item, then open the checkout sheet
    await page.getByTestId('menu-item-add').first().click();
    await page.goto(`${BASE}/s/demo?checkout=1`, { waitUntil: 'domcontentloaded' });
    const privacy = page.getByTestId('checkout-privacy-notice');
    await expect(privacy).toBeVisible({ timeout: 15000 });
    await expect(privacy).toContainText(/never sell|never share|nuk i shesim|не продаємо/i);
    await expect(page.getByText(/Did you know|A e dini|Чи знали ви/i)).toHaveCount(0);
  });
});
