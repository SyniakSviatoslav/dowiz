import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import { expectJwt } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// Mobile-first polish + proof: test@dowiz.com must see the Dubin & Sushi (demo) data,
// and key screens must render clean at a phone viewport.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const SHOTS = 'e2e/artifacts/mobile-polish';
test.use({ viewport: { width: 390, height: 844 } });

fs.mkdirSync(SHOTS, { recursive: true });

async function ownerToken(request: any): Promise<string> {
  const res = await request.post(`${BASE}/api/auth/local/login`, {
    data: { email: 'test@dowiz.com', password: 'test123456' },
  });
  expect(res.ok(), `local-login failed ${res.status()}`).toBeTruthy();
  return (await res.json()).access_token;
}

test.describe('Mobile polish — owner (test@dowiz.com → Dubin & Sushi)', () => {
  test('admin dashboard, menu, settings render the sushi data on mobile', async ({ page, request }) => {
    const token = await ownerToken(request);
    await page.addInitScript((t: string) => localStorage.setItem('dos_access_token', t), token);

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await page.evaluate(() => (document as any).fonts?.ready);
    await page.waitForTimeout(500);
    // Proof: the owner dashboard actually rendered (not a login redirect or error
    // page). ws-status-dot lives in the live-orders header that only the dashboard mounts.
    await expect(page.getByTestId('ws-status-dot')).toBeVisible({ timeout: 15000 });
    await page.screenshot({ path: `${SHOTS}/owner-dashboard.png`, fullPage: true });

    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await page.evaluate(() => (document as any).fonts?.ready);
    await page.waitForTimeout(1500);
    await page.screenshot({ path: `${SHOTS}/owner-menu.png`, fullPage: true });
    // Proof: a seeded menu *item* is visible to test@dowiz.com — scoped to the product
    // card (the broad /sushi/i regex matched the venue name "Dubin & Sushi" in the header,
    // a false-green). Product cards are the only `rounded-xl border cursor-pointer` rows.
    await expect(
      page.locator('div.rounded-xl.border.cursor-pointer').filter({ hasText: /Crispy Sunset|Maguro/i }).first(),
    ).toBeVisible({ timeout: 15000 });

    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle' });
    await page.evaluate(() => (document as any).fonts?.ready);
    await page.waitForTimeout(1000);
    await page.screenshot({ path: `${SHOTS}/owner-settings.png`, fullPage: true });
    // Data access is already proven by the menu assertion above; here just assert the
    // owner settings page rendered (target the page heading by role to avoid hidden matches).
    await expect(page.getByRole('heading', { name: /Cilësimet|Settings/i }).first()).toBeVisible({ timeout: 15000 });
  });
});

test.describe('Mobile polish — client storefront (/s/demo)', () => {
  test('storefront, product modal, cart render clean on mobile', async ({ page }) => {
    await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle' });
    await page.evaluate(() => (document as any).fonts?.ready);
    await page.waitForTimeout(1500);
    await expect(page.getByText(/Dubin & Sushi/i).first()).toBeVisible({ timeout: 15000 });
    await page.screenshot({ path: `${SHOTS}/client-storefront.png`, fullPage: true });

    // Open the first product (cinematic modal) — capture for polish review.
<<<<<<< Updated upstream
    const card = page.locator('[data-testid^="product-card"], [class*="product"]').first();
    if (await card.isVisible({ timeout: 5000 }).catch(() => false)) {
      await card.click();
      await page.waitForTimeout(800);
      await page.screenshot({ path: `${SHOTS}/client-product-modal.png`, fullPage: true });
    }
=======
    // No silent guard: a missing card is a real failure (500/empty-menu must go red).
    const card = page.locator('[data-testid="menu-item"]').first();
    await expect(card).toBeVisible({ timeout: 15000 });
    await card.click();
    await page.waitForTimeout(800);
    await page.screenshot({ path: `${SHOTS}/client-product-modal.png`, fullPage: true });
>>>>>>> Stashed changes
  });
});

