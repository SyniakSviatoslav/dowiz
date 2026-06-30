import { test, expect } from '@playwright/test';

// Allergen FREEZE (operator directive 2026-06-30) + dish nutrition viz, against deployed staging /s/demo.
// Allergens are hidden everywhere: no allergen surface, no "not a complete list" reliance bound, no
// allergen filter chips. The detail modal instead shows the DishStats nutrition/ingredients visualization.
// (The computeAllergenSurface single-source library + guardrails stay intact underneath — display frozen.)
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

async function gotoMenu(page: import('@playwright/test').Page) {
  await page.goto(`${BASE}/s/demo`, { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('[data-testid=menu-item]', { timeout: 25000 });
}

// Open a dish that has nutrition data (a sushi item with a BOM) so DishStats renders.
async function openNutritionDish(page: import('@playwright/test').Page) {
  const card = page.locator('[data-testid=menu-item]', { hasText: 'Crunchy Ebi Sunset' }).first();
  await card.scrollIntoViewIfNeeded();
  await card.click();
  await page.waitForTimeout(800);
}

test.describe('Storefront · allergens frozen + nutrition viz · /s/demo', () => {
  test.describe.configure({ retries: 2 });

  test('NO allergen surface or reliance bound anywhere in the detail modal', async ({ page }) => {
    await gotoMenu(page);
    await openNutritionDish(page);
    await expect(page.getByTestId('allergen-surface')).toHaveCount(0);
    await expect(page.getByTestId('allergen-reliance')).toHaveCount(0);
    await expect(page.getByTestId('allergen-no-info')).toHaveCount(0);
    // and the reliance-bound copy is gone from the whole page
    await expect(page.getByText(/not a complete allergen list/i)).toHaveCount(0);
  });

  test('the detail modal shows the DishStats nutrition visualization', async ({ page }) => {
    await gotoMenu(page);
    await openNutritionDish(page);
    await expect(page.getByTestId('dish-stats')).toBeVisible();
  });

  test('no allergen filter chips on the storefront (frozen)', async ({ page }) => {
    await gotoMenu(page);
    await expect(page.getByTestId('allergen-filter-chips')).toHaveCount(0);
  });
});
