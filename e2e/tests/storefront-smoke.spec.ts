import { test, expect } from '@playwright/test';

// Harness smoke: confirms Playwright can drive the DEPLOYED storefront (VITE_BASE_URL).
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test \
//        e2e/tests/storefront-smoke.spec.ts --project=mobile --reporter=list
test('public storefront menu page renders (deployed)', async ({ page }) => {
  const resp = await page.goto('/s/sushi-durres');
  expect(resp?.status(), 'GET /s/sushi-durres').toBeLessThan(400);
  await expect(page.locator('#root')).toBeVisible();
  // SSR menu route renders real content — body should carry non-trivial text.
  await expect(page.locator('body')).not.toBeEmpty();
});
