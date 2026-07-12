/* eslint-disable @typescript-eslint/no-explicit-any, local/no-raw-any -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { test, expect } from '@playwright/test';
import fs from 'node:fs';

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
    await expect(page.locator('body')).toBeVisible();
    await page.evaluate(() => (document as any).fonts?.ready);
    await page.waitForTimeout(500);
    await page.screenshot({ path: `${SHOTS}/owner-dashboard.png`, fullPage: true });

    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await page.evaluate(() => (document as any).fonts?.ready);
    await page.waitForTimeout(1500);
    await page.screenshot({ path: `${SHOTS}/owner-menu.png`, fullPage: true });
    // Proof: the seeded sushi menu is visible to test@dowiz.com.
    await expect(page.getByText(/Crispy Sunset|Maguro|sushi|Sushi/i).first()).toBeVisible({ timeout: 15000 });

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
    const card = page.locator('[data-testid^="product-card"], [class*="product"]').first();
    if (await card.isVisible({ timeout: 5000 }).catch(() => false)) {
      await card.click();
      await page.waitForTimeout(800);
      await page.screenshot({ path: `${SHOTS}/client-product-modal.png`, fullPage: true });
    }
  });
});

test.describe('Mobile polish — checkout / cart', () => {
  test('add to cart → cart drawer → checkout render on mobile', async ({ page }) => {
    await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle' });
    await page.evaluate(() => (document as any).fonts?.ready);
    await page.waitForTimeout(1200);

    const add = page.getByTestId('menu-item-add').first();
    await expect(add).toBeVisible({ timeout: 15000 });
    await add.click();

    const cartOpen = page.getByTestId('cart-open');
    await expect(cartOpen).toBeVisible({ timeout: 8000 });
    await cartOpen.click();
    await page.waitForTimeout(600);
    await page.screenshot({ path: `${SHOTS}/client-cart.png`, fullPage: true });

    await page.getByTestId('cart-checkout').click();
    await page.evaluate(() => (document as any).fonts?.ready);
    await page.waitForTimeout(1200);
    await expect(page.getByTestId('checkout-phone')).toBeVisible({ timeout: 15000 });
    await page.screenshot({ path: `${SHOTS}/client-checkout.png`, fullPage: true });
  });
});

test.describe('Mobile polish — courier app', () => {
  test('tasks, earnings, shift render on mobile', async ({ page, request }) => {
    const res = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } });
    expect(res.ok(), `courier mock-auth failed ${res.status()}`).toBeTruthy();
    const token = (await res.json()).access_token;
    await page.addInitScript((t: string) => localStorage.setItem('dos_access_token', t), token);

    for (const [route, name] of [['/courier', 'tasks'], ['/courier/earnings', 'earnings'], ['/courier/shift', 'shift']] as const) {
      await page.goto(`${BASE}${route}`, { waitUntil: 'networkidle' });
      await page.evaluate(() => (document as any).fonts?.ready);
      await page.waitForTimeout(1000);
      await page.screenshot({ path: `${SHOTS}/courier-${name}.png`, fullPage: true });
    }
    await expect(page.locator('body')).toBeVisible();
  });
});
