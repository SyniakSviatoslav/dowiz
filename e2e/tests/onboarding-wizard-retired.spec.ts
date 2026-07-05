import { test, expect } from '@playwright/test';
import { expectJwt } from '../helpers/assert-shape';

// Proof for O3: the 9-step onboarding wizard is retired. A fresh owner now lands
// on a minimal create-storefront form (name · phone · link) that POSTs to the real
// /owner/onboarding/start, then hands off to the activation tool. The old wizard's
// false "You're live!", demo-menu seeding, share-before-publish, and ephemeral
// "Step N of 9" progress are gone.
//
// Run against staging (separate DB — safe to create a throwaway draft):
//   VITE_BASE_URL=https://dowiz-staging.fly.dev DEV_AUTH_SECRET=stg-e2e-secret \
//     pnpm exec playwright test e2e/tests/onboarding-wizard-retired.spec.ts \
//     --project=desktop --reporter=list

const SECRET = process.env.DEV_AUTH_SECRET || 'stg-e2e-secret';

async function loginAsOwner(page: import('@playwright/test').Page, request: import('@playwright/test').APIRequestContext) {
  const r = await request.post('/api/dev/mock-auth', {
    headers: { 'x-dev-auth-secret': SECRET },
    data: { role: 'owner' },
  });
  expect(r.ok(), `mock-auth failed: ${r.status()}`).toBeTruthy();
  const { access_token } = await r.json();
  expectJwt(access_token, 'access_token');
  await page.goto('/');
  // Force English so the locale-specific assertions below (incl. the negative
  // "old wizard text is gone" checks) are meaningful — staging defaults to sq.
  await page.evaluate((t) => {
    localStorage.setItem('dos_access_token', t);
    localStorage.setItem('dos_locale', 'en');
  }, access_token);
}

test.describe('Onboarding wizard retired (O3)', () => {
  test('/admin/onboarding renders the minimal create-storefront form, not the 9-step wizard', async ({ page, request }) => {
    await loginAsOwner(page, request);
    await page.goto('/admin/onboarding');

    // New minimal form present.
    await expect(page.getByRole('heading', { name: /create your storefront/i })).toBeVisible();
    await expect(page.getByPlaceholder('e.g. Pizza Roma')).toBeVisible();
    await expect(page.getByPlaceholder('+355 69 XXX XXXX')).toBeVisible();
    await expect(page.getByPlaceholder('sushi-durres')).toBeVisible();
    await expect(page.getByRole('button', { name: /create & continue/i })).toBeVisible();

    // Old 9-step wizard markers gone.
    await expect(page.getByText(/Step\s*1\s*of\s*9/i)).toHaveCount(0);
    await expect(page.getByText(/Demo Menu/i)).toHaveCount(0);
    await expect(page.getByText(/Run Flow Test/i)).toHaveCount(0);
    await expect(page.getByText(/You'?re live/i)).toHaveCount(0);
  });

  test('submitting the form creates a draft and hands off to the activation tool', async ({ page, request }) => {
    await loginAsOwner(page, request);
    await page.goto('/admin/onboarding');

    const slug = `e2e-retire-${Date.now().toString(36)}`;
    await page.getByPlaceholder('e.g. Pizza Roma').fill('E2E Retire Test');
    await page.getByPlaceholder('+355 69 XXX XXXX').fill('+355690000001');
    await page.getByPlaceholder('sushi-durres').fill(slug);
    await page.getByRole('button', { name: /create & continue/i }).click();

    // Real POST /owner/onboarding/start succeeded → routed to the activation tool.
    await page.waitForURL('**/admin/activation', { timeout: 15000 });
    // Both draft ("Get your storefront live") and live ("Your storefront is live")
    // headings contain "storefront" — assert the activation tool actually rendered.
    await expect(page.getByRole('heading', { name: /storefront/i })).toBeVisible();
  });
});
