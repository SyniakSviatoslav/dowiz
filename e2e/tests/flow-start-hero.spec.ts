import { test, expect } from '@playwright/test';

/**
 * L1 — public /start front door (simple form). The paper/Nomadic animated hero
 * was rolled back, so this proves the plain layout: a heading, the single
 * "upload your menu" CTA at a thumb-friendly size (≥48px), and the wired import
 * file input — no WebGL scene, no decorative swan.
 *
 * Runs against the FE under test (VITE_BASE_URL). The 'choose' phase is fully
 * client-rendered, so it needs no backend data.
 */

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe('L1: /start onboarding (simple form)', () => {
  test('renders the heading + upload CTA at a tappable size', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (e) => errors.push(String(e)));

    await page.goto(`${BASE}/start`);

    // Heading is visible (not a bare/blank card).
    await expect(page.getByRole('heading', { name: /menu/i })).toBeVisible();

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

  test('renders fully under reduced-motion', async ({ browser }) => {
    const ctx = await browser.newContext({ reducedMotion: 'reduce', viewport: { width: 390, height: 844 } });
    const page = await ctx.newPage();
    await page.goto(`${BASE}/start`);
    await expect(page.getByRole('heading', { name: /menu/i })).toBeVisible();
    await expect(page.getByTestId('upload-menu-cta')).toBeVisible();
    await ctx.close();
  });
});
