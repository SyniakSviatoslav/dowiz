import { test, expect } from '@playwright/test';

// Proof for the UI-loop run fixes (2026-06-24). Runs against the deployed app.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test ui-loop-fixes --project=desktop --reporter=list
// NOTE: requires a deploy of the fix commit — both assertions fail on the pre-fix build (the prior
// blank /admin/login, and the retry-only not-found), which is the expected red→green.

test('/admin/login no longer renders blank — redirects to the real login form', async ({ page }) => {
  await page.goto('/admin/login');
  // The fix routes /admin/login → /login (and a catch-all sends unknown /admin/* → /admin → /login).
  // Anchor the FULL path to '/login' so /courier/login, /owner/login, /x/login can't pass.
  await expect(page).toHaveURL(/^https?:\/\/[^/]+\/login\/?(?:[?#].*)?$/, { timeout: 15000 });
  // The REAL login form renders (was 0 inputs/0 buttons before the fix). Use semantic selectors
  // so a stray hidden input or nav button can't satisfy this — the email field + Sign In submit.
  await expect(page.getByRole('textbox', { name: /email/i }), 'login form has an email field').toBeVisible();
  await expect(page.getByRole('button', { name: /sign in/i }), 'login form has a submit control').toBeVisible();
});

test('unknown venue slug shows a "not found" state with a way home, not a futile retry', async ({ page }) => {
  await page.goto('/s/__definitely_not_a_real_slug__');
  // Escape hatch: a link back home (href="/"), not a retry button that re-fails on a bad slug.
  const home = page.locator('a[href="/"]');
  await expect(home, 'not-found offers a home escape').toBeVisible({ timeout: 15000 });
  // The transient "Retry" CTA must NOT be the offered action for a 404 slug.
  await expect(page.getByRole('button', { name: /retry/i }), 'no futile retry on a 404 slug').toHaveCount(0);
});
