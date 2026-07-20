import { test, expect } from '@playwright/test';

// Proof for the FE-polish batch (commit 1feff7c0). FE-only, server read-only.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test fe-polish-batch --project=desktop --reporter=list

test.describe('FE polish batch', () => {
  test('404 is a branded soft state with a real return-home CTA (was bare text)', async ({ page }) => {
    await page.goto('/this-route-does-not-exist-xyz-123');
    // Heading is locale-independent.
    await expect(page.getByRole('heading', { name: '404' })).toBeVisible();
    // The new branded CTA is a styled button-link (brand-primary bg), not the old
    // plain text link. Assert the link to "/" carries the brand-primary background.
    const cta = page.locator('a[href="/"]');
    await expect(cta).toBeVisible();
    const bg = await cta.evaluate((el) => getComputedStyle(el).backgroundColor);
    // Old CTA had no background (transparent / rgba(0,0,0,0)); branded one resolves
    // --brand-primary to an opaque colour.
    expect(bg, 'return-home CTA has an opaque branded background').not.toBe('rgba(0, 0, 0, 0)');
    expect(bg).not.toBe('transparent');
  });

  test('storefront hero title is visible and uses --brand-text (WCAG fix, not on-primary white)', async ({ page }) => {
    await page.goto('/s/demo');
    const h1 = page.getByRole('heading', { level: 1 }).first();
    await expect(h1).toBeVisible();
    // The fix switched the hero title off --color-on-primary onto --brand-text so it
    // contrasts with the brand-bg-tinted bottom scrim. Assert the inline style references it.
    const style = await h1.getAttribute('style');
    expect(style, 'hero h1 inline style').toContain('var(--brand-text)');
    expect(style, 'hero h1 no longer uses on-primary white').not.toContain('--color-on-primary');
  });
});
