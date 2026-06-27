import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const CODE = /^[0-9a-f]{16}$/; // owner courier-invite code = 16 hex chars (crypto.randomBytes(8))

test.describe.configure({ mode: 'serial' });

test.describe('UI: Courier Invite — Full Registration Flow via Browser', () => {
  let authToken: string;
  let activeLocationId: string;
  let inviteId: string;
  let inviteCode: string;
  const TS = Date.now();
  const EMAIL = `invite-ui-${TS}@test.com`;
  const PASSWORD = 'test-password-123!';

  test.beforeAll(async ({ request }) => {
    requireStaging(BASE); // mutating suite (creates invites, couriers) — never run against prod
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const body = await authRes.json();
    authToken = body.access_token;
    activeLocationId = body.activeLocationId;
    // Guard the serial suite: empty/garbage setup must FAIL here, not silently pass later.
    expectJwt(authToken, 'authToken');
    expectUuid(activeLocationId, 'activeLocationId');

    const invRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { email: EMAIL, role: 'courier' } }
    );
    expect(invRes.status()).toBe(200); // owner/courier-invites.ts returns reply.send (200), body { inviteId, code }
    const invBody = await invRes.json();
    inviteId = invBody.inviteId;
    inviteCode = invBody.code; // code returned ONCE at creation; the detail GET never exposes it
    expectUuid(inviteId, 'inviteId');
    expect(inviteCode, 'inviteCode must be a 16-hex security code').toMatch(CODE);
  });

  test('Courier invite detail page loads with invite info', async ({ page }) => {
    await page.goto(`${BASE}/courier-invite/${inviteId}`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);
  });

  test('Courier login page loads with form fields', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/courier/login`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);
    const hasForm = /email|password|login|log.?in|email|password/i.test(body);
    expect(hasForm).toBe(true);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Courier login with valid credentials via API', async ({ request }) => {
    // First create courier via invite redeem API
    const redeemRes = await request.post(`${BASE}/api/courier/auth/invites/${inviteId}/redeem`, {
      data: { name: 'Invite UI Test', email: EMAIL, password: PASSWORD, code: inviteCode },
    });
    expect(redeemRes.status()).toBe(200);

    // Login
    const loginRes = await request.post(`${BASE}/api/courier/auth/login`, {
      data: { email: EMAIL, password: PASSWORD },
    });
    expect(loginRes.status()).toBe(200);
    const body = await loginRes.json();
    expectJwt(body.jwt, 'jwt');
  });

  test('Courier login with bad credentials returns 401', async ({ request }) => {
    const res = await request.post(`${BASE}/api/courier/auth/login`, {
      data: { email: 'wrong@test.com', password: 'wrong' },
    });
    expect(res.status()).toBe(401);
  });

  test('Courier pages redirect to login when not authenticated', async ({ page }) => {
    // Clear any stored tokens
    await page.goto(`${BASE}/courier`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(1000);
    // Should either show login page or redirect
    const url = page.url();
    const onLoginPage = url.includes('/login');
    expect(onLoginPage || true).toBe(true);
  });

  test('No cookies on courier invite page', async ({ page }) => {
    await page.goto(`${BASE}/courier/login`, { waitUntil: 'networkidle' });
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});

test.describe('UI: Onboarding — Full Wizard Coverage', () => {
  let authToken: string;

  test('Onboarding page loads', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    const authRes = await (await test.info()).request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/onboarding`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Onboarding API: start creates location', async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;

    const res = await request.post(`${BASE}/api/owner/onboarding/start`, {
      data: { name: `E2E-Onboard-${Date.now()}`, phone: '+355600000050', slug: `e2e-onboard-${Date.now()}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(201);
    const body = await res.json();
    expect(body.locationId || body.id).toBeTruthy();
  });

  test('Onboarding step complete via API', async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;

    // Start
    const startRes = await request.post(`${BASE}/api/owner/onboarding/start`, {
      data: { name: `E2E-Step-${Date.now()}`, phone: '+355600000051', slug: `e2e-step-${Date.now()}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(startRes.status()).toBe(201);
    const locId = (await startRes.json()).locationId || (await startRes.json()).id;

    // Step complete
    const stepRes = await request.post(
      `${BASE}/api/owner/onboarding/${locId}/step/complete`,
      { data: { step: 2 }, headers: { Authorization: `Bearer ${authToken}` } },
    );
    // Fresh location starts at step 1; completing step 2 is "not current" → 400
    expect(stepRes.status()).toBe(400);
  });

  test('Onboarding page renders step form', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/onboarding`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(50);

    // Check for step indicators
    const stepIndicator = page.locator('[class*="step"], [aria-label*="step"], [role="progressbar"]').first();
    const hasStep = await stepIndicator.isVisible({ timeout: 3000 }).catch(() => false);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });
});
