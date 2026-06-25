import { test, expect } from '@playwright/test';

/**
 * L1 — public /start front door (Paper/Nomadic redesign). Proves the live hero
 * panel renders with the signature delivery-swan art, carries the single "upload
 * your menu" CTA at a thumb-friendly size (≥48px), wires the import file input,
 * and shows the three-step "how it works" value showcase.
 *
 * The hero art is a live three.js PaperScene with an authored SVG fallback
 * (NomadicScene) for reduced-motion / no-WebGL / SSR — so we assert on the
 * DeliverySwan SVG overlay (`.dz-dswan-svg`), which is pure SVG + CSS and present
 * regardless of WebGL availability, rather than on the scene canvas.
 *
 * Runs against the FE under test (VITE_BASE_URL). The 'choose' phase is fully
 * client-rendered, so it needs no backend data.
 */

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe('L1: /start swan hero', () => {
  test('renders the hero + upload CTA at a tappable size', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (e) => errors.push(String(e)));

    await page.goto(`${BASE}/start`);

    // Hero headline ("Your menu, online tonight.") + the live hero stage panel.
    await expect(page.getByRole('heading', { name: /menu/i })).toBeVisible();
    await expect(page.locator('.dz-stage-1')).toBeVisible();

    // Signature delivery-swan art (authored SVG overlay, WebGL-independent).
    await expect(page.locator('.dz-stage-1 .dz-dswan-svg')).toBeVisible();

    // Value showcase: three "how it works" feature glyphs.
    await expect(page.locator('.dz-glyph')).toHaveCount(3);

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

  test('respects reduced-motion (swan fully drawn, not stuck hidden)', async ({ browser }) => {
    const ctx = await browser.newContext({ reducedMotion: 'reduce', viewport: { width: 390, height: 844 } });
    const page = await ctx.newPage();
    await page.goto(`${BASE}/start`);
    await expect(page.getByRole('heading', { name: /menu/i })).toBeVisible();
    // The swan's self-drawing strokes must be fully revealed (offset 0) under
    // reduce — never stuck at the hidden pathLength=1 dashoffset.
    const drawn = await page.locator('.dz-dswan-draw').first().evaluate(
      (el) => getComputedStyle(el).strokeDashoffset,
    );
    expect(['none', '0', '0px']).toContain(drawn);
    await ctx.close();
  });
});
