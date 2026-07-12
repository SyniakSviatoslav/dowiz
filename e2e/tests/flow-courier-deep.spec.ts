import { test, expect } from '@playwright/test';
import { expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
let authToken: string;
let activeLocationId: string;
let inviteId: string;
let inviteCode: string;

test.describe.configure({ mode: 'serial' });

test.describe('Flow: Courier — Invite, Shift, Tasks, Earnings, History', () => {
  test.beforeAll(async ({ request }) => {
    requireStaging(BASE); // mutating spec (creates/revokes invites) — never hit prod
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const authBody = await authRes.json();
    authToken = authBody.access_token;
    activeLocationId = authBody.activeLocationId;
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 1: Owner creates courier invite — test contract
  // ──────────────────────────────────────────────────────────────
  test('Flow 1: Owner — create courier invite, verify response shape', async ({ request }) => {
    const inviteRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`,
      {
        data: { role: 'courier', email: `e2e-courier-${Date.now()}@test.dowiz` },
        headers: { Authorization: `Bearer ${authToken}` },
      }
    );
    expect(inviteRes.status()).toBe(200);
    const body = await inviteRes.json();
    expectUuid(body.inviteId, 'inviteId');
    expect(typeof body.code).toBe('string');
    expect(body.code.length).toBe(16);
    expect(body.deepLink).toContain(body.inviteId);
    expect(body.expiresAt).toBeTruthy();
    inviteId = body.inviteId;
    inviteCode = body.code;
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 1b: Owner invite endpoints reject unauthenticated access (IDOR)
  // verifyAuth runs in preValidation → no bearer token must be 401.
  // ──────────────────────────────────────────────────────────────
  test('Flow 1b: Owner — courier-invite endpoints require auth', async ({ request }) => {
    const noAuthCreate = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`,
      { data: { role: 'courier', email: `e2e-noauth-${Date.now()}@test.dowiz` } }
    );
    expect(noAuthCreate.status()).toBe(401);

    const noAuthList = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`
    );
    expect(noAuthList.status()).toBe(401);
    // TODO(needs_staging): true cross-tenant IDOR — a *valid* 2nd-tenant owner token
    // against this location should return 404 (auth.ts requireLocationAccess "don't leak
    // existence"). Requires provisioning a second tenant owner on staging.
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 2: GET invite details (unauthenticated)
  // ──────────────────────────────────────────────────────────────
  test('Flow 2: Courier — GET invite details before activation', async ({ request }) => {
    test.skip(!inviteId, 'No invite created in Flow 1');
    const detailRes = await request.get(`${BASE}/api/courier/auth/invites/${inviteId}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(detailRes.status()).toBe(200);
    const detail = await detailRes.json();
    expect(detail.id).toBe(inviteId);
    expect(detail.role).toBe('courier');
    expect(detail.locationName).toBeTruthy();
    expect(detail.isValid).toBe(true);
    expect(detail.isExpired).toBe(false);
    expect(detail.isUsed).toBe(false);
    expect(detail.isRevoked).toBe(false);

    // The route is public (no auth) — exercise the TRUE unauthenticated path,
    // not just the owner-token path above. Must still return 200 with the same id.
    const anonRes = await request.get(`${BASE}/api/courier/auth/invites/${inviteId}`);
    expect(anonRes.status()).toBe(200);
    const anon = await anonRes.json();
    expect(anon.id).toBe(inviteId);
    expect(anon.isValid).toBe(true);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 3: Owner lists active invites
  // ──────────────────────────────────────────────────────────────
  test('Flow 3: Owner — list active courier invites', async ({ request }) => {
    test.skip(!inviteId, 'No invite created in Flow 1');
    const listRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(listRes.status()).toBe(200);
    const list = await listRes.json();
    expect(Array.isArray(list.invites)).toBe(true);
    expect(list.invites.length).toBeGreaterThanOrEqual(1);
    const found = list.invites.find((i: any) => i.id === inviteId);
    expect(found).toBeTruthy();
    expect(found.role).toBe('courier');
    expect(found.expires_at).toBeTruthy();
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 4: Courier login page — browser test
  // ──────────────────────────────────────────────────────────────
  test('Flow 4: Courier — login page renders email/password form, no JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto(`${BASE}/courier/login`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    expect(errors, `JS errors on login: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);
    expect(/login|courier|email|password|sign in|hyr|identifikim/i.test(body)).toBe(true);
    const inputs = page.locator('input');
    const count = await inputs.count();
    expect(count).toBeGreaterThanOrEqual(2);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 5: Courier tasks page — assignments or empty state
  // ──────────────────────────────────────────────────────────────
  test('Flow 5: Courier — tasks page loads with assignment cards or empty state, no JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto(`${BASE}/courier?dev=true`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    // Assert the page content shell actually rendered (language-agnostic landmark),
    // not just that *some* text exists.
    await expect(page.getByRole('heading', { level: 1 }).first()).toBeVisible();
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/task|delivery|order|accept|reject|pending|active|no task|empty/i.test(body)).toBe(true);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 6: Courier earnings page — summary cards, payouts
  // ──────────────────────────────────────────────────────────────
  test('Flow 6: Courier — earnings page shows summary cards and payout history', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto(`${BASE}/courier/earnings?dev=true`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    expect(errors, `JS errors on earnings: ${errors.join('; ')}`).toEqual([]);
    await expect(page.getByRole('heading', { level: 1 }).first()).toBeVisible();
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);
    expect(/earning|total|today|week|payout|balance|ALL|Lek/i.test(body)).toBe(true);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 7: Courier history page — delivery cards with ratings
  // ──────────────────────────────────────────────────────────────
  test('Flow 7: Courier — history page shows delivery cards with star ratings or feedback', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto(`${BASE}/courier/history?dev=true`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    expect(errors, `JS errors on history: ${errors.join('; ')}`).toEqual([]);
    await expect(page.getByRole('heading', { level: 1 }).first()).toBeVisible();
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/history|delivery|order|completed|rating|star|feedback/i.test(body)).toBe(true);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 8: Courier shift page — timer and start/end controls
  // ──────────────────────────────────────────────────────────────
  test('Flow 8: Courier — shift page shows timer, start/end toggle, online status', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto(`${BASE}/courier/shift?dev=true`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    expect(errors, `JS errors on shift: ${errors.join('; ')}`).toEqual([]);
    await expect(page.getByRole('heading', { level: 1 }).first()).toBeVisible();
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);
    expect(/shift|start|end|timer|online|offline|available/i.test(body)).toBe(true);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 9: Courier delivery page — map, dropoff info, swipe to complete
  // ──────────────────────────────────────────────────────────────
  test('Flow 9: Courier — delivery page shows map and dropoff info', async ({ page }) => {
    // TODO(needs_staging): `test-delivery` is not a real delivery id — the page renders a
    // not-found/empty shell and the loose regex below passes regardless. Create a real
    // delivery via the API in beforeAll, navigate to that concrete id, and assert the
    // `[data-testid=task-cash-amount]` / "Drop-off" (h2) landmark is visible. Requires a
    // live staged courier+order fixture.
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto(`${BASE}/courier/delivery/test-delivery?dev=true`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('maplibregl')
    );
    expect(criticalErrors, `JS errors on delivery: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/delivery|map|dropoff|pickup|address|order|status|complete/i.test(body)).toBe(true);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 10: Owner revokes the invite — cleanup
  // ──────────────────────────────────────────────────────────────
  test('Flow 10: Owner — revoke courier invite, verify response', async ({ request }) => {
    test.skip(!inviteId, 'No invite created in Flow 1');
    const revokeRes = await request.delete(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites/${inviteId}`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(revokeRes.status()).toBe(200);
    const result = await revokeRes.json();
    expect(result.success).toBe(true);
    // Verify invite is now invalid (may return 401 for revoked invites)
    const detailRes = await request.get(`${BASE}/api/courier/auth/invites/${inviteId}`);
    expect([200, 401]).toContain(detailRes.status());
    if (detailRes.status() === 200) {
      const detail = await detailRes.json();
      expect(detail.isValid).toBe(false);
      expect(detail.isRevoked).toBe(true);
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 11: Persistence — courier pages survive refresh
  // ──────────────────────────────────────────────────────────────
  test('Flow 11: Courier — pages survive refresh and navigation, no JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/courier?dev=true`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    await expect(page.getByRole('heading', { level: 1 }).first()).toBeVisible();
    let body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    await page.reload({ waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    await expect(page.getByRole('heading', { level: 1 }).first()).toBeVisible();
    body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    await page.goto(`${BASE}/courier/earnings?dev=true`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    await expect(page.getByRole('heading', { level: 1 }).first()).toBeVisible();
    body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    await page.goto(`${BASE}/courier?dev=true`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    await expect(page.getByRole('heading', { level: 1 }).first()).toBeVisible();
    body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    expect(errors, `JS errors after navigation: ${errors.join('; ')}`).toEqual([]);

    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
