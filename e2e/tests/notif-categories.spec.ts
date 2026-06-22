import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

/**
 * E2E proof for the notification category preference-centre (Unit #1 / Part D-web-UI).
 *
 * PREREQUISITES to run green (all deploy-time, outside the dev sandbox):
 *   1. Target env built WITH the flag: VITE_TG_CATEGORY_GATING=true — the preference-centre
 *      is intentionally dark otherwise (mirrors the server TG_CATEGORY_GATING flag).
 *   2. DEV_AUTH_SECRET present so /dev/* endpoints respond (playwright.config sends the
 *      x-dev-auth-secret header from process.env.DEV_AUTH_SECRET).
 *   3. The category write path (owner PUT → setCategoryPref) deployed.
 *
 * Run: VITE_BASE_URL=https://dowiz-staging.fly.dev DEV_AUTH_SECRET=… \
 *        pnpm exec playwright test e2e/tests/notif-categories.spec.ts --project=desktop --reporter=list
 */
test.describe('UI: Notification category preference-centre', () => {
  let token: string;
  let locationId: string;
  let userId: string;
  let targetId: string;

  test.beforeAll(async ({ request }) => {
    const auth = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(auth.status(), 'mock-auth (needs DEV_AUTH_SECRET)').toBe(200);
    const a = await auth.json();
    token = a.access_token;
    locationId = a.activeLocationId;
    userId = a.userId;
    expect(locationId, 'mock-auth must return an active location').toBeTruthy();

    const seed = await request.post(`${BASE}/api/dev/seed-telegram-target`, { data: { locationId, userId } });
    expect(seed.status(), 'seed-telegram-target').toBe(200);
    targetId = (await seed.json()).targetId;
  });

  test('renders the three categories and persists an operational toggle', async ({ page, request }) => {
    await page.addInitScript((tk: string) => localStorage.setItem('dos_access_token', tk), token);
    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle' });

    const card = page.getByTestId('notif-categories');
    await expect(card).toBeVisible({ timeout: 15000 });

    // 🔴 transactional row is always-on (no toggle)
    await expect(page.getByTestId('notif-cat-transactional')).toContainText(/always on/i);

    // 🟠 operational defaults ON
    const opToggle = page.getByTestId('notif-cat-operational').getByRole('switch');
    await expect(opToggle).toBeVisible();
    await expect(opToggle).toHaveAttribute('aria-checked', 'true');

    // 🟡 quality defaults OFF
    const qToggle = page.getByTestId('notif-cat-quality').getByRole('switch');
    await expect(qToggle).toHaveAttribute('aria-checked', 'false');

    // toggle operational OFF → verify the write reached the API (prefs.operational === false)
    await opToggle.click();
    await expect.poll(async () => {
      const res = await request.get(`${BASE}/api/owner/locations/${locationId}/notifications/targets`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      const targets = (await res.json()).targets as any[];
      return targets.find((x) => x.id === targetId)?.prefs?.operational;
    }, { timeout: 10000 }).toBe(false);

    await expect(opToggle).toHaveAttribute('aria-checked', 'false');
  });
});
