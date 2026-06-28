import { test, expect } from '@playwright/test';

// P6-6 ProvisionVerifier — DEPLOY-TIME Mandatory-Proof E2E. Re-asserts the labeled-preview external-boundary
// invariants against a LIVE shadow on staging (the server-side verifyShadowPreview gates VERIFIED in-process;
// this proves the same in a real browser against the deployed URL).
//
// Setup (run on staging once P6 migrations are applied + PROVISION_OPS_SECRET is set):
//   1. provision a shadow via the ops surface (POST /internal/acquisition + /provision/mint + /provision/spine
//      with x-provision-ops-secret), giving it a known slug.
//   2. set PROVISION_VERIFY_SLUG=<that slug> and run:
//      VITE_BASE_URL=https://dowiz-staging.fly.dev PROVISION_VERIFY_SLUG=<slug> \
//        pnpm exec playwright test e2e/tests/p6-provision-verify.spec.ts --reporter=list
const SLUG = process.env.PROVISION_VERIFY_SLUG;
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

(SLUG ? test : test.skip)('shadow preview: labeled, noindex, generic-OG, never-orderable', async ({ page, request }) => {
  const url = `${BASE}/s/${SLUG}`;

  // Header-level: X-Robots-Tag noindex on the response.
  const resp = await request.get(url);
  expect(resp.status()).toBe(200);
  expect((resp.headers()['x-robots-tag'] || '').toLowerCase()).toContain('noindex');

  await page.goto(url);

  // Honest banner renders; menu items visible.
  await expect(page.getByText(/not a live store/i)).toBeVisible();
  await expect(page.locator('.item-name').first()).toBeVisible();

  // noindex meta + never-orderable (no cart/checkout affordance anywhere).
  await expect(page.locator('meta[name="robots"][content*="noindex"]')).toHaveCount(1);
  await expect(page.getByRole('button', { name: /add to cart|checkout/i })).toHaveCount(0);

  // H3 — generic OG: the real restaurant name must NOT appear in <title> or og:* metadata (no unfurl leak).
  const title = await page.title();
  expect(title).toMatch(/preview|dowiz/i);
  const h1 = (await page.locator('h1').first().textContent())?.trim() || '';
  expect(h1.length).toBeGreaterThan(0); // the real name DOES appear in the body…
  const ogTitle = await page.locator('meta[property="og:title"]').getAttribute('content');
  expect(ogTitle || '').not.toContain(h1); // …but NOT in the unfurl metadata
});
