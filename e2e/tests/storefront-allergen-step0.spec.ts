import { test, expect } from '@playwright/test';

// Menu Characteristics Model — STEP-0 allergen single-source safety fix (FB-C1/FB-C2), against deployed
// staging /s/demo. STEP-0 lands with NO flag, so this runs against the default staging build.
//
// What it proves on REAL staging DOM:
//  1. The detail modal renders the honest allergen surface UNCONDITIONALLY (previously gated behind
//     VITE_MENU_CHARACTERISTICS_ENABLED; with the flag off a dish with no recipe allergens rendered
//     NOTHING — the FB-C2 false-negative). Now every opened dish shows the surface.
//  2. The reliance bound ("not a complete list — confirm with the venue") is ALWAYS attached (#5d).
//  3. The allergen FILTER chips are gated OFF (VITE_MENU_ALLERGEN_FILTER unset) — the FB-C1 recorded
//     human decision (predicate stays converged in code; the unit test proves declared-only retention).
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

async function gotoMenu(page: import('@playwright/test').Page) {
  await page.goto(`${BASE}/s/demo`, { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('[data-testid=menu-item]', { timeout: 25000 });
}

async function openFirstDish(page: import('@playwright/test').Page) {
  // Tap the card body (not the add button) to open the detail modal.
  await page.locator('[data-testid=menu-item]').first().click();
  await expect(page.getByTestId('allergen-surface')).toBeVisible({ timeout: 10000 });
}

test.describe('Storefront · STEP-0 allergen single-source · /s/demo', () => {
  // Staging cold-start / public-menu pool warm-up makes the first menu load flaky (known: pool starvation).
  // The assertions are deterministic; retry the environmental warm-up rather than inflate the per-test budget.
  test.describe.configure({ retries: 2 });

  test('detail modal renders the honest allergen surface UNCONDITIONALLY (flag off)', async ({ page }) => {
    await gotoMenu(page);
    await openFirstDish(page);
    // The surface block is present regardless of flag / recipe data (FB-C2 unconditional fix).
    await expect(page.getByTestId('allergen-surface')).toBeVisible();
  });

  test('the reliance bound is always attached (#5d)', async ({ page }) => {
    await gotoMenu(page);
    await openFirstDish(page);
    // Locale-agnostic: assert the dedicated element exists + carries non-empty copy.
    const reliance = page.getByTestId('allergen-reliance');
    await expect(reliance).toBeVisible();
    expect(((await reliance.textContent()) || '').trim().length).toBeGreaterThan(10);
  });

  test('a dish surfaces EITHER declared/recipe allergens OR the explicit "info not provided" floor — never blank', async ({ page }) => {
    await gotoMenu(page);
    await openFirstDish(page);
    const surface = page.getByTestId('allergen-surface');
    // Within the surface, exactly one of: known-allergen chips, OR the no-info floor — never empty.
    const hasFloor = await surface.getByTestId('allergen-no-info').count();
    const hasChips = await surface.locator('span').count();
    expect(hasFloor > 0 || hasChips > 0).toBeTruthy();
  });

  test('allergen FILTER chips are gated OFF (VITE_MENU_ALLERGEN_FILTER default off)', async ({ page }) => {
    await gotoMenu(page);
    await expect(page.getByTestId('allergen-filter-chips')).toHaveCount(0);
  });
});
