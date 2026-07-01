import { test, expect } from '@playwright/test';

// PROOF for per-tenant storefront fonts (commit 5cd298aa + migration 084). Product titles must render the
// tenant's font — NOT the former hardcoded serif. ArtePasta (cuisine=italian) seeds heading=Fraunces; the
// unbranded /s/demo has no font ids so it resolves the default pairing (Playfair) — i.e. visually unchanged.
//
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test \
//   e2e/tests/storefront-fonts.spec.ts --project=mobile --project=desktop --reporter=list

async function firstTitleFont(page: import('@playwright/test').Page, slug: string): Promise<string> {
  await page.goto(`/s/${slug}`);
  const title = page.getByTestId('menu-item').first().locator('h3').first();
  await title.waitFor({ state: 'visible', timeout: 20000 });
  return title.evaluate((el) => getComputedStyle(el).fontFamily);
}

test.describe('Storefront · per-tenant fonts', () => {
  test('ArtePasta product titles render the tenant heading font (Fraunces), not the old hardcoded serif', async ({ page }) => {
    const font = await firstTitleFont(page, 'artepasta');
    // computed font-family is the resolved stack; the tenant face must be first/present.
    expect(font).toContain('Fraunces');
  });

  test('the unbranded /s/demo keeps the default heading font (Playfair) — no regression', async ({ page }) => {
    const font = await firstTitleFont(page, 'demo');
    expect(font).toContain('Playfair');
  });
});
