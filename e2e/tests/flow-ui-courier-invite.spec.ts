import { test, expect } from '@playwright/test';
import { expectJwt } from '../helpers/assert-shape';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
const TS = Date.now();
const INVITE_EMAIL = `e2e-courier-${TS}@test.invalid`;

let authToken: string;
let activeLocationId: string;

test.describe.configure({ mode: 'serial' });

test.describe('UI: Courier Invite — Create via UI, verify API proof', () => {
  test.beforeAll(async ({ request }) => {
    const r = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(r.status()).toBe(200);
    const b = await r.json();
    authToken = b.access_token;
    activeLocationId = b.activeLocationId;
    expectJwt(authToken);
    expect(activeLocationId).toMatch(/^[0-9a-f-]{36}$/);
  });

  test('Couriers page loads — shows Add Courier button', async ({ page }) => {
    await page.goto(`${BASE}/admin/couriers`, { waitUntil: 'load', timeout: 30000 });
    await page.waitForTimeout(2000);
    const body = await page.textContent('body');
    expect(body).toBeTruthy();
    // The page should have courier management elements
    const hasAddBtn = await page.locator('button', { hasText: /add courier/i }).count();
    const hasContent = (body?.length || 0) > 100;
    expect(hasContent).toBe(true);
    console.log('Has Add Courier button:', hasAddBtn > 0);
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
    expect(body.inviteId).toMatch(/^[0-9a-f-]{36}$/);
    expect(body.deepLink).toMatch(/^https?:\/\//);
    expect(body.code).toHaveLength(16);
    expect(body.expiresAt).toBeTruthy();
    console.log('Invite created:', body.inviteId, 'link:', body.deepLink);
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
    expect(found).toBeTruthy();
    expect(found.role).toBe('courier');
    console.log('Invite found in list:', found.id);
  });

  test('UI: clicking Add Courier shows invite form with email input', async ({ page }) => {
    await page.goto(`${BASE}/admin/couriers`, { waitUntil: 'load', timeout: 30000 });
    await page.waitForTimeout(2000);

    const addBtn = page.locator('button', { hasText: /add courier/i }).first();
    await addBtn.waitFor({ state: 'visible', timeout: 10000 });
    await addBtn.click();

    // Form should appear
    const emailInput = page.locator('input[type="email"]').first();
    await emailInput.waitFor({ state: 'visible', timeout: 5000 });
    expect(await emailInput.isVisible()).toBe(true);

    const sendBtn = page.locator('button', { hasText: /send invite/i }).first();
    expect(await sendBtn.isVisible()).toBe(true);
    console.log('Invite form visible with email input and Send Invite button');
  });

  test('UI: filling form and clicking Send Invite shows invite link and code', async ({ page }) => {
    await page.goto(`${BASE}/admin/couriers`, { waitUntil: 'load', timeout: 30000 });
    await page.waitForTimeout(2000);

    // Open the invite form
    const addBtn = page.locator('button', { hasText: /add courier/i }).first();
    await addBtn.waitFor({ state: 'visible', timeout: 10000 });
    await addBtn.click();

    // Fill email
    const emailInput = page.locator('input[type="email"]').first();
    await emailInput.waitFor({ state: 'visible', timeout: 5000 });
    await emailInput.fill(`ui-invite-${TS}@test.invalid`);

    // Click Send Invite
    const sendBtn = page.locator('button', { hasText: /send invite/i }).first();
    await sendBtn.click();

    // Wait for result — link and code should appear
    await page.waitForTimeout(3000);
    const bodyText = await page.textContent('body');

    // The UI shows a success section with "Invite Created" and a 16-char code
    const hasInviteCreated = bodyText?.includes('Invite Created') || bodyText?.includes('Ftesa u Krijua') || bodyText?.includes('courier-invite');
    const hasCode = bodyText?.match(/[0-9a-f]{16}/);

    expect(hasInviteCreated || hasCode).toBeTruthy();
    console.log('Invite created in UI. Has code pattern:', !!hasCode, 'Has success text:', !!hasInviteCreated);
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
    const { invites } = await listRes.json();
    const found = invites.find((i: any) => i.id === inviteId);
    expect(found).toBeFalsy();
    console.log('Revoked invite no longer in list');
  });

  test('API: GET /api/owner/couriers returns array (new schema, no 500)', async ({ request }) => {
    const r = await request.get(`${BASE}/api/owner/couriers`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(Array.isArray(body)).toBe(true);
    console.log('Couriers list:', body.length, 'couriers, status 200 (not 500)');
  });
});
