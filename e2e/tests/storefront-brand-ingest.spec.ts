import { test, expect } from '@playwright/test';

// PROOF for the brand-ingest display-attributes feature (commit b3f75ed5): a scraped/authored menu's
// display-only attribute keys (image_url / ingredients / description_sq) must render on the SHADOW
// preview BEFORE claim — surviving read_preview_menu's `attributes - 'bom'` allergen strip. Runs live
// against the deployed /s/artepasta shadow (real Wolt-extracted ArtePasta menu: 50 items, 38 photos,
// 31 ingredient lists, 28 Albanian descriptions).
//
// Run:
//   VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test \
//     e2e/tests/storefront-brand-ingest.spec.ts --project=mobile --project=desktop --reporter=list

const SLUG = 'artepasta';

test.describe('Storefront · brand-ingest display attributes render pre-claim · /s/artepasta', () => {
  test('scraped photos + ingredient badges + bilingual description render, still non-orderable + noindex', async ({ page }, testInfo) => {
    const consoleErrors: string[] = [];
    page.on('console', (m) => { if (m.type() === 'error') consoleErrors.push(m.text()); });
    page.on('pageerror', (e) => consoleErrors.push(String(e)));

    const resp = await page.goto(`/s/${SLUG}`);
    const robots = (resp?.headers()['x-robots-tag'] || '').toLowerCase();

    // The real Wolt menu hydrated (not the stale 16-item hand-authored one).
    await page.getByTestId('menu-item').first().waitFor({ state: 'visible', timeout: 20000 });
    const itemCount = await page.getByTestId('menu-item').count();
    expect(itemCount).toBeGreaterThanOrEqual(3);

    // FEATURE 1 — scraped card photos ride attributes.image_url (no uploaded image_key exists pre-claim).
    // They render as <img src="…imageproxy.wolt.com…"> via ProductCard. Assert several actually painted.
    const woltImgs = page.locator('img[src*="imageproxy.wolt.com"]');
    await expect(woltImgs.first()).toBeVisible();
    expect(await woltImgs.count()).toBeGreaterThanOrEqual(3);

    // Open a dish known to carry all three attribute kinds (Pasta Piramida).
    await page.getByTestId('menu-item').filter({ hasText: 'Pasta Piramida' }).first().click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();

    // FEATURE 2 — ingredient badges from attributes.ingredients (survive the bom-strip). The heading is
    // locale-driven (ArtePasta default_locale=sq → "PËRBËRËSIT"), but the badge TEXT is the raw data, so
    // assert on the badges themselves — proof the block rendered from attributes.ingredients.
    await expect(dialog.getByText('Porcini mushrooms', { exact: true })).toBeVisible();
    await expect(dialog.getByText('Stracciatella', { exact: true })).toBeVisible();

    // FEATURE 3 — original-language (Albanian) description under the English one, lang="sq".
    const sq = dialog.locator('p[lang="sq"]');
    await expect(sq.first()).toBeVisible();
    await expect(sq.first()).toContainText('Porcini');

    // The modal photo is the scraped Wolt image too.
    await expect(dialog.locator('img[src*="imageproxy.wolt.com"]').first()).toBeVisible();

    // SAFETY — the shadow preview stays NEVER-ORDERABLE (B3) and noindex, exactly as before this feature.
    expect(await page.getByTestId('menu-item-add').count()).toBe(0);
    const noindexMeta = await page.locator('meta[name="robots"][content*="noindex"]').count();
    expect(robots.includes('noindex') || noindexMeta > 0).toBe(true);

    expect(consoleErrors, `console errors: ${consoleErrors.join(' | ')}`).toHaveLength(0);

    await page.screenshot({ path: `audit/brand-ingest/${SLUG}-${testInfo.project.name}-modal.png`, fullPage: false });
  });
});
