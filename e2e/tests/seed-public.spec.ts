import { test, expect } from '@playwright/test';

// Public storefront seed — no auth needed. Lands the agent on the live menu.
// Read-only (navigation only) → no requireStaging guard; BASE already defaults to staging.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test('seed: public storefront is live', async ({ page }) => {
  // Gate on the real menu API response, not the HTML shell load: a blank shell loads even
  // when the menu fetch 404s/500s. EXACT 200 — anything else is a broken storefront.
  const [menuResponse] = await Promise.all([
    page.waitForResponse((r) => r.url().includes('/public/locations/sushi-durres/menu')),
    page.goto(`${BASE}/s/sushi-durres`),
  ]);
  expect(menuResponse.status()).toBe(200);

  // Real render proof: at least one product card is visible. The error/empty fallback and the
  // product list are mutually-exclusive branches, so a visible menu-item rules out the error state.
  await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible();
});

test('seed: unknown storefront slug is rejected (no cross-tenant leak)', async ({ page }) => {
  // Isolation: a non-existent slug must 404 at the menu API and render zero product cards.
  // TODO(needs-staging): for stronger tenant-isolation, point this at a REAL second tenant's
  // private/unpublished slug (404/403 by authorization, not just absence) once one is seeded.
  const [menuResponse] = await Promise.all([
    page.waitForResponse((r) => r.url().includes('/public/locations/___invalid-slug-xyz/menu')),
    page.goto(`${BASE}/s/___invalid-slug-xyz`),
  ]);
  expect(menuResponse.status()).toBe(404);

  // No product card may leak through for an unknown venue.
  await expect(page.locator('[data-testid="menu-item"]')).toHaveCount(0);
});
