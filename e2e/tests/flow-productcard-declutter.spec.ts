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

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const SLUG = process.env.TRACK_SLUG || 'sushi-durres';

test.describe('Storefront ProductCard', () => {
  test('cards render and the add-to-cart affordance is interactive', async ({ page }) => {
    await page.goto(`${BASE}/s/${SLUG}`);

    const firstCard = page.getByTestId('menu-item').first();
    await expect(firstCard).toBeVisible({ timeout: 20000 });

    // The card keeps its title and a working add button (≥44px tap target).
    // Title must be visible AND carry a non-empty product name (an empty/whitespace
    // <h3> would otherwise pass a bare toBeVisible() check).
    const title = firstCard.locator('h3');
    await expect(title).toBeVisible();
    await expect(title).not.toBeEmpty();
    const add = firstCard.getByTestId('menu-item-add');
    await expect(add).toBeVisible();
    const box = await add.boundingBox();
    expect(box!.height, 'add button tap target ≥44px').toBeGreaterThanOrEqual(44);

    // Allergen scent is not duplicated: a card shows at most one allergen-label
    // cluster (the image-corner badges), never the old second body row.
    // Allergen badges live in exactly one image-corner cluster (ProductCard's
    // `absolute top-1.5 left-1.5` div). Counting the CLUSTER container — not the
    // individual badge spans — makes the dedup proof count-independent: a regressed
    // duplicate body row adds a 2nd cluster even for items with only 1-2 allergens,
    // which a span-count cap of ≤4 would silently miss.
    const allergenClusters = await firstCard.locator('div.absolute.left-1\\.5').count();
    expect(allergenClusters, 'at most one allergen badge cluster — no duplicated body row').toBeLessThanOrEqual(1);
  });
});
