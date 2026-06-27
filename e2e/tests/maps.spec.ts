import { test, expect } from '@playwright/test';

// Non-prod base for the one absolute-URL test below (Test Integrity #6 — never point a
// test at the prod host). Override with VITE_BASE_URL; default to staging.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

// All maps (MapWithPin, CourierLiveMap, …) render through MapLibreBase, whose root
// carries data-testid="map-container". Asserting that node is visible is real render
// proof — body-text / body.length matched nav/footer/script noise, not a mounted map.

test.describe('Map Components', () => {

  test('checkout page shows map with pin component', async ({ page }) => {
    // Add item to cart first, then go to checkout
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });

    // Add item and go to checkout
    await page.getByRole('button', { name: /add|shto/i }).first().click();
    await expect(page.getByTestId('cart-fab')).toBeVisible({ timeout: 5000 });
    await page.getByTestId('cart-fab').click();
    await expect(page.getByText(/your cart|shporta|кошик/i)).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /checkout|porosit|vazhdo/i }).click();
    await expect(page).toHaveURL(/\/checkout/, { timeout: 5000 });

    // Real render proof: the MapWithPin container is mounted and visible.
    await expect(page.getByTestId('map-container')).toBeVisible({ timeout: 10000 });
  });

  test('checkout map loads without CSP worker errors', async ({ page }) => {
    const cspErrors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error' && msg.text().includes('Content Security Policy')) {
        cspErrors.push(msg.text());
      }
    });

    await page.goto(`${BASE}/s/demo/checkout`, { waitUntil: 'networkidle', timeout: 30000 });
    // Wait on the map mounting (worker/blob load happens during this), not a fixed sleep.
    await expect(page.getByTestId('map-container')).toBeVisible({ timeout: 15000 });

    // Verify no CSP worker-src/blob errors
    const workerCspErrors = cspErrors.filter(e => e.includes('worker-src') || e.includes('blob'));
    expect(workerCspErrors).toHaveLength(0);
  });

  test('delivery page renders map container', async ({ page }) => {
    // TODO(needs-staging): on a production build the dev mock task only fabricates under
    // import.meta.env.DEV, so a fake id won't render a map — this needs a seeded real
    // courier assignment id to prove the live map renders against staging.
    await page.goto('/courier/delivery/test-id?dev=true');
    await expect(page.getByTestId('map-container')).toBeVisible({ timeout: 15000 });
  });

  test('delivery page fabricates no map for a fake id without auth', async ({ page }) => {
    // Negative control (Test Integrity #4/#5): an unauthenticated request for a fake
    // assignment id must show the "task not found" soft state and NEVER fabricate a map.
    await page.goto('/courier/delivery/test-id');
    await expect(page.getByText(/task not found|delivery task not found|nuk u gjet/i)).toBeVisible({ timeout: 15000 });
    await expect(page.getByTestId('map-container')).not.toBeVisible();
  });

  test('order status page has map area', async ({ page }) => {
    // TODO(needs-staging): the courier live map renders for DELIVERY orders only — a
    // seeded real delivery order id is required for this to render against staging.
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await expect(page.getByTestId('map-container')).toBeVisible({ timeout: 15000 });
  });

  test('admin dashboard shows courier map', async ({ page }) => {
    // TODO(needs-staging): /admin requires an authed owner + tenant; against staging the
    // dev bypass loads the demo tenant whose dashboard renders the live courier map.
    await page.goto('/admin?dev=true');
    // The "Couriers Live" section + its map are rendered unconditionally on the dashboard.
    await expect(page.getByText(/couriers live|postierët live|кур'єри/i)).toBeVisible({ timeout: 15000 });
    await expect(page.getByTestId('map-container')).toBeVisible({ timeout: 15000 });
  });

  test('onboarding page renders', async ({ page }) => {
    // TODO(needs-staging): /admin/onboarding (MenuFirstOnboarding) renders NO radius map —
    // that feature is absent on this route. Asserting the upload CTA is real render proof;
    // add a radius-map testid assertion here if/when the feature ships.
    await page.goto('/admin/onboarding?dev=true');
    await expect(page.getByTestId('upload-menu-cta')).toBeVisible({ timeout: 15000 });
  });

});
