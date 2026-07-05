import { test, expect } from '@playwright/test';

/**
 * Storefront ProductCard anti-slop declutter: the redundant body allergen row was
 * removed (allergen scent stays on the image-corner badges + full list in the
 * detail modal). This proves the card still renders and stays interactive after
 * the change — i.e. no structural regression. Theme-independent assertions only
 * (colours are tenant-SSR-driven and not asserted here).
 *
 * Runs against the FE under test (VITE_BASE_URL).
 */

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
const SLUG = process.env.TRACK_SLUG || 'sushi-durres';

test.describe('Storefront ProductCard', () => {
  test('cards render and the add-to-cart affordance is interactive', async ({ page }) => {
    await page.goto(`${BASE}/s/${SLUG}`);

    const firstCard = page.getByTestId('menu-item').first();
    await expect(firstCard).toBeVisible({ timeout: 20000 });

    // The card keeps its title and a working add button (≥44px tap target).
    await expect(firstCard.locator('h3')).toBeVisible();
    const add = firstCard.getByTestId('menu-item-add');
    await expect(add).toBeVisible();
    const box = await add.boundingBox();
    expect(box!.height, 'add button tap target ≥44px').toBeGreaterThanOrEqual(44);

    // Allergen scent is not duplicated: a card shows at most one allergen-label
    // cluster (the image-corner badges), never the old second body row.
    const allergenLabels = await firstCard.locator('span', { hasText: /^[A-ZËÇ]{3,}$/ }).count();
    // corner shows up to 3 + an overflow "+N"; the removed body row would have
    // pushed this well past 4. Cap proves the dedup holds.
    expect(allergenLabels, 'no duplicated allergen row in card body').toBeLessThanOrEqual(4);
  });
});
