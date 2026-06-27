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

// /s/demo = the live demo (Dubin & Sushi). 'sushi-durres' is a dead shell.
const SLUG = process.env.DEMO_SLUG || 'demo';
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

    // 2) interactive state — open the first product detail modal and scan it.
    // The demo storefront always renders products, so this is asserted (no vacuous
    // `if(count)` skip) and the click is NOT swallowed: a failed click → red test.
    const product = page.locator('[data-testid="menu-item"]').first();
    await expect(product).toBeVisible({ timeout: 15000 });
    await product.click();

    // Prove the modal actually opened — otherwise expectNoA11y would re-scan the
    // initial render and falsely claim the interactive state passed.
    const modal = page.locator('[role="dialog"][aria-modal="true"]').first();
    await expect(modal).toBeVisible({ timeout: 5000 });

    await expectNoA11y(page);
  });
}
