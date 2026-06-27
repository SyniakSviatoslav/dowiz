import { test, expect } from '@playwright/test';

// Harness smoke: confirms Playwright can drive the DEPLOYED storefront (VITE_BASE_URL).
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test \
//        e2e/tests/storefront-smoke.spec.ts --project=mobile --reporter=list

const DEMO_SLUG = 'sushi-durres';
// TODO(needs_staging): set E2E_SECOND_SLUG to a REAL second tenant on the target so the
// cross-tenant isolation test below runs with two distinct published menus.
const SECOND_SLUG = process.env.E2E_SECOND_SLUG || 'demo';

test('public storefront menu page renders (deployed)', async ({ page }) => {
  const resp = await page.goto(`/s/${DEMO_SLUG}`);
  // Exact 200 — a 204/301-loop/302-to-error all used to pass under `< 400`.
  expect(resp?.status(), `GET /s/${DEMO_SLUG}`).toBe(200);
  // Catch a silent redirect away from the storefront route.
  expect(page.url(), 'final URL stayed on the storefront route').toContain(`/s/${DEMO_SLUG}`);
  // Shell mounted is not enough — assert real menu content (a product card) is visible.
  // The SPA fetches the menu after mount, so allow for the network round-trip.
  await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible({ timeout: 20_000 });
  await expect(page.locator('[data-testid="menu-item-add"]').first()).toBeVisible();
});

test('unknown tenant slug shows a "not found" state (no blank/error shell)', async ({ page }) => {
  // The document GET still serves the SPA shell (200); the API returns 404 for the slug and the
  // app renders an explicit "venue not found" state (MenuPage.tsx setNotFound → client.venue_not_found).
  const resp = await page.goto('/s/this-slug-does-not-exist-zzz999');
  expect(resp?.status(), 'shell document GET').toBe(200);
  await expect(page.getByText('Restaurant not found')).toBeVisible({ timeout: 20_000 });
  // No menu content should leak into a not-found page.
  await expect(page.locator('[data-testid="menu-item"]')).toHaveCount(0);
});

test('cross-tenant isolation: a second tenant renders its OWN menu, not the demo tenant', async ({ page }) => {
  // Load the demo tenant and capture its venue name (h1 = menu.location_name, MenuPage.tsx:590).
  await page.goto(`/s/${DEMO_SLUG}`);
  await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible({ timeout: 20_000 });
  const demoName = (await page.locator('h1').first().innerText()).trim();
  expect(demoName, 'demo venue name must be a real name, not the "Menu" fallback').not.toBe('Menu');
  expect(demoName.length, 'demo venue name non-empty').toBeGreaterThan(0);

  // Load a DISTINCT tenant; its menu/name must not be the demo tenant's data.
  const resp = await page.goto(`/s/${SECOND_SLUG}`);
  expect(resp?.status(), `GET /s/${SECOND_SLUG}`).toBe(200);
  expect(page.url(), 'final URL stayed on the second tenant route').toContain(`/s/${SECOND_SLUG}`);
  await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible({ timeout: 20_000 });
  const secondName = (await page.locator('h1').first().innerText()).trim();
  expect(secondName, 'second venue name non-empty').not.toBe('');
  expect(secondName, 'second tenant must not render the demo tenant name').not.toBe(demoName);
});
