/* eslint-disable @typescript-eslint/no-explicit-any -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { test, expect } from '@playwright/test';

/**
 * Self-contained UI proof for the notification category preference-centre.
 *
 * Serves the REAL built web (apps/web/dist, built with VITE_TG_CATEGORY_GATING=true) via
 * `vite preview` and mocks only the bootstrap API — the backend write path is proven
 * separately by the DB-integration tests. This runs fully offline (no deploy, no secret),
 * unlike notif-categories.spec.ts which needs deployed staging + DEV_AUTH_SECRET.
 *
 * Run:
 *   VITE_TG_CATEGORY_GATING=true pnpm --filter web build
 *   pnpm --filter web exec vite preview --port 4173 --strictPort &
 *   VITE_BASE_URL=http://localhost:4173 pnpm exec playwright test \
 *     e2e/tests/notif-categories-local.spec.ts --project=desktop --reporter=list
 */
const LOC = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
const TGT = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';

// Runs only when explicitly targeted against a `vite preview` of the built dist — skipped
// in the default suite (which boots the dev API, not the flag-built preview).
test.skip(!process.env.LOCAL_UI_PROOF, 'set LOCAL_UI_PROOF=1 and serve apps/web/dist via vite preview');

test('preference-centre renders three categories and persists an operational toggle', async ({ page }) => {
  let putBody: any = null;
  const prefs: Record<string, boolean> = { operational: true, quality: false };

  await page.route('**/api/**', async (route) => {
    const url = route.request().url();
    const method = route.request().method();
    if (url.includes('/owner/settings')) {
      return route.fulfill({ json: { id: LOC, locationName: 'Test Loc', slug: 'test-loc', phone: '' } });
    }
    if (url.includes('/notifications/targets') && method === 'PUT') {
      putBody = route.request().postDataJSON();
      Object.assign(prefs, putBody?.prefs || {}); // reflect change so the UI refetch shows it
      return route.fulfill({ json: { success: true } });
    }
    if (url.includes('/notifications/targets')) {
      return route.fulfill({ json: { targets: [{ id: TGT, channel: 'telegram', status: 'active', address: 'chat123', prefs: { ...prefs } }] } });
    }
    if (url.includes('/settings/fallback')) {
      return route.fulfill({ json: { phone: null, showPhoneOnError: false, showPhoneOnOffline: false } });
    }
    return route.fulfill({ json: {} });
  });

  await page.addInitScript(() => localStorage.setItem('dos_access_token', 'mock-token'));
  await page.goto('/admin/settings', { waitUntil: 'networkidle' });

  const card = page.getByTestId('notif-categories');
  await expect(card).toBeVisible({ timeout: 15000 });

  // 🔴 transactional is always-on (no toggle)
  await expect(page.getByTestId('notif-cat-transactional')).toContainText(/always on/i);

  // 🟠 operational defaults ON, 🟡 quality defaults OFF
  const opToggle = page.getByTestId('notif-cat-operational').getByRole('switch');
  const qToggle = page.getByTestId('notif-cat-quality').getByRole('switch');
  await expect(opToggle).toHaveAttribute('aria-checked', 'true');
  await expect(qToggle).toHaveAttribute('aria-checked', 'false');

  // toggle operational OFF → the UI sends the correct atomic PUT body...
  await opToggle.click();
  await expect.poll(() => putBody?.prefs?.operational, { timeout: 5000 }).toBe(false);
  // ...and reflects the new state after refetch
  await expect(opToggle).toHaveAttribute('aria-checked', 'false');
});
