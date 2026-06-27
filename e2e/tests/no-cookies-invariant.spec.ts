import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

// Test Integrity (2026-06-27): relocated + hardened from apps/api/e2e/phase2.spec.ts,
// which was 5 `expect(true).toBeTruthy()` "simulated" stubs that (a) asserted NOTHING
// and (b) lived in apps/api/e2e/ — a dir OUTSIDE the Playwright testDir (./e2e/tests),
// so the whole file NEVER RAN. The four placeholder flows duplicated real specs and were
// dropped (owner→flow-onboarding-* · customer→client/checkout,flow-order-creation ·
// price-drift→flow-order-creation hard_block · subdomain/embed→subdomain-rewrite.test.ts,
// embed-mode.spec.ts). What remains is the one invariant with no real home: the public
// storefront must set ZERO cookies (PII/compliance — no tracking/session cookies on /s/:slug).

test('No-Cookies invariant — the public storefront sets zero cookies', async ({ page, context }) => {
  await page.goto(`${BASE}/s/demo`);
  await expect(page.locator('[data-testid=menu-item]').first()).toBeVisible({ timeout: 20_000 });
  const cookies = await context.cookies();
  expect(cookies, `storefront must set no cookies — got: ${cookies.map((c) => c.name).join(', ') || '(none)'}`).toHaveLength(0);
});
