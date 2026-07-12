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
  let getCount = 0;
  let getsAtPut = -1;
  const prefs: Record<string, boolean> = { operational: true, quality: false };

  await page.route('**/api/**', async (route) => {
    const url = route.request().url();
    const method = route.request().method();
    if (url.includes('/owner/settings')) {
      return route.fulfill({ json: { id: LOC, locationName: 'Test Loc', slug: 'test-loc', phone: '' } });
    }
    if (url.includes('/notifications/targets') && method === 'PUT') {
      putBody = route.request().postDataJSON();
      getsAtPut = getCount; // snapshot GET count at write time to prove a later refetch
      Object.assign(prefs, putBody?.prefs || {}); // reflect change so the UI refetch shows it
      return route.fulfill({ json: { success: true } });
    }
    if (url.includes('/notifications/targets')) {
      getCount += 1;
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

  // 🔴 transactional is always-on (no toggle). Assert the label AND the structural
  // proof that no switch exists — a sibling "always on" text-bleed cannot satisfy a
  // toHaveCount(0) on the category's own switch (Finding 1/2: substring false-green).
  const txCat = page.getByTestId('notif-cat-transactional');
  await expect(txCat).toContainText(/always on/i);
  await expect(txCat.getByRole('switch')).toHaveCount(0);

  // 🟠 operational defaults ON, 🟡 quality defaults OFF
  const opToggle = page.getByTestId('notif-cat-operational').getByRole('switch');
  const qToggle = page.getByTestId('notif-cat-quality').getByRole('switch');
  await expect(opToggle).toHaveAttribute('aria-checked', 'true');
  await expect(qToggle).toHaveAttribute('aria-checked', 'false');

  // toggle operational OFF → the UI sends the correct atomic PUT body...
  await opToggle.click();
  await expect.poll(() => putBody?.prefs?.operational, { timeout: 5000 }).toBe(false);
  // ...the UI actually REFETCHES from the server after the write (a GET fires post-PUT),
  // so the rendered state is server truth, not an optimistic local flip (Finding 3)...
  await expect.poll(() => getCount, { timeout: 5000 }).toBeGreaterThan(getsAtPut);
  // ...and reflects the new state after that refetch
  await expect(opToggle).toHaveAttribute('aria-checked', 'false');
});
