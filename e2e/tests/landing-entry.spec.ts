/**
 * Bebop landing at `/` — deployed-reality proof (Mandatory Proof Rule + VbM).
 *
 * Proves the Warm Cosmo-Noir entry page (commit 330ff4ed) is what the root
 * route actually serves on the target host: the `[data-skin="bebop"]` shell
 * mounts, the Session-01 headline renders with real text, and the primary CTA
 * routes to /claim. Console-guard is auto-attached — any runtime error fails.
 *
 * FALSIFIABLE (the RED case): the same spec against a host still running the
 * pre-330ff4ed bundle (e.g. prod, deployed from main) has no `[data-skin=
 * "bebop"]` element at `/` and goes RED on the first assertion.
 *
 * GREEN: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test e2e/tests/landing-entry.spec.ts --reporter=list
 * RED:   VITE_BASE_URL=https://dowiz.fly.dev         pnpm exec playwright test e2e/tests/landing-entry.spec.ts --reporter=list
 */
import { test, expect } from '../fixtures/console-guard';

test('/ serves the Bebop landing (data-skin="bebop" shell + headline)', async ({ page }) => {
  const response = await page.goto('/', { waitUntil: 'networkidle' });
  expect(response?.status(), 'GET / must be 200').toBe(200);

  // The old bundle rendered no bebop shell at / — this is the RED/GREEN pivot.
  await expect(page.locator('.lp-root[data-skin="bebop"]')).toBeVisible({ timeout: 15000 });

  // Session 01 headline carries real (i18n) copy, not an empty mount.
  const title = page.locator('h1.lp-title');
  await expect(title).toBeVisible();
  expect((await title.innerText()).trim().length).toBeGreaterThan(10);

  // Corner HUD logo — the Nomadic skeleton chrome mounted.
  await expect(page.locator('a.lp-logo')).toHaveText('dowiz');
});

test('primary CTA routes to /claim', async ({ page }) => {
  await page.goto('/', { waitUntil: 'networkidle' });
  const cta = page.locator('.lp-cta--primary').first();
  await expect(cta).toBeVisible({ timeout: 15000 });
  await cta.click();
  await expect(page).toHaveURL(/\/claim/);
});

test('landing has no horizontal overflow', async ({ page }) => {
  await page.goto('/', { waitUntil: 'networkidle' });
  await expect(page.locator('.lp-root[data-skin="bebop"]')).toBeVisible({ timeout: 15000 });
  const { sw, cw } = await page.evaluate(() => ({
    sw: document.documentElement.scrollWidth,
    cw: document.documentElement.clientWidth,
  }));
  expect(sw, `horizontal overflow (scrollWidth ${sw} > clientWidth ${cw})`).toBeLessThanOrEqual(cw + 1);
});
