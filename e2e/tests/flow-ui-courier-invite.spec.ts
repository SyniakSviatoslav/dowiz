import { test, expect } from '@playwright/test';
<<<<<<< Updated upstream
=======
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';
>>>>>>> Stashed changes

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const TS = Date.now();
const INVITE_EMAIL = `e2e-courier-${TS}@test.invalid`;

let authToken: string;
let activeLocationId: string;

test.describe.configure({ mode: 'serial' });

test.describe('UI: Courier Invite — Create via UI, verify API proof', () => {
  test.beforeAll(async ({ request }) => {
    requireStaging(BASE); // mutating spec (creates/revokes invites) — never run against prod
    const r = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(r.status()).toBe(200);
    const b = await r.json();
    authToken = b.access_token;
    activeLocationId = b.activeLocationId;
<<<<<<< Updated upstream
    expect(authToken).toBeTruthy();
    expect(activeLocationId).toMatch(/^[0-9a-f-]{36}$/);
=======
    expectJwt(authToken);
    expectUuid(activeLocationId, 'activeLocationId');
    // mock-auth must mint an OWNER-scoped token. The response body carries no role field —
    // the scope lives in the signed JWT claims, so decode and assert it is owner-only.
    const claims = JSON.parse(Buffer.from(authToken.split('.')[1], 'base64url').toString('utf8'));
    expect(claims.role).toBe('owner');
>>>>>>> Stashed changes
  });

  test('Couriers page loads — shows Add Courier button', async ({ page }) => {
    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/couriers`, { waitUntil: 'load', timeout: 30000 });
    // Assert the specific control is visible (a 500/redirect/spinner must fail this).
    await expect(page.locator('button', { hasText: /add courier/i }).first()).toBeVisible({ timeout: 15000 });
  });

  test('API: POST courier-invite creates an invite with deepLink and code', async ({ request }) => {
    const r = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`,
      {
        headers: { Authorization: `Bearer ${authToken}` },
        data: { role: 'courier', email: INVITE_EMAIL, ttl_hours: 24 },
      }
    );
    expect(r.status()).toBe(200);
    const body = await r.json();
    expectUuid(body.inviteId, 'inviteId');
    expect(body.deepLink).toMatch(/^https?:\/\/.+\/courier-invite\/[0-9a-f-]{36}$/);
    expect(body.code).toMatch(/^[0-9a-f]{16}$/);
    expect(Number.isFinite(Date.parse(body.expiresAt))).toBe(true);
  });

  test('API: GET courier-invites list shows the created invite', async ({ request }) => {
    // Create a fresh invite so we have one to verify
    const createRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`,
      {
        headers: { Authorization: `Bearer ${authToken}` },
        data: { role: 'courier', email: `list-check-${TS}@test.invalid`, ttl_hours: 24 },
      }
    );
    expect(createRes.status()).toBe(200);
    const { inviteId } = await createRes.json();

    const listRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(listRes.status()).toBe(200);
    const { invites } = await listRes.json();
    expect(Array.isArray(invites)).toBe(true);
    const found = invites.find((i: any) => i.id === inviteId);
    expect(found, 'created invite must appear in the list').toBeTruthy();
    expect(found.role).toBe('courier');
  });

  test('API: GET courier-invites without a token is rejected 401 (negative control)', async ({ request }) => {
    // Positive control = the list test above (valid token → 200, non-empty). This is the paired
    // negative: the protected route must reject an unauthenticated caller (verifyAuth, auth.ts:47).
    const r = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`
    );
    expect(r.status()).toBe(401);
    // TODO(needs_staging): cross-tenant IDOR — GET this list with a SECOND real tenant's
    // locationId must return 404 ("Cross-tenant courier is 404", auth.ts:137). Requires a second
    // seeded tenant on staging; a random/nil UUID 404s by absence and proves nothing, so it is
    // intentionally NOT asserted here rather than faked.
  });

  test('UI: clicking Add Courier shows invite form with email input', async ({ page }) => {
    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/couriers`, { waitUntil: 'load', timeout: 30000 });

    const addBtn = page.locator('button', { hasText: /add courier/i }).first();
    await addBtn.waitFor({ state: 'visible', timeout: 15000 });
    await addBtn.click();

    // Form should appear
    const emailInput = page.locator('input[type="email"]').first();
    await expect(emailInput).toBeVisible({ timeout: 5000 });

    const sendBtn = page.locator('button', { hasText: /send invite/i }).first();
    await expect(sendBtn).toBeVisible();
  });

  test('UI: filling form and clicking Send Invite shows invite link and code', async ({ page }) => {
    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/couriers`, { waitUntil: 'load', timeout: 30000 });

    // Open the invite form
    const addBtn = page.locator('button', { hasText: /add courier/i }).first();
    await addBtn.waitFor({ state: 'visible', timeout: 15000 });
    await addBtn.click();

    // Fill email
    const emailInput = page.locator('input[type="email"]').first();
    await expect(emailInput).toBeVisible({ timeout: 5000 });
    await emailInput.fill(`ui-invite-${TS}@test.invalid`);

    // Click Send Invite and wait for the REAL create call to resolve 200 (deterministic —
    // replaces a blind fixed sleep).
    const sendBtn = page.locator('button', { hasText: /send invite/i }).first();
    const [resp] = await Promise.all([
      page.waitForResponse(
        (r) => r.url().includes('/courier-invites') && r.request().method() === 'POST' && r.status() === 200,
        { timeout: 15000 }
      ),
      sendBtn.click(),
    ]);
    expect(resp.ok()).toBe(true);

    // Assert BOTH success affordances INDEPENDENTLY (not an OR-of-truthy): the rendered 16-hex
    // code element AND the invite deep-link anchor must both be visible.
    const codeEl = page.locator('code').filter({ hasText: /^[0-9a-f]{16}$/ });
    await expect(codeEl).toBeVisible();
    await expect(codeEl).toHaveText(/^[0-9a-f]{16}$/);
    const inviteLink = page.locator('a[href*="/courier-invite/"]');
    await expect(inviteLink).toBeVisible();
  });

  test('API: DELETE (revoke) invite removes it from list', async ({ request }) => {
    // Create invite to revoke
    const createRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`,
      {
        headers: { Authorization: `Bearer ${authToken}` },
        data: { role: 'courier', email: `revoke-${TS}@test.invalid`, ttl_hours: 1 },
      }
    );
    expect(createRes.status()).toBe(200);
    const { inviteId } = await createRes.json();

    // Revoke it
    const revokeRes = await request.delete(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites/${inviteId}`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(revokeRes.status()).toBe(200);
    const { success } = await revokeRes.json();
    expect(success).toBe(true);

    // Verify gone from list
    const listRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(listRes.status()).toBe(200);
    const { invites } = await listRes.json();
    const found = invites.find((i: any) => i.id === inviteId);
    expect(found, 'revoked invite must be gone from the list').toBeFalsy();
  });

  test('API: GET /api/owner/couriers returns array (new schema, no 500)', async ({ request }) => {
    const r = await request.get(`${BASE}/api/owner/couriers`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(Array.isArray(body)).toBe(true);
  });
});
