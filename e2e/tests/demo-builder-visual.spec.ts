import { test, expect } from '@playwright/test';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

// demo-builder LAYER-3 visual acceptance gate (live). The orchestrator (scripts/demo-builder.mjs) spawns
// this spec against the deployed /s/:slug for a freshly-provisioned shadow, on the mobile + desktop projects,
// and gates on exit 0 + the artifact this writes. It asserts the SAME conditions as the pure assertPreviewDom
// (rendered menu cards, honest preview banner + claim CTA, ZERO console errors, NEVER-ORDERABLE, noindex) but
// against a REAL browser render — so a storefront the API marks "verified" that renders empty / errored /
// orderable FAILS here and the loop classifies it needs-review (no-fake-green).
//
// Run (standalone):
//   VITE_BASE_URL=https://dowiz-staging.fly.dev PROVISION_VERIFY_SLUG=<slug> \
//     pnpm exec playwright test e2e/tests/demo-builder-visual.spec.ts --project=mobile --project=desktop --reporter=list

const SLUG = process.env.PROVISION_VERIFY_SLUG;
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const MIN_RENDERED_ITEMS = Number(process.env.DEMO_MIN_RENDERED_ITEMS ?? 3);

function recordArtifact(project: string, payload: Record<string, unknown>) {
  if (!SLUG) return;
  const dir = resolve('e2e/artifacts');
  mkdirSync(dir, { recursive: true });
  const file = resolve(dir, `demo-builder-visual-${SLUG}.json`);
  let acc: Record<string, unknown> = {};
  try { acc = JSON.parse(readFileSync(file, 'utf8')); } catch { /* first project run */ }
  acc[project] = payload;
  writeFileSync(file, JSON.stringify(acc, null, 2));
}

(SLUG ? test : test.skip)('demo-builder visual gate: /s/:slug renders a demo-quality, never-orderable preview', async ({ page }, testInfo) => {
  const project = testInfo.project.name; // 'mobile' | 'desktop'
  const url = `${BASE}/s/${SLUG}`;

  // Capture real page console errors — a broken demo must NOT slip through as "renders fine".
  const consoleErrors: string[] = [];
  page.on('console', (m) => { if (m.type() === 'error') consoleErrors.push(m.text()); });
  page.on('pageerror', (e) => consoleErrors.push(String(e)));

  const resp = await page.goto(url);
  const robots = (resp?.headers()['x-robots-tag'] || '').toLowerCase();

  // rendered menu items (the demo-quality floor) + honest preview banner + claim CTA.
  await expect(page.getByTestId('category-nav')).toBeVisible();
  // The SPA paints the shell + category nav FIRST, then hydrates the item cards a beat later. Counting
  // immediately after the nav yields 0 on a perfectly good preview (false negative). Wait for the first
  // card to render before counting. A GENUINELY empty preview still fails: this wait times out and the
  // count stays 0, which the ≥MIN_RENDERED_ITEMS assertion below rejects (the gate is not weakened).
  await page.getByTestId('menu-item').first().waitFor({ state: 'visible', timeout: 15000 }).catch(() => {});
  const itemCount = await page.getByTestId('menu-item').count();
  await expect(page.getByTestId('venue-preview-banner')).toBeVisible();
  // Claim CTA is hidden by design (SHOW_CLAIM_CTA=false) — the banner now carries the demo pitch instead.

  // NEVER-ORDERABLE (B3) — verified in the render, not assumed.
  const addCount = await page.getByTestId('menu-item-add').count();
  const cartCount = await page.getByTestId('cart-open').count();
  const orderBtns = await page.getByRole('button', { name: /add to cart|checkout/i }).count();
  const noindexMeta = await page.locator('meta[name="robots"][content*="noindex"]').count();

  recordArtifact(project, { url, itemCount, consoleErrors: consoleErrors.length, addCount, cartCount, orderBtns, noindex: robots.includes('noindex') || noindexMeta > 0 });

  // Assertions (all must hold — the loop reads exit code, not just the artifact).
  expect(itemCount, `≥${MIN_RENDERED_ITEMS} menu items must render`).toBeGreaterThanOrEqual(MIN_RENDERED_ITEMS);
  expect(consoleErrors, `no console errors: ${consoleErrors.join(' | ')}`).toHaveLength(0);
  expect(addCount, 'no add affordance (never-orderable)').toBe(0);
  expect(cartCount, 'no cart FAB (never-orderable)').toBe(0);
  expect(orderBtns, 'no add-to-cart/checkout button (never-orderable)').toBe(0);
  expect(robots.includes('noindex') || noindexMeta > 0, 'preview must be noindex').toBeTruthy();
});
