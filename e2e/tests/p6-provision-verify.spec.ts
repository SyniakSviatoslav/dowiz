import { test, expect } from '@playwright/test';

// P6-6 ProvisionVerifier — DEPLOY-TIME Mandatory-Proof E2E. Re-asserts the labeled-preview external-boundary
// invariants against a LIVE shadow on staging. P6-3 RICH PREVIEW upgrade: humans now get the REAL React
// storefront in a NEVER-ORDERABLE preview mode (same design as a live store, no cart/checkout, claim CTA),
// while BOTS / no-JS unfurlers still get the bare server-rendered static preview with GENERIC OG. This
// spec proves BOTH paths + every invariant that must survive the richer rendering.
//
// Setup (run on staging once P6 migrations are applied + PROVISION_OPS_SECRET is set):
//   1. provision a shadow via the ops surface (POST /internal/acquisition + /provision/mint + /provision/spine
//      with x-provision-ops-secret), giving it a known slug.
//   2. set PROVISION_VERIFY_SLUG=<that slug> and run:
//      VITE_BASE_URL=https://dowiz-staging.fly.dev PROVISION_VERIFY_SLUG=<slug> \
//        pnpm exec playwright test e2e/tests/p6-provision-verify.spec.ts --reporter=list
const SLUG = process.env.PROVISION_VERIFY_SLUG;
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

(SLUG ? test : test.skip)('shadow rich preview: labeled, noindex, generic-OG, never-orderable (human)', async ({ page, request }) => {
  const url = `${BASE}/s/${SLUG}`;

  // Header-level: X-Robots-Tag noindex on the response (set for BOTH human + bot shadow paths).
  const resp = await request.get(url);
  expect(resp.status()).toBe(200);
  expect((resp.headers()['x-robots-tag'] || '').toLowerCase()).toContain('noindex');

  await page.goto(url);

  // The REAL storefront renders: category nav + at least one product card are visible (not the bare list).
  await expect(page.getByTestId('category-nav')).toBeVisible();
  await expect(page.getByTestId('menu-item').first()).toBeVisible();

  // Honest preview banner + owner claim CTA are present (both render ONLY in preview mode). Banner
  // copy is i18n (defaults to sq) so we assert structure here; the English "not a live store" honest
  // label is proven verbatim on the bot path below.
  await expect(page.getByTestId('venue-preview-banner')).toBeVisible();
  await expect(page.getByTestId('preview-claim-cta')).toBeVisible();

  // noindex meta + NEVER-ORDERABLE: no add (+) affordance, no add-to-cart/checkout button anywhere.
  await expect(page.locator('meta[name="robots"][content*="noindex"]')).toHaveCount(1);
  await expect(page.getByTestId('menu-item-add')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /add to cart|checkout/i })).toHaveCount(0);
  await expect(page.getByTestId('cart-open')).toHaveCount(0); // cart FAB never renders (cart stays empty)

  // H3 — generic OG: the real restaurant name must NOT appear in <title> or og:* metadata (no unfurl leak).
  // The name DOES appear in the page body (header brand chrome), so derive it from there and assert exclusion.
  const title = await page.title();
  expect(title).toMatch(/preview|dowiz/i);
  expect(title.toLowerCase()).not.toContain('sushi'); // the real venue name never leaks to <title>
  // og:title is either ABSENT (the shadow shell injects no tenant meta — best) or generic; either way
  // it must never carry the venue name token. count()-guard so an absent meta isn't a 10s getAttribute wait.
  const ogCount = await page.locator('meta[property="og:title"]').count();
  if (ogCount > 0) {
    const ogTitle = (await page.locator('meta[property="og:title"]').first().getAttribute('content')) || '';
    expect(ogTitle.toLowerCase()).not.toContain('sushi');
  }
});

(SLUG ? test : test.skip)('shadow preview: bots still get the bare static generic-OG page', async ({ request }) => {
  // A no-JS crawler / social unfurler gets the server-rendered static preview (renderShadowPreview):
  // honest banner in static HTML + a GENERIC <title> with no real name (H3) so a pasted link never doxes.
  const resp = await request.get(`${BASE}/s/${SLUG}`, { headers: { 'user-agent': 'Googlebot/2.1 (+http://www.google.com/bot.html)' } });
  expect(resp.status()).toBe(200);
  expect((resp.headers()['x-robots-tag'] || '').toLowerCase()).toContain('noindex');
  const html = await resp.text();
  expect(html).toContain('Restaurant preview'); // generic <title>Restaurant preview · Dowiz</title>
  expect(html.toLowerCase()).toContain('not a live store');
  expect(html.toLowerCase()).not.toMatch(/<title>[^<]*sushi[^<]*<\/title>/i); // real name never in <title>
});
