import { test, expect } from '@playwright/test';

// Public storefront seed — no auth needed. Lands the agent on the live menu.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test('seed: public storefront is live', async ({ page }) => {
  await page.goto(`${BASE}/s/sushi-durres`, { waitUntil: 'load' });
  await expect(page.locator('body')).toBeVisible();
});
