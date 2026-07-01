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
  // Staging cold-start / public-menu pool warm-up makes the first menu load flaky (known: pool starvation).
  // The assertions are deterministic; retry the environmental warm-up rather than inflate the per-test budget.
  test.describe.configure({ retries: 2 });
  test.beforeEach(async ({ page }) => {
    await gotoMenu(page);
    // If the compare affordance is absent, the build is flag-off → skip (these are flag-dark surfaces).
    if (await page.getByTestId('compare-toggle').count() === 0) test.skip(true, 'characteristics flags off in this build');
  });

  test('COMPARE: pick two dishes → bar shows names, panel shows both + nutrition viz, no verdict/allergens', async ({ page }) => {
    // Pick two dishes that HAVE nutrition (a BOM) so DishStats renders in both columns. The compare toggle
    // is a sibling button of the card article within its wrapper → target it precisely via the shared parent.
    const toggleFor = (name: string) =>
      page.locator('[data-testid=menu-item]', { hasText: name }).first().locator('xpath=../button[@data-testid="compare-toggle"]');
    const a = toggleFor('Crunchy Ebi Sunset'); await a.scrollIntoViewIfNeeded(); await a.click();
    const b = toggleFor('Crispy Sunset'); await b.scrollIntoViewIfNeeded(); await b.click();
    // selection bar appears; it shows the selected dish NAME(s), not just a count
    const bar = page.getByTestId('compare-bar');
    await expect(bar).toBeVisible();
    expect(((await bar.innerText()) || '').replace(/\s+/g, '')).not.toMatch(/^×?2\/2.*Compare$/i); // not just "2/2 … Compare"
    await page.getByTestId('compare-open').click();
    const panel = page.getByTestId('compare-panel');
    await expect(panel).toBeVisible();
    // both dishes carry the DishStats nutrition viz (2 columns)
    await expect(panel.getByTestId('dish-stats')).toHaveCount(2);
    // allergens are frozen — no reliance bound / allergen copy in the panel
    await expect(panel.getByText(/not a complete allergen list/i)).toHaveCount(0);
    // facts-not-verdict (#11): no global winner/healthier verdict
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
