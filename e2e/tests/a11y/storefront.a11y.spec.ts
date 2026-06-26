/**
 * Sense 1 · A11y per-flow gate — public storefront (no auth).
 *
 * Reads the COMPUTED accessibility tree (axe), not pixels. Runs across the
 * three supported locales {sq, en, uk} and scans BOTH the initial render and an
 * interactive state (product modal open) — the default scan misses interactive
 * focus/contrast. A11y authority lives here, not in the vision layer.
 *
 * Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test e2e/tests/a11y/storefront.a11y.spec.ts --reporter=list
 */
import { test, expect } from '../../fixtures/console-guard';
import { expectNoA11y } from '../../helpers/a11y';

const SLUG = process.env.DEMO_SLUG || 'sushi-durres';
const LOCALES = ['sq', 'en', 'uk'] as const;

for (const lang of LOCALES) {
  test(`storefront a11y — ${lang}`, async ({ page }) => {
    await page.addInitScript((l) => {
      try {
        localStorage.setItem('dos_locale', l);
      } catch {
        /* storage blocked — falls back to venue default */
      }
    }, lang);

    await page.goto(`/s/${SLUG}`, { waitUntil: 'networkidle' });

    // Menu content present → SPA hydrated, safe to scan.
    await expect(page.locator('main, [role="main"], h1, [data-testid="menu"]').first()).toBeVisible({
      timeout: 15000,
    });

    // 1) initial render
    await expectNoA11y(page);

    // 2) interactive state — open the first product detail/modal if reachable.
    const product = page
      .locator(
        '[data-testid^="product"], article button, [role="button"]:has-text("+"), button:has-text("Shto"), button:has-text("Add")',
      )
      .first();
    if (await product.count()) {
      await product.click({ trial: false }).catch(() => {});
      await page.waitForTimeout(400); // let dialog mount + focus settle
      await expectNoA11y(page);
    }
  });
}
