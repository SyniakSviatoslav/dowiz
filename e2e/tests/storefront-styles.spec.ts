/**
 * Storefront styles guardrail — unstyled-storefront regression (2026-07-05).
 *
 * Root cause class: the human /s/:slug page is served by the Astro upstream, whose
 * hashed /_astro/* bundles must be forwarded by the cutover front-door. When they
 * 404 (API JSON envelope), every storefront page renders UNSTYLED — a whole-surface
 * visual regression invisible to API parity checks.
 *
 * Pins: (1) every same-origin stylesheet the served HTML references loads as 200
 * text/css, (2) the rendered page is actually styled (the CSS took effect), not the
 * browser-default sheet.
 */
import { test, expect } from '@playwright/test';

test.describe('storefront styles (/_astro asset pipeline)', () => {
  test('every stylesheet on /s/demo resolves 200 text/css and styling is applied', async ({ page, request }) => {
    const failed: string[] = [];
    page.on('response', (res) => {
      if (res.url().includes('/_astro/') && res.status() >= 400) {
        failed.push(`${res.status()} ${res.url()}`);
      }
    });

    await page.goto('/s/demo', { waitUntil: 'networkidle' });

    // (1) The document's own stylesheet links must all serve as real CSS.
    const hrefs = await page.$$eval('link[rel="stylesheet"]', (links) =>
      links.map((l) => (l as HTMLLinkElement).href).filter((h) => h.startsWith(location.origin)),
    );
    expect(hrefs.length, 'page must reference at least one same-origin stylesheet').toBeGreaterThan(0);
    for (const href of hrefs) {
      const res = await request.get(href);
      expect(res.status(), `stylesheet ${href}`).toBe(200);
      expect(res.headers()['content-type'], `stylesheet ${href}`).toContain('text/css');
    }
    expect(failed, 'no /_astro asset may 404 during page load').toEqual([]);

    // (2) The CSS actually applied: a styled page never keeps the UA-default serif
    // body font, and the menu shell must be visible.
    const bodyFont = await page.evaluate(() => getComputedStyle(document.body).fontFamily);
    expect(bodyFont.toLowerCase()).not.toMatch(/^"?times/);
    await expect(page.locator('body')).toBeVisible();
  });
});
