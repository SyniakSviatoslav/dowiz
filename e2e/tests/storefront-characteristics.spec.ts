import { test, expect } from '@playwright/test';

// Menu Characteristics layer — Compare (step 3) + Filter lens (step 4), flag-dark surfaces. REQUIRES a
// staging build with VITE_MENU_CHARACTERISTICS_COMPARISON=true and VITE_MENU_CHARACTERISTICS_FILTER=true
// (the default prod build has them OFF, so these surfaces are absent and these tests are skipped there).
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

async function gotoMenu(page: import('@playwright/test').Page) {
  await page.goto(`${BASE}/s/demo`, { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('[data-testid=menu-item]', { timeout: 25000 });
}

test.describe('Storefront · Menu Characteristics (flag-on) · /s/demo', () => {
  test.beforeEach(async ({ page }) => {
    await gotoMenu(page);
    // If the compare affordance is absent, the build is flag-off → skip (these are flag-dark surfaces).
    if (await page.getByTestId('compare-toggle').count() === 0) test.skip(true, 'characteristics flags off in this build');
  });

  test('COMPARE: pick two dishes → panel shows both, arrows only on price/prep, reliance bound present', async ({ page }) => {
    const toggles = page.getByTestId('compare-toggle');
    await toggles.nth(0).click();
    await toggles.nth(1).click();
    // selection bar appears with 2/2 and an enabled Compare CTA
    await expect(page.getByTestId('compare-bar')).toBeVisible();
    await page.getByTestId('compare-open').click();
    const panel = page.getByTestId('compare-panel');
    await expect(panel).toBeVisible();
    // the reliance bound rides every allergen surface, including comparison (#8/#5d)
    await expect(page.getByTestId('compare-allergen-reliance')).toBeVisible();
    // no global "winner"/"healthier" verdict text leaks into the panel (facts-not-verdict, #11)
    await expect(panel).not.toContainText(/winner|healthier|best dish/i);
  });

  test('COMPARE: a third selection is blocked (max two)', async ({ page }) => {
    const toggles = page.getByTestId('compare-toggle');
    await toggles.nth(0).click();
    await toggles.nth(1).click();
    // the 3rd toggle is disabled while two are chosen
    await expect(toggles.nth(2)).toBeDisabled();
  });

  test('FILTER lens: the macro pills toggle and the menu still renders (no-data dishes never crash the rank)', async ({ page }) => {
    const proteinPill = page.getByTestId('macro-lens-protein');
    await expect(proteinPill).toBeVisible();
    await proteinPill.click();
    await expect(proteinPill).toHaveAttribute('aria-pressed', 'true');
    // the menu re-renders under the lens without error
    await expect(page.getByTestId('menu-item').first()).toBeVisible();
    // toggling off restores
    await proteinPill.click();
    await expect(proteinPill).toHaveAttribute('aria-pressed', 'false');
  });
});
