import { test, expect } from '@playwright/test';

/**
 * L1 — public /start front door. Proves the static swan hero renders, carries the
 * single "upload your menu" CTA at a thumb-friendly size (≥48px), and that the CTA
 * is the entry into menu import. No WebGL — the hero is authored SVG + CSS.
 *
 * Runs against the FE under test (VITE_BASE_URL). The hero is client-rendered, so
 * it needs no backend data.
 */

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe('L1: /start swan hero', () => {
  test('renders the hero + upload CTA at a tappable size', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (e) => errors.push(String(e)));

    await page.goto(`${BASE}/start`);

    // Hero headline + authored swan art are visible (not a bare card).
    await expect(page.getByRole('heading', { name: /menu/i })).toBeVisible();
    await expect(page.locator('.dz-hero-art svg')).toBeVisible();

    // Value showcase: three steps ending in "go live".
    await expect(page.locator('.dz-hero-steps li')).toHaveCount(3);

    // Single primary CTA → menu import, sized for thumbs (≥48px tall).
    const cta = page.getByTestId('upload-menu-cta');
    await expect(cta).toBeVisible();
    const box = await cta.boundingBox();
    expect(box, 'CTA has a layout box').not.toBeNull();
    expect(box!.height, 'CTA tap target ≥48px').toBeGreaterThanOrEqual(48);

    // Hidden file input wired to the CTA (the import entry point).
    await expect(page.getByTestId('menu-file-input')).toHaveCount(1);

    expect(errors, `no page errors: ${errors.join('; ')}`).toHaveLength(0);
  });

  test('respects reduced-motion (hero still fully rendered)', async ({ browser }) => {
    const ctx = await browser.newContext({ reducedMotion: 'reduce', viewport: { width: 390, height: 844 } });
    const page = await ctx.newPage();
    await page.goto(`${BASE}/start`);
    await expect(page.getByRole('heading', { name: /menu/i })).toBeVisible();
    // The self-drawing strokes must be fully visible (not stuck hidden) under reduce.
    const drawn = await page.locator('.dz-swan-line.dz-draw').first().evaluate(
      (el) => getComputedStyle(el).strokeDashoffset,
    );
    expect(['none', '0', '0px']).toContain(drawn);
    await ctx.close();
  });
});
