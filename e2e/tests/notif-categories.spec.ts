import { test, expect } from '@playwright/test';
<<<<<<< Updated upstream
=======
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';
>>>>>>> Stashed changes

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
    // Mutating spec (UI toggle + prefs writes): fail fast on prod/unknown targets.
    requireStaging(BASE);

    const auth = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(auth.status(), 'mock-auth (needs DEV_AUTH_SECRET)').toBe(200);
    const a = await auth.json();
    token = a.access_token;
    locationId = a.activeLocationId;
    userId = a.userId;
<<<<<<< Updated upstream
    expect(locationId, 'mock-auth must return an active location').toBeTruthy();
=======
    expectJwt(token, 'mock-auth must return an access token');
    expectUuid(userId, 'mock-auth must return a user id');
    expectUuid(locationId, 'mock-auth must return an active location');
>>>>>>> Stashed changes

    // seed-telegram-target lives on the mockAuthRoutes plugin, mounted at /dev (not /api/dev)
    const seed = await request.post(`${BASE}/dev/seed-telegram-target`, { data: { locationId, userId } });
    expect(seed.status(), 'seed-telegram-target').toBe(200);
    targetId = (await seed.json()).targetId;
    expectUuid(targetId, 'seed-telegram-target must return the seeded target id');

    // Idempotent start: reset the primary target's prefs to defaults (operational ON,
    // quality OFF) so the test isn't affected by state a prior run left behind. We just
    // seeded an ACTIVE telegram target, so a primary MUST exist — assert it rather than
    // silently no-op (a missing primary means the seed/fixture is broken, not "nothing to do").
    const list = await request.get(`${BASE}/api/owner/locations/${locationId}/notifications/targets`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    const primary = ((await list.json()).targets as any[]).find((x) => x.channel === 'telegram' && x.status === 'active');
    expect(primary, 'an active telegram target must exist after seeding').toBeTruthy();
    expectUuid(primary.id, 'primary telegram target id');
    const reset = await request.put(`${BASE}/api/owner/locations/${locationId}/notifications/targets/${primary.id}`, {
      headers: { Authorization: `Bearer ${token}` },
      data: { prefs: { operational: true, quality: false } },
    });
    expect(reset.status(), 'reset prefs to defaults').toBe(200);
  });

  test('renders the three categories and persists an operational toggle', async ({ page, request }) => {
    await page.addInitScript((tk: string) => {
      localStorage.setItem('dos_access_token', tk);
      localStorage.setItem('dos_locale', 'en'); // assert against English copy deterministically
    }, token);
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

    // toggle operational OFF → verify the write reached the API. The preference-centre
    // controls the FIRST active telegram target, which may not be the one this test seeded
    // (staging can hold several from prior runs), so assert on that same primary target.
    await opToggle.click();
    await expect.poll(async () => {
      const res = await request.get(`${BASE}/api/owner/locations/${locationId}/notifications/targets`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      const targets = (await res.json()).targets as any[];
      const primary = targets.find((x) => x.channel === 'telegram' && x.status === 'active');
      return primary?.prefs?.operational;
    }, { timeout: 10000 }).toBe(false);

    await expect(opToggle).toHaveAttribute('aria-checked', 'false');

    // Persistence proof: reload so the toggle is re-hydrated from the API (the GET /targets
    // prefs), NOT the optimistic in-memory update — the write must survive a fresh page load.
    await page.reload({ waitUntil: 'networkidle' });
    await expect(card).toBeVisible({ timeout: 15000 });
    await expect(page.getByTestId('notif-cat-operational').getByRole('switch')).toHaveAttribute(
      'aria-checked',
      'false',
    );
  });

  // ── Error matrix (Test Integrity §3/§4): protected route needs negative controls with
  //    EXACT statuses, read from apps/api: verifyAuth→401, validation preserved at 400
  //    (server.ts setErrorHandler), cross-tenant owner→404 (requireLocationAccess, no membership).
  test('PUT prefs without a token is rejected (401)', async ({ request }) => {
    const res = await request.put(
      `${BASE}/api/owner/locations/${locationId}/notifications/targets/${targetId}`,
      { data: { prefs: { operational: false } } },
    );
    expect(res.status(), 'no Authorization header must be 401').toBe(401);
  });

  test('PUT with a non-boolean prefs payload is rejected (400)', async ({ request }) => {
    const res = await request.put(
      `${BASE}/api/owner/locations/${locationId}/notifications/targets/${targetId}`,
      {
        headers: { Authorization: `Bearer ${token}` },
        // prefs is z.record(z.boolean()); a string value fails schema validation → 400.
        data: { prefs: { operational: 'yes' } },
      },
    );
    expect(res.status(), 'invalid prefs body must be 400 VALIDATION_FAILED').toBe(400);
  });

  // CRITICAL — cross-tenant isolation against a REAL second tenant (vis-owner, seeded by
  // /dev/seed-visual-state), not a nil-UUID. The authenticated dev owner has NO membership
  // on that location, so requireLocationAccess must 404 (it 404s rather than 403 to avoid
  // leaking existence — auth.ts:153). Proves the prefs write is tenant-scoped.
  // TODO(needs-staging): requires a live staging run with /dev/seed-visual-state available.
  test('owner cannot write prefs on a foreign tenant location (404)', async ({ request }) => {
    const vis = await request.post(`${BASE}/dev/seed-visual-state`, { data: {} });
    expect(vis.status(), 'seed-visual-state (foreign tenant)').toBe(200);
    const foreign = await vis.json();
    const foreignLocationId = foreign.closed.locationId as string;
    expectUuid(foreignLocationId, 'foreign tenant locationId');

    const foreignSeed = await request.post(`${BASE}/dev/seed-telegram-target`, {
      data: { locationId: foreignLocationId },
    });
    expect(foreignSeed.status(), 'seed foreign telegram target').toBe(200);
    const foreignTargetId = (await foreignSeed.json()).targetId as string;
    expectUuid(foreignTargetId, 'foreign tenant targetId');

    const res = await request.put(
      `${BASE}/api/owner/locations/${foreignLocationId}/notifications/targets/${foreignTargetId}`,
      { headers: { Authorization: `Bearer ${token}` }, data: { prefs: { operational: false } } },
    );
    expect(res.status(), 'cross-tenant prefs write must be denied (404)').toBe(404);
  });
});
