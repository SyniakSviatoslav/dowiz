/**
 * Sense 3 · Other-engine smoke (WebKit) — functional only, NO visual baselines.
 *
 * Catches Safari-engine layout breaks that the Chromium baseline is blind to:
 * horizontal overflow, sub-44 tap targets, and runtime JS errors (via the
 * console-guard fixture). The webkit-smoke / webkit-mobile-smoke projects in
 * playwright.config.ts match *.smoke.spec.ts. This file must never call
 * toHaveScreenshot — Chromium-in-Docker stays the single visual source of truth.
 *
 * Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test e2e/tests/storefront.smoke.spec.ts --project=webkit-mobile-smoke --reporter=list
 */
import { test, expect } from '../fixtures/console-guard';
import { checkTouchTargets } from '../helpers/a11y';

// /s/demo = the live demo (Dubin & Sushi). 'sushi-durres' is a dead shell.
const SLUG = process.env.DEMO_SLUG || 'demo';

async function assertNoHorizontalOverflow(page: import('@playwright/test').Page, where: string) {
  const { sw, cw } = await page.evaluate(() => ({
    sw: document.documentElement.scrollWidth,
    cw: document.documentElement.clientWidth,
  }));
  expect(sw, `horizontal overflow @ ${where} (scrollWidth ${sw} > clientWidth ${cw})`).toBeLessThanOrEqual(
    cw + 1,
  );
}

test('storefront — no overflow / clean runtime on WebKit', async ({ page }) => {
  await page.goto(`/s/${SLUG}`, { waitUntil: 'networkidle' });
  // Product-specific proof: a generic `main/h1` is satisfied by 404/500/error-boundary
  // pages, so a broken menu render would stay green. Assert a real menu item is visible.
  await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible({ timeout: 15000 });
  await assertNoHorizontalOverflow(page, 'storefront');

  // Tap-target survey on the primary CTA(s) — visible, enabled controls only.
  const tapIssues = await checkTouchTargets(page);
  const sized = tapIssues.filter((i) => i.startsWith('size:'));
  const proximity = tapIssues.filter((i) => i.startsWith('proximity:'));
  test.info().annotations.push({ type: 'tap-targets', description: sized.join(' | ') || 'none' });
  test.info().annotations.push({ type: 'tap-proximity', description: proximity.join(' | ') || 'none' });
  expect(sized, `sub-44 tap targets on WebKit:\n${sized.join('\n')}`).toEqual([]);
  // proximity:* findings were collected but previously discarded — assert them too.
  expect(proximity, `<8px-gap adjacent tap targets on WebKit:\n${proximity.join('\n')}`).toEqual([]);
});

test('checkout — no overflow / clean runtime on WebKit', async ({ page }) => {
  // A direct hit on /checkout with an empty cart renders the "your cart is empty"
  // guard — a loose `main/h1/form` locator passes on that, hiding a broken checkout
  // form. Seed real cart state first (add a menu item), then assert the checkout
  // FORM itself (phone input) is visible.
  // TODO(needs-staging): requires the demo storefront to have ≥1 orderable product
  // live — validate against VITE_BASE_URL=https://dowiz-staging.fly.dev.
  await page.goto(`/s/${SLUG}`, { waitUntil: 'networkidle' });
  const addBtn = page.locator('[data-testid="menu-item-add"]').first();
  await expect(addBtn).toBeVisible({ timeout: 15000 });
  await addBtn.click();
  await expect(page.locator('#cartFabBtn')).toBeVisible({ timeout: 10000 });

  await page.goto(`/s/${SLUG}/checkout`, { waitUntil: 'networkidle' });
  await expect(page.locator('[data-testid="checkout-phone"]')).toBeVisible({ timeout: 15000 });
  await assertNoHorizontalOverflow(page, 'checkout');
});
