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

const SLUG = process.env.DEMO_SLUG || 'sushi-durres';

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
  await expect(page.locator('main, [role="main"], h1').first()).toBeVisible({ timeout: 15000 });
  await assertNoHorizontalOverflow(page, 'storefront');

  // Tap-target survey on the primary CTA(s) — visible, enabled controls only.
  const tapIssues = await checkTouchTargets(page);
  const sized = tapIssues.filter((i) => i.startsWith('size:'));
  test.info().annotations.push({ type: 'tap-targets', description: sized.join(' | ') || 'none' });
  expect(sized, `sub-44 tap targets on WebKit:\n${sized.join('\n')}`).toEqual([]);
});

test('checkout — no overflow / clean runtime on WebKit', async ({ page }) => {
  await page.goto(`/s/${SLUG}/checkout`, { waitUntil: 'networkidle' });
  await expect(page.locator('main, [role="main"], h1, form').first()).toBeVisible({ timeout: 15000 });
  await assertNoHorizontalOverflow(page, 'checkout');
});