test.describe('Mobile polish — checkout / cart', () => {
  test('add to cart → cart drawer → checkout render on mobile', async ({ page }) => {
    await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle' });
    await page.evaluate(() => (document as any).fonts?.ready);
    await page.waitForTimeout(1200);

    const firstItem = page.getByTestId('menu-item').first();
    await expect(firstItem).toBeVisible({ timeout: 15000 });
    const itemName = (await firstItem.locator('h3').first().innerText()).trim();
    expect(itemName, 'menu item must have a name').not.toBe('');

    await firstItem.getByTestId('menu-item-add').click();

    const cartOpen = page.getByTestId('cart-open');
    await expect(cartOpen).toBeVisible({ timeout: 8000 });
    // Proof: exactly one unit landed in the cart (the FAB count badge reads '1').
    await expect(cartOpen.getByText('1', { exact: true }).first()).toBeVisible({ timeout: 8000 });
    await cartOpen.click();
    await page.waitForTimeout(600);
    // Proof: the cart drawer shows the product we added — scoped to the dialog, not the
    // menu grid (where the same name is always visible).
    await expect(page.getByRole('dialog').getByText(itemName, { exact: true }).first()).toBeVisible({ timeout: 8000 });
    await page.screenshot({ path: `${SHOTS}/client-cart.png`, fullPage: true });

    await page.getByTestId('cart-checkout').click();
    await page.evaluate(() => (document as any).fonts?.ready);
    await page.waitForTimeout(1200);
    await expect(page.getByTestId('checkout-phone')).toBeVisible({ timeout: 15000 });
    // Proof: the cart total carried into checkout (a non-empty money value).
    await expect(page.getByTestId('checkout-total')).toBeVisible({ timeout: 8000 });
    await expect(page.getByTestId('checkout-total')).toContainText(/\d/);
    await page.screenshot({ path: `${SHOTS}/client-checkout.png`, fullPage: true });
  });
});

test.describe('Mobile polish — courier app', () => {
  // mock-auth is the dev backdoor — never let this run against prod.
  test.beforeAll(() => requireStaging(BASE));

  test('tasks, earnings, shift render on mobile', async ({ page, request }) => {
    const res = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } });
    expect(res.ok(), `courier mock-auth failed ${res.status()}`).toBeTruthy();
    const token = (await res.json()).access_token;
    expectJwt(token, 'courier access_token');

    // Negative control: the courier API rejects an unauthenticated caller (401, not a
    // silent empty list) — so a passing isolation/earnings read means the gate works.
    const noAuth = await request.get(`${BASE}/api/courier/me/earnings`);
    expect(noAuth.status(), 'earnings must 401 without a token').toBe(401);

    // Positive control: the minted token reads its OWN earnings — scoped by courier_id +
    // activeLocationId server-side (apps/api/src/routes/courier/me.ts:177).
    const earnings = await request.get(`${BASE}/api/courier/me/earnings`, {
      headers: { authorization: `Bearer ${token}` },
    });
    expect(earnings.status(), 'courier earnings with token').toBe(200);
    const earningsBody = await earnings.json();
    expect(earningsBody.summary, 'earnings must return a scoped summary').toHaveProperty('today');
    expect(typeof earningsBody.summary.today, 'today earnings is a number').toBe('number');
    // TODO(needs-staging): full cross-tenant isolation needs a REAL second tenant's courier
    // fixture — assert this token's earnings/assignments NEVER surface another location's
    // order ids. mock-auth mints a synthetic single-location courier, so this run proves the
    // auth gate + own-tenant scope, not the cross-tenant block.

    await page.addInitScript((t: string) => localStorage.setItem('dos_access_token', t), token);

    const heading = {
      tasks: /Tasks|Detyrat|Завдання/i,
      earnings: /Earnings|Fitimet|Заробіток/i,
      shift: /Shift|Turni|Зміна/i,
    } as const;
    for (const [route, name] of [['/courier', 'tasks'], ['/courier/earnings', 'earnings'], ['/courier/shift', 'shift']] as const) {
      await page.goto(`${BASE}${route}`, { waitUntil: 'networkidle' });
      await page.evaluate(() => (document as any).fonts?.ready);
      await page.waitForTimeout(1000);
      // Proof: the courier-specific page rendered (its own h1), not a 401/login/error screen.
      await expect(page.getByRole('heading', { name: heading[name] }).first()).toBeVisible({ timeout: 15000 });
      await page.screenshot({ path: `${SHOTS}/courier-${name}.png`, fullPage: true });
    }
  });
});
